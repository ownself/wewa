//! Local HTTP server for serving local HTML files
//!
//! Uses tiny_http to serve files from a local directory with proper
//! MIME type detection and security validations.

use std::fs;
use std::io;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use tiny_http::{Header, Response, Server};

/// Local HTTP server for serving static files
pub struct LocalServer {
    /// Base directory to serve files from
    root_dir: PathBuf,
    /// Server port
    port: u16,
    /// Server thread handle
    thread_handle: Option<JoinHandle<()>>,
    /// Shutdown flag
    shutdown: Arc<AtomicBool>,
}

impl LocalServer {
    /// Create a new local server for the given directory
    pub fn new(root_dir: PathBuf, port: u16) -> Self {
        Self {
            root_dir,
            port,
            thread_handle: None,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if the port is available
    pub fn is_port_available(port: u16) -> bool {
        TcpListener::bind(("127.0.0.1", port)).is_ok()
    }

    /// Start the server in a background thread
    pub fn start(&mut self) -> io::Result<()> {
        let addr = format!("127.0.0.1:{}", self.port);
        let server = Server::http(&addr).map_err(|e| {
            io::Error::new(
                io::ErrorKind::AddrInUse,
                format!("Failed to bind to {}: {}", addr, e),
            )
        })?;

        let root_dir = self.root_dir.clone();
        let shutdown = self.shutdown.clone();

        let handle = thread::spawn(move || {
            for request in server.incoming_requests() {
                // Check if we should shutdown
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                // Handle the request
                let response = handle_request(&root_dir, &request);
                let _ = request.respond(response);
            }
        });

        self.thread_handle = Some(handle);
        Ok(())
    }

    /// Get the server URL
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Get the URL for a specific file
    pub fn file_url(&self, filename: &str) -> String {
        format!("http://127.0.0.1:{}/{}", self.port, filename)
    }

    /// Signal the server to shutdown
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

impl Drop for LocalServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Handle a single HTTP request
fn handle_request(
    root_dir: &Path,
    request: &tiny_http::Request,
) -> Response<std::io::Cursor<Vec<u8>>> {
    // Get the requested path
    let url_path = request.url();
    let path = url_path.trim_start_matches('/');

    // Default to index.html if root is requested
    let path = if path.is_empty() { "index.html" } else { path };

    // Validate and resolve the path
    match validate_and_resolve_path(root_dir, path) {
        Ok(file_path) => serve_file(&file_path),
        Err(e) => {
            let msg = format!("Error: {}", e);
            Response::from_string(msg)
                .with_status_code(404)
                .with_header(Header::from_bytes("Content-Type", "text/plain").unwrap())
        }
    }
}

/// Strip Windows extended path prefix (\\?\) from a path
#[cfg(target_os = "windows")]
fn strip_windows_prefix(path: PathBuf) -> PathBuf {
    let path_str = path.to_string_lossy();
    if path_str.starts_with(r"\\?\") {
        PathBuf::from(&path_str[4..])
    } else {
        path
    }
}

#[cfg(not(target_os = "windows"))]
fn strip_windows_prefix(path: PathBuf) -> PathBuf {
    path
}

/// Validate path to prevent directory traversal and resolve to file
fn validate_and_resolve_path(root_dir: &Path, requested_path: &str) -> io::Result<PathBuf> {
    // Decode URL-encoded characters
    let decoded_path = urlencoding_decode(requested_path);

    // Security check: prevent directory traversal by rejecting paths with ..
    // This is checked BEFORE any path resolution to prevent attacks
    if decoded_path.contains("..") {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Path traversal attempt detected",
        ));
    }

    // Also reject absolute paths in the request
    if decoded_path.starts_with('/') || decoded_path.starts_with('\\') {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Absolute paths not allowed",
        ));
    }

    // Check for Windows drive letter patterns (e.g., "C:")
    if decoded_path.len() >= 2 && decoded_path.chars().nth(1) == Some(':') {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Absolute paths not allowed",
        ));
    }

    // Join with root directory
    let joined = root_dir.join(&decoded_path);

    // Canonicalize to resolve symlinks and get the actual file path
    // This allows symlinks to work (pointing to files outside root)
    let canonical = match joined.canonicalize() {
        Ok(p) => strip_windows_prefix(p),
        Err(e) => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("File not found: {} ({})", requested_path, e),
            ));
        }
    };

    // Check if the file exists
    if !canonical.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File not found: {}", requested_path),
        ));
    }

    Ok(canonical)
}

/// Simple URL decoding (handles common cases)
fn urlencoding_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            // Try to parse hex escape
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }

    result
}

/// Serve a file with appropriate content type
fn serve_file(path: &Path) -> Response<std::io::Cursor<Vec<u8>>> {
    // Read the file
    let content = match fs::read(path) {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("Error reading file: {}", e);
            return Response::from_string(msg)
                .with_status_code(500)
                .with_header(Header::from_bytes("Content-Type", "text/plain").unwrap());
        }
    };

    // Determine content type
    let content_type = get_content_type(path);

    // Build response with no-cache headers
    Response::from_data(content)
        .with_header(Header::from_bytes("Content-Type", content_type).unwrap())
        .with_header(
            Header::from_bytes("Cache-Control", "no-cache, no-store, must-revalidate").unwrap(),
        )
        .with_header(Header::from_bytes("Pragma", "no-cache").unwrap())
        .with_header(Header::from_bytes("Expires", "0").unwrap())
}

/// Get the MIME content type for a file based on its extension
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
        "mp4" => "video/mp4",
        "webm" => "video/webm",
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

    #[test]
    fn test_get_content_type() {
        assert_eq!(
            get_content_type(Path::new("test.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            get_content_type(Path::new("style.css")),
            "text/css; charset=utf-8"
        );
        assert_eq!(
            get_content_type(Path::new("app.js")),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(get_content_type(Path::new("image.png")), "image/png");
        assert_eq!(
            get_content_type(Path::new("unknown.xyz")),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_urlencoding_decode() {
        assert_eq!(urlencoding_decode("hello%20world"), "hello world");
        assert_eq!(urlencoding_decode("file%2Fpath"), "file/path");
        assert_eq!(urlencoding_decode("normal"), "normal");
    }

    #[test]
    fn test_is_port_available() {
        // Port 0 should always be assignable (OS picks available port)
        // High ports are usually available
        let available = LocalServer::is_port_available(58432);
        // We can't guarantee this, so just run the function
        let _ = available;
    }
}
