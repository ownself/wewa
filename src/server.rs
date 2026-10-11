//! Loopback HTTP service for static projects and isolated video wallpapers.
//! Files are streamed, never collected into a file-sized memory buffer.

use crate::video::{self, VideoFit};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tiny_http::{Header, Method, Response, ResponseBox, Server, StatusCode};

const WORKERS: usize = 4;

#[derive(Clone)]
enum Content {
    Directory(PathBuf),
    Video { path: PathBuf, html: String },
}

pub struct LocalServer {
    content: Content,
    port: u16,
    workers: usize,
    server: Option<Arc<Server>>,
    shutdown: Arc<AtomicBool>,
}

impl LocalServer {
    pub fn new(root_dir: PathBuf, port: u16) -> Self {
        Self::with_content(Content::Directory(root_dir), port)
    }

    /// Only these two routes are exposed; sibling files are not published.
    pub fn for_video(path: &Path, port: u16, fit: VideoFit) -> io::Result<Self> {
        let path = path.canonicalize()?;
        if !path.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Not a video file",
            ));
        }
        Ok(Self::with_content(
            Content::Video {
                path,
                html: video::build_video_html(fit),
            },
            port,
        ))
    }

    fn with_content(content: Content, port: u16) -> Self {
        Self {
            content,
            port,
            workers: WORKERS,
            server: None,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Leave room for concurrent media transfers and page/seek requests on
    /// multiple monitors, while keeping application worker creation bounded.
    pub fn for_displays(mut self, count: usize) -> Self {
        self.workers = count.saturating_mul(2).saturating_add(2).clamp(WORKERS, 64);
        self
    }

    pub fn is_port_available(port: u16) -> bool {
        TcpListener::bind(("127.0.0.1", port)).is_ok()
    }

    pub fn start(&mut self) -> io::Result<()> {
        if self.server.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Server already started",
            ));
        }
        let addr = format!("127.0.0.1:{}", self.port);
        let server = Arc::new(Server::http(&addr).map_err(|e| {
            io::Error::new(
                io::ErrorKind::AddrInUse,
                format!("Failed to bind to {}: {}", addr, e),
            )
        })?);
        self.port = server.server_addr().to_ip().unwrap().port();
        self.shutdown.store(false, Ordering::Relaxed);
        let content = Arc::new(self.content.clone());

        // A fixed pool avoids one OS thread per range request. Idle workers
        // block without periodic polling and are explicitly woken on shutdown.
        for _ in 0..self.workers {
            let server = Arc::clone(&server);
            let content = Arc::clone(&content);
            let shutdown = Arc::clone(&self.shutdown);
            thread::spawn(move || {
                while !shutdown.load(Ordering::Relaxed) {
                    let request = match server.recv() {
                        Ok(request) => request,
                        Err(_) => break,
                    };
                    if shutdown.load(Ordering::Relaxed) {
                        break;
                    }
                    let response = handle_request(&content, &request);
                    let _ = request.respond(response);
                }
            });
        }
        self.server = Some(server);
        Ok(())
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn file_url(&self, filename: &str) -> String {
        format!("{}/{}", self.url(), encode_path(filename))
    }

    pub fn video_url(&self) -> String {
        format!("{}{}", self.url(), video::PLAYER_PATH)
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(server) = &self.server {
            for _ in 0..self.workers {
                server.unblock();
            }
        }
    }
}

impl Drop for LocalServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name, value).unwrap()
}

fn text_response(code: u16, message: &str) -> ResponseBox {
    Response::from_string(message)
        .with_status_code(code)
        .with_header(header("Content-Type", "text/plain; charset=utf-8"))
        .with_header(header("Cache-Control", "no-store"))
        .boxed()
}

fn handle_request(content: &Content, request: &tiny_http::Request) -> ResponseBox {
    if !matches!(request.method(), Method::Get | Method::Head) {
        return text_response(405, "Method not allowed").with_header(header("Allow", "GET, HEAD"));
    }
    let path = request.url().split(['?', '#']).next().unwrap_or("");
    // Range only applies to GET. Without validators we conservatively ignore
    // If-Range, and duplicate Range headers, rather than send an incorrect 206.
    let mut ranges = request.headers().iter().filter(|h| h.field.equiv("Range"));
    let range = ranges.next().map(|h| h.value.as_str());
    let range = if request.method() == &Method::Head
        || ranges.next().is_some()
        || request.headers().iter().any(|h| h.field.equiv("If-Range"))
    {
        None
    } else {
        range
    };

    match content {
        Content::Video {
            path: video_path,
            html,
        } => match path {
            video::PLAYER_PATH => Response::from_string(html)
                .with_header(header("Content-Type", "text/html; charset=utf-8"))
                .with_header(header("Cache-Control", "no-store"))
                .boxed(),
            video::MEDIA_PATH => serve_file(video_path, range),
            _ => text_response(404, "Not found"),
        },
        Content::Directory(root) => {
            let path = path.strip_prefix('/').unwrap_or(path);
            let path = if path.is_empty() { "index.html" } else { path };
            match validate_and_resolve_path(root, path) {
                Ok(path) => serve_file(&path, range),
                Err(_) => text_response(404, "File not found or path not allowed"),
            }
        }
    }
}

fn validate_and_resolve_path(root: &Path, requested_path: &str) -> io::Result<PathBuf> {
    let decoded = urlencoding_decode(requested_path)?;
    // Treat both separator styles as path separators for validation, including
    // on Unix; reject Windows drive/alternate-stream syntax as well.
    if decoded.starts_with(['/', '\\'])
        || decoded.contains(':')
        || decoded.split(['/', '\\']).any(|part| part == "..")
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Path not allowed",
        ));
    }
    let root = root.canonicalize()?;
    let path = root.join(decoded).canonicalize()?;
    if !path.starts_with(&root) || !path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Path not allowed",
        ));
    }
    Ok(path)
}

fn urlencoding_decode(s: &str) -> io::Result<String> {
    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let decoded = bytes.get(i + 1..i + 3).and_then(|pair| {
                let high = (pair[0] as char).to_digit(16)?;
                let low = (pair[1] as char).to_digit(16)?;
                Some((high * 16 + low) as u8)
            });
            match decoded {
                Some(byte) => {
                    result.push(byte);
                    i += 3;
                }
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Invalid URL escape",
                    ))
                }
            }
        } else {
            // '+' is literal in URL paths, unlike form/query encoding.
            result.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(result)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid UTF-8 path"))
}

fn encode_path(path: &str) -> String {
    let mut encoded = String::new();
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            encoded.push(byte as char);
        } else {
            use std::fmt::Write;
            write!(encoded, "%{:02X}", byte).unwrap();
        }
    }
    encoded
}

#[derive(Debug, PartialEq, Eq)]
enum ByteRange {
    Full,
    Partial { start: u64, end: u64 },
    Unsatisfiable,
}

/// Unsupported or malformed ranges are ignored (200), as permitted by HTTP.
/// Only a syntactically valid single range with no overlap gets a 416.
fn parse_range(value: Option<&str>, length: u64) -> ByteRange {
    let Some(value) = value else {
        return ByteRange::Full;
    };
    let Some(value) = value.strip_prefix("bytes=") else {
        return ByteRange::Full;
    };
    let Some((start, end)) = value.split_once('-') else {
        return ByteRange::Full;
    };
    let number = |s: &str| {
        if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
            s.parse::<u64>().ok()
        } else {
            None
        }
    };
    if start.is_empty() {
        let Some(suffix) = number(end) else {
            return ByteRange::Full;
        };
        if suffix == 0 || length == 0 {
            return ByteRange::Unsatisfiable;
        }
        return ByteRange::Partial {
            start: length.saturating_sub(suffix),
            end: length - 1,
        };
    }
    let Some(start) = number(start) else {
        return ByteRange::Full;
    };
    let end = if end.is_empty() {
        length.saturating_sub(1)
    } else {
        let Some(end) = number(end) else {
            return ByteRange::Full;
        };
        if end < start {
            return ByteRange::Full;
        }
        end
    };
    if start >= length {
        ByteRange::Unsatisfiable
    } else {
        ByteRange::Partial {
            start,
            end: end.min(length - 1),
        }
    }
}

fn serve_file(path: &Path, range: Option<&str>) -> ResponseBox {
    match stream_file(path, range) {
        Ok(response) => response,
        Err(_) => text_response(500, "Error reading file"),
    }
}

fn stream_file(path: &Path, range: Option<&str>) -> io::Result<ResponseBox> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    let (status, start, count, content_range) = match parse_range(range, length) {
        ByteRange::Full => (200, 0, length, None),
        ByteRange::Partial { start, end } => (
            206,
            start,
            end - start + 1,
            Some(format!("bytes {}-{}/{}", start, end, length)),
        ),
        ByteRange::Unsatisfiable => {
            return Ok(text_response(416, "Requested range not satisfiable")
                .with_header(header("Accept-Ranges", "bytes"))
                .with_header(header("Content-Range", &format!("bytes */{}", length))))
        }
    };
    file.seek(SeekFrom::Start(start))?;
    // On 32-bit platforms an exceptionally large full response may use chunked
    // transfer; ordinary files and partial responses have an explicit length.
    let mut response = Response::new(
        StatusCode(status),
        vec![
            header("Content-Type", get_content_type(path)),
            header("Accept-Ranges", "bytes"),
            header("Cache-Control", "no-cache, no-store, must-revalidate"),
        ],
        file.take(count),
        usize::try_from(count).ok(),
        None,
    )
    .with_chunked_threshold(usize::MAX);
    if let Some(content_range) = content_range {
        response.add_header(header("Content-Range", &content_range));
    }
    Ok(response.boxed())
}

fn get_content_type(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "ogv" => "video/ogg",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::net::TcpStream;
    use std::sync::atomic::AtomicU64;
    use std::time::Duration;

    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "wewa-http-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn request(port: u16, method: &str, path: &str, extra: &str) -> (String, Vec<u8>) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(
            stream,
            "{} {} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n{}\r\n",
            method, path, extra
        )
        .unwrap();
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).unwrap();
        let split = bytes.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        (
            String::from_utf8(bytes[..split].to_vec())
                .unwrap()
                .to_lowercase(),
            bytes[split + 4..].to_vec(),
        )
    }

    #[test]
    fn parses_ranges_and_clamps_without_underflow() {
        assert_eq!(parse_range(None, 10), ByteRange::Full);
        for (value, start, end) in [
            ("bytes=2-4", 2, 4),
            ("bytes=7-", 7, 9),
            ("bytes=-3", 7, 9),
            ("bytes=-100", 0, 9),
            ("bytes=0-99", 0, 9),
        ] {
            assert_eq!(
                parse_range(Some(value), 10),
                ByteRange::Partial { start, end }
            );
        }
        for value in ["bytes=10-", "bytes=20-30", "bytes=-0"] {
            assert_eq!(parse_range(Some(value), 10), ByteRange::Unsatisfiable);
        }
        for value in ["bytes=0-", "bytes=-1", "bytes=0-0"] {
            assert_eq!(parse_range(Some(value), 0), ByteRange::Unsatisfiable);
        }
        for value in [
            "bytes=",
            "bytes=-",
            "bytes=7-2",
            "bytes=0-1,4-5",
            "items=0-1",
            "bytes=+1-2",
            "bytes=0-18446744073709551616",
            "bytes=你好-2",
        ] {
            assert_eq!(parse_range(Some(value), 10), ByteRange::Full);
        }
    }

    #[test]
    fn utf8_paths_roundtrip_and_plus_is_literal() {
        for value in [
            "hello world",
            "中文💗.mp4",
            "a+b#%&?.html",
            "nested/file..html",
        ] {
            assert_eq!(urlencoding_decode(&encode_path(value)).unwrap(), value);
        }
        assert_eq!(urlencoding_decode("a+b").unwrap(), "a+b");
        for value in ["%", "%0", "%GG", "%FF", "%你"] {
            assert!(urlencoding_decode(value).is_err());
        }
    }

    #[test]
    fn static_http_supports_get_head_ranges_and_safe_paths() {
        let dir = TestDir::new();
        fs::write(dir.0.join("index.html"), b"0123456789").unwrap();
        fs::write(dir.0.join("电影 +#%&.mp4"), b"video").unwrap();
        fs::write(dir.0.join("empty.mp4"), b"").unwrap();
        let mut server = LocalServer::new(dir.0.clone(), 0);
        server.start().unwrap();
        let port = server.port();
        let (heads, body) = request(port, "GET", "/?ignored=1", "");
        assert!(heads.starts_with("http/1.1 200"));
        assert!(heads.contains("content-length: 10"));
        assert!(heads.contains("accept-ranges: bytes"));
        assert!(heads.contains("content-type: text/html; charset=utf-8"));
        assert_eq!(body, b"0123456789");
        for (range, expected, content_range) in [
            ("bytes=2-4", &b"234"[..], "bytes 2-4/10"),
            ("bytes=7-", &b"789"[..], "bytes 7-9/10"),
            ("bytes=-3", &b"789"[..], "bytes 7-9/10"),
            ("bytes=0-99", &b"0123456789"[..], "bytes 0-9/10"),
        ] {
            let (heads, body) =
                request(port, "GET", "/index.html", &format!("Range: {}\r\n", range));
            assert!(heads.starts_with("http/1.1 206"));
            assert!(heads.contains(&format!("content-range: {}", content_range)));
            assert!(heads.contains(&format!("content-length: {}", expected.len())));
            assert_eq!(body, expected);
        }
        let (heads, body) = request(port, "HEAD", "/index.html", "Range: bytes=2-4\r\n");
        assert!(heads.starts_with("http/1.1 200"));
        assert!(heads.contains("content-length: 10"));
        assert!(body.is_empty());
        let (heads, _) = request(port, "GET", "/index.html", "Range: bytes=10-\r\n");
        assert!(heads.starts_with("http/1.1 416"));
        assert!(heads.contains("content-range: bytes */10"));
        for extra in [
            "Range: bytes=bad\r\n",
            "Range: bytes=0-1,5-6\r\n",
            "Range: bytes=0-1\r\nIf-Range: \"unknown\"\r\n",
            "Range: bytes=0-1\r\nRange: bytes=5-6\r\n",
        ] {
            let (heads, body) = request(port, "GET", "/index.html", extra);
            assert!(heads.starts_with("http/1.1 200"));
            assert_eq!(body, b"0123456789");
        }
        let file_url = server.file_url("电影 +#%&.mp4");
        let url_path = file_url.strip_prefix(&server.url()).unwrap();
        let (heads, body) = request(port, "GET", &format!("{}?x=1", url_path), "");
        assert!(heads.contains("content-type: video/mp4"));
        assert_eq!(body, b"video");
        for path in [
            "/../index.html",
            "/%2e%2e/index.html",
            "/%2e%2e%5cindex.html",
            "//index.html",
            "/C%3A/index.html",
            "/index.html%3Astream",
            "/%FF",
            "/missing.html",
        ] {
            assert!(request(port, "GET", path, "").0.starts_with("http/1.1 404"));
        }
        assert!(
            request(port, "POST", "/index.html", "Content-Length: 0\r\n")
                .0
                .starts_with("http/1.1 405")
        );
        let (heads, body) = request(port, "GET", "/empty.mp4", "");
        assert!(heads.starts_with("http/1.1 200"));
        assert!(heads.contains("content-length: 0"));
        assert!(body.is_empty());
        assert!(request(port, "GET", "/empty.mp4", "Range: bytes=0-\r\n")
            .0
            .contains("content-range: bytes */0"));
        for _ in 0..3 {
            assert_eq!(
                request(port, "GET", "/index.html", "Range: bytes=0-1\r\n").1,
                b"01"
            );
        }
    }

    #[test]
    fn video_routes_are_isolated_and_concurrent() {
        let dir = TestDir::new();
        let path = dir.0.join("电影 +#%.MP4");
        fs::write(&path, b"0123456789").unwrap();
        assert!(video::validate_input(path.to_str().unwrap(), Some(VideoFit::Cover)).unwrap());
        assert!(LocalServer::for_video(&dir.0, 0, VideoFit::Contain).is_err());
        assert!(LocalServer::for_video(&dir.0.join("missing.mp4"), 0, VideoFit::Contain).is_err());
        fs::write(dir.0.join("secret.txt"), b"secret").unwrap();
        let mut server = LocalServer::for_video(&path, 0, VideoFit::Cover).unwrap();
        server.start().unwrap();
        let (heads, body) = request(server.port(), "GET", video::PLAYER_PATH, "");
        assert!(heads.contains("content-type: text/html"));
        assert!(String::from_utf8(body)
            .unwrap()
            .contains("object-fit: cover"));
        let (heads, body) = request(server.port(), "HEAD", video::MEDIA_PATH, "");
        assert!(heads.contains("content-length: 10"));
        assert!(body.is_empty());
        for path in ["/", "/secret.txt", "/%2e%2e/secret.txt"] {
            assert!(request(server.port(), "GET", path, "")
                .0
                .starts_with("http/1.1 404"));
        }
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let port = server.port();
                thread::spawn(move || {
                    request(port, "GET", video::MEDIA_PATH, "Range: bytes=3-5\r\n")
                })
            })
            .collect();
        for handle in handles {
            let (heads, body) = handle.join().unwrap();
            assert!(heads.starts_with("http/1.1 206"));
            assert_eq!(body, b"345");
        }
    }

    #[test]
    fn streams_a_small_range_from_a_large_sparse_file() {
        let dir = TestDir::new();
        let path = dir.0.join("large.mp4");
        let file = File::create(&path).unwrap();
        file.set_len(1024 * 1024 * 1024).unwrap();
        drop(file);
        let response = stream_file(&path, Some("bytes=-8")).unwrap();
        assert_eq!(response.status_code(), StatusCode(206));
        let mut body = Vec::new();
        response.into_reader().read_to_end(&mut body).unwrap();
        assert_eq!(body, vec![0; 8]);
    }

    #[test]
    fn idle_server_releases_port_after_drop() {
        let dir = TestDir::new();
        let mut server = LocalServer::new(dir.0.clone(), 0);
        server.start().unwrap();
        let port = server.port();
        assert!(!LocalServer::is_port_available(port));
        drop(server);
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while !LocalServer::is_port_available(port) {
            assert!(
                std::time::Instant::now() < deadline,
                "server did not release port"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_may_not_escape_the_static_root() {
        let dir = TestDir::new();
        let outside = TestDir::new();
        fs::write(outside.0.join("secret"), b"secret").unwrap();
        std::os::unix::fs::symlink(outside.0.join("secret"), dir.0.join("link")).unwrap();
        assert!(validate_and_resolve_path(&dir.0, "link").is_err());
    }

    #[test]
    fn worker_count_is_bounded_and_scales_with_displays() {
        let dir = TestDir::new();
        assert_eq!(
            LocalServer::new(dir.0.clone(), 0).for_displays(1).workers,
            4
        );
        assert_eq!(
            LocalServer::new(dir.0.clone(), 0).for_displays(4).workers,
            10
        );
        assert_eq!(
            LocalServer::new(dir.0.clone(), 0)
                .for_displays(usize::MAX)
                .workers,
            64
        );
    }

    #[test]
    fn video_mime_types_are_correct() {
        for (ext, mime) in [
            ("MP4", "video/mp4"),
            ("m4v", "video/mp4"),
            ("webm", "video/webm"),
            ("mov", "video/quicktime"),
            ("ogv", "video/ogg"),
            ("bin", "application/octet-stream"),
        ] {
            assert_eq!(get_content_type(Path::new(&format!("file.{}", ext))), mime);
        }
    }
}
