//! Platform-specific implementations
//!
//! This module provides platform detection and dispatches to the appropriate
//! platform-specific implementations for display enumeration and wallpaper management.

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub use windows::{
    display::{enumerate_displays, print_display_info},
    ensure_runtime_available, init_platform,
    wallpaper::create_wallpapers,
};

#[cfg(target_os = "linux")]
pub use linux::{
    display::{enumerate_displays, print_display_info},
    ensure_runtime_available, init_platform,
    wallpaper::create_wallpapers,
};

#[cfg(target_os = "macos")]
pub use macos::{
    display::{enumerate_displays, print_display_info},
    ensure_runtime_available, init_platform,
    wallpaper::create_wallpapers,
};

/// Platform initialization result
pub type PlatformResult<T> = Result<T, PlatformError>;

/// Platform-specific errors
#[derive(Debug)]
#[allow(dead_code)]
pub enum PlatformError {
    /// Windows-specific error
    #[cfg(target_os = "windows")]
    WindowsError(String),
    /// Linux-specific error
    #[cfg(target_os = "linux")]
    LinuxError(String),
    /// macOS-specific error
    #[cfg(target_os = "macos")]
    MacOSError(String),
    /// Display enumeration failed
    DisplayEnumerationFailed(String),
    /// Webview creation failed
    WebviewError(String),
    /// Generic platform error
    Other(String),
}

impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(target_os = "windows")]
            PlatformError::WindowsError(msg) => write!(f, "Windows error: {}", msg),
            #[cfg(target_os = "linux")]
            PlatformError::LinuxError(msg) => write!(f, "Linux error: {}", msg),
            #[cfg(target_os = "macos")]
            PlatformError::MacOSError(msg) => write!(f, "macOS error: {}", msg),
            PlatformError::DisplayEnumerationFailed(msg) => {
                write!(f, "Display enumeration failed: {}", msg)
            }
            PlatformError::WebviewError(msg) => write!(f, "Webview error: {}", msg),
            PlatformError::Other(msg) => write!(f, "Platform error: {}", msg),
        }
    }
}

impl std::error::Error for PlatformError {}
