//! macOS platform implementation.
//!
//! Uses NSWindow with desktop-level window ordering and WKWebView (via wry)
//! to render web content as wallpaper.

pub mod display;
pub mod wallpaper;

use crate::platform::PlatformResult;

pub fn init_platform() -> PlatformResult<()> {
    // macOS handles DPI/Retina scaling automatically — no manual setup needed.
    // Activation policy (hiding Dock icon) is set later when creating the event loop.
    Ok(())
}

pub fn ensure_runtime_available() -> PlatformResult<()> {
    // WKWebView is a system framework on macOS (available since 10.10).
    // No runtime check necessary.
    Ok(())
}
