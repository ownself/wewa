//! Windows platform implementation
//!
//! Provides Windows-specific functionality for display enumeration,
//! wallpaper window management, and DPI awareness.

pub mod display;
pub mod wallpaper;

use crate::platform::{PlatformError, PlatformResult};
use std::path::Path;
use windows::Win32::UI::HiDpi::{SetProcessDpiAwareness, PROCESS_PER_MONITOR_DPI_AWARE};

/// Initialize Windows platform features
///
/// - Enables per-monitor DPI awareness for proper scaling on high-DPI displays
pub fn init_platform() -> PlatformResult<()> {
    // Enable per-monitor DPI awareness
    // This ensures correct sizing on high-DPI displays and mixed-DPI multi-monitor setups
    unsafe {
        // Try per-monitor DPI awareness (Windows 8.1+)
        let result = SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE);
        if result.is_err() {
            // Not critical - continue without DPI awareness if it fails
            // This can happen if already set via manifest or on older Windows
            eprintln!(
                "[WARN] Could not set DPI awareness: {:?}",
                result.err().unwrap()
            );
        }
    }

    Ok(())
}

/// Ensure the Windows runtime prerequisites are available.
pub fn ensure_runtime_available() -> PlatformResult<()> {
    if is_webview2_available() {
        Ok(())
    } else {
        Err(PlatformError::Other(
            "WebView2 runtime not available. Install Microsoft Edge or the WebView2 Runtime from https://developer.microsoft.com/microsoft-edge/webview2/".to_string(),
        ))
    }
}

/// Check if WebView2 runtime is available
///
/// Returns true if WebView2 runtime is installed, false otherwise.
/// WebView2 is typically pre-installed on Windows 10 (April 2018+) and Windows 11.
pub fn is_webview2_available() -> bool {
    // Check common WebView2 installation paths
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());

    // WebView2 can be installed in several locations:
    // 1. Evergreen: C:\Program Files (x86)\Microsoft\EdgeWebView\Application
    // 2. System32: %SystemRoot%\System32\msedgewebview2.exe
    // 3. User profile: %LocalAppData%\Microsoft\EdgeWebView

    let paths = [
        Path::new("C:\\Program Files (x86)\\Microsoft\\EdgeWebView\\Application").to_path_buf(),
        Path::new("C:\\Program Files\\Microsoft\\EdgeWebView\\Application").to_path_buf(),
        Path::new(&format!("{}\\System32\\msedgewebview2.exe", system_root)).to_path_buf(),
    ];

    // Also check LocalAppData
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let user_path = Path::new(&local_app_data).join("Microsoft\\EdgeWebView");
        if user_path.exists() {
            return true;
        }
    }

    // Check system paths
    for path in &paths {
        if path.exists() {
            return true;
        }
    }

    false
}
