//! Wallpaper window management
//!
//! Defines the WallpaperConfig and provides cross-platform abstraction
//! for creating and managing wallpaper windows.

use crate::display::Display;

/// Rendering mode for wallpaper windows
#[derive(Debug, Clone, PartialEq)]
pub enum RenderMode {
    /// Use WebView2 for rendering (URLs, HTML files)
    WebView,
    /// Use native wgpu GPU rendering (shader files)
    NativeGpu,
}

/// Configuration for creating a wallpaper window
#[derive(Debug, Clone)]
pub struct WallpaperConfig {
    /// URL to display in the webview (used in WebView mode)
    pub url: String,
    /// Target display for the wallpaper
    pub display: Display,
    /// Whether to enable verbose logging
    pub verbose: bool,
    /// Rendering mode
    pub render_mode: RenderMode,
    /// Shader source code (used in NativeGpu mode)
    pub shader_source: Option<String>,
    /// Render scale factor (0.1 - 2.0)
    pub scale: f32,
    /// Time scale factor
    pub time_scale: f32,
}

impl WallpaperConfig {
    /// Create a new wallpaper configuration for WebView mode
    pub fn new(url: String, display: Display, verbose: bool) -> Self {
        Self {
            url,
            display,
            verbose,
            render_mode: RenderMode::WebView,
            shader_source: None,
            scale: 1.0,
            time_scale: 1.0,
        }
    }

    /// Create a new wallpaper configuration for native GPU shader mode
    pub fn new_native_gpu(
        shader_source: String,
        display: Display,
        scale: f32,
        time_scale: f32,
        verbose: bool,
    ) -> Self {
        Self {
            url: String::new(),
            display,
            verbose,
            render_mode: RenderMode::NativeGpu,
            shader_source: Some(shader_source),
            scale,
            time_scale,
        }
    }
}

/// Result type for wallpaper operations
pub type WallpaperResult<T> = Result<T, WallpaperError>;

/// Errors that can occur during wallpaper operations
#[derive(Debug)]
#[allow(dead_code)]
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
