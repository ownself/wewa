//! Local video wallpapers rendered by a minimal HTML media element.

use clap::ValueEnum;
use serde::Deserialize;
use std::path::Path;

pub const PLAYER_PATH: &str = "/__wewa_video.html";
pub const MEDIA_PATH: &str = "/__wewa_media";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum VideoFit {
    #[default]
    Contain,
    Cover,
}

impl VideoFit {
    fn css(self) -> &'static str {
        match self {
            Self::Contain => "contain",
            Self::Cover => "cover",
        }
    }
}

pub fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            ["mp4", "m4v", "webm", "mov", "ogv"]
                .iter()
                .any(|candidate| ext.eq_ignore_ascii_case(candidate))
        })
        .unwrap_or(false)
}

/// Validate before replacing an existing wallpaper or creating any server.
/// Remote URLs deliberately remain ordinary web content in this version.
pub fn validate_input(input: &str, fit: Option<VideoFit>) -> Result<bool, String> {
    let is_url = input.starts_with("http://")
        || input.starts_with("https://")
        || input.starts_with("file://");
    let is_video = !is_url && is_video_file(Path::new(input));
    if fit.is_some() && !is_video {
        return Err("--video-fit is only supported for local video files".to_string());
    }
    if is_video && !Path::new(input).is_file() {
        return Err(format!(
            "Video file not found or not a regular file: {}",
            input
        ));
    }
    Ok(is_video)
}

/// No filenames, arbitrary URLs or shader text are inserted into this page.
/// The browser owns decoding and frame scheduling; no JS frame loop is needed.
pub fn build_video_html(fit: VideoFit) -> String {
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><title>wewa video</title>
<style>
html, body {{ margin: 0; width: 100%; height: 100%; overflow: hidden; background: #000; }}
video {{ display: block; width: 100%; height: 100%; object-fit: {fit}; object-position: center; pointer-events: none; }}
#error {{ display: none; position: fixed; left: 1rem; bottom: 1rem; max-width: 80%; padding: 1rem; color: white; background: #600; font: 16px sans-serif; }}
</style></head><body>
<video id="video" autoplay muted loop playsinline preload="auto" disablepictureinpicture></video>
<div id="error" role="alert"></div>
<script>
(() => {{
  const video = document.getElementById('video');
  const errorBox = document.getElementById('error');
  let failed = false;
  const report = (event, detail) => {{
    if (window.ipc) {{
      window.ipc.postMessage(JSON.stringify({{ source: 'wewa-video', event, detail }}));
    }}
  }};
  const fail = (detail) => {{
    if (failed) return;
    failed = true;
    errorBox.textContent = 'Video playback failed: ' + detail + ' Try an H.264 MP4 file and check system media codecs.';
    errorBox.style.display = 'block';
    report('error', detail);
  }};
  video.addEventListener('error', () => {{
    const names = {{ 1: 'playback aborted', 2: 'network/file read error', 3: 'decode error', 4: 'unsupported source or codec' }};
    const code = video.error ? video.error.code : 0;
    fail(names[code] || 'unknown media error');
  }});
  video.addEventListener('playing', () => {{
    report('playing', video.videoWidth + 'x' + video.videoHeight + ', fit={fit}, muted loop');
  }}, {{ once: true }});
  video.muted = true;
  video.src = '{media_path}';
  const playback = video.play();
  if (playback && playback.catch) {{
    playback.catch(error => fail('play() rejected: ' + error.name));
  }}
}})();
</script></body></html>"#,
        fit = fit.css(),
        media_path = MEDIA_PATH,
    )
}

#[derive(Deserialize)]
struct MediaEvent {
    source: String,
    event: String,
    detail: String,
}

/// Only bounded, recognized messages from our video page become log output.
pub fn report_media_event(message: &str, display: u32, verbose: bool) {
    if message.len() > 4096 {
        return;
    }
    let Ok(event) = serde_json::from_str::<MediaEvent>(message) else {
        return;
    };
    if event.source != "wewa-video" {
        return;
    }
    let detail: String = event
        .detail
        .chars()
        .filter(|c| !c.is_control())
        .take(512)
        .collect();
    match event.event.as_str() {
        "error" => eprintln!(
            "[ERROR] Video on display {}: {}. Try H.264 MP4 and check system media codecs.",
            display, detail
        ),
        "playing" if verbose => println!("[INFO] Video playing on display {}: {}", display, detail),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_video_extensions_without_claiming_codec_support() {
        for name in [
            "movie.mp4",
            "电影.MP4",
            "movie.m4v",
            "movie.webm",
            "movie.mov",
            "movie.ogv",
        ] {
            assert!(is_video_file(Path::new(name)));
        }
        for name in [
            "index.html",
            "movie.mp4.html",
            "demo.shader",
            "movie",
            "movie.mkv",
        ] {
            assert!(!is_video_file(Path::new(name)));
        }
    }

    #[test]
    fn page_is_muted_looping_and_uses_fixed_media_route() {
        let html = build_video_html(VideoFit::default());
        assert!(html.contains("autoplay muted loop playsinline"));
        assert!(html.contains("object-fit: contain"));
        assert!(html.contains("background: #000"));
        assert!(html.contains(MEDIA_PATH));
        assert!(html.contains("video.muted = true"));
        assert!(html.contains("video.play()"));
        assert!(html.contains("playback.catch"));
        assert!(html.contains("video.addEventListener('error'"));
        assert!(html.contains("window.ipc.postMessage"));
        assert!(!html.contains("requestAnimationFrame"));
        assert!(!html.contains("setInterval"));
        assert!(!html.contains(" controls"));
        assert!(!html.contains("<canvas"));
        assert!(build_video_html(VideoFit::Cover).contains("object-fit: cover"));
    }

    #[test]
    fn looping_uses_native_media_without_manual_restarts() {
        let html = build_video_html(VideoFit::Contain);
        assert!(html.contains("autoplay muted loop playsinline"));
        assert!(!html.contains("video.loop = false"));
        assert!(!html.contains("video.addEventListener('ended'"));
        assert!(!html.contains("video.currentTime ="));
        assert!(!html.contains("report('loop'"));
        assert!(!html.contains("setTimeout"));
        assert!(!html.contains("setInterval"));
    }

    #[test]
    fn explicit_fit_is_rejected_for_non_local_video() {
        for input in [
            "index.html",
            "demo.shader",
            "https://example.com/video.mp4",
            "file:///video.mp4",
        ] {
            assert!(validate_input(input, Some(VideoFit::Cover)).is_err());
            assert!(!validate_input(input, None).unwrap());
        }
        assert!(validate_input("__missing_video__.mp4", None).is_err());
    }
}
