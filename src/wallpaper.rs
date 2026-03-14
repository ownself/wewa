//! Wallpaper window management
//!
//! Defines the WallpaperConfig and provides cross-platform abstraction
//! for creating and managing wallpaper windows.

use crate::display::Display;

/// Configuration for creating a wallpaper window
#[derive(Debug, Clone)]
pub struct WallpaperConfig {
    /// URL to display in the webview
    pub url: String,
    /// Target display for the wallpaper
    pub display: Display,
    /// Whether to enable verbose logging
    pub verbose: bool,
}

impl WallpaperConfig {
    /// Create a new wallpaper configuration
    pub fn new(url: String, display: Display, verbose: bool) -> Self {
        Self {
            url,
            display,
            verbose,
        }
    }
}

/// Result type for wallpaper operations
pub type WallpaperResult<T> = Result<T, WallpaperError>;

/// Errors that can occur during wallpaper operations
#[derive(Debug)]
pub enum WallpaperError {
    /// Failed to create the webview window
    WindowCreationFailed(String),
    /// Failed to apply window styles
    StyleApplicationFailed(String),
    /// Failed to load the URL
    UrlLoadFailed(String),
    /// Platform-specific error
    PlatformError(String),
}

impl std::fmt::Display for WallpaperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WallpaperError::WindowCreationFailed(msg) => {
                write!(f, "Window creation failed: {}", msg)
            }
            WallpaperError::StyleApplicationFailed(msg) => {
                write!(f, "Style application failed: {}", msg)
            }
            WallpaperError::UrlLoadFailed(msg) => write!(f, "URL load failed: {}", msg),
            WallpaperError::PlatformError(msg) => write!(f, "Platform error: {}", msg),
        }
    }
}

impl std::error::Error for WallpaperError {}
