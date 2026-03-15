//! Linux platform implementation.
//!
//! The initial Linux target is Wayland compositors with layer-shell support,
//! with Hyprland as the primary supported environment.

pub mod display;
pub mod wallpaper;

use crate::platform::{PlatformError, PlatformResult};
use std::sync::OnceLock;

static GTK_INIT: OnceLock<Result<(), String>> = OnceLock::new();

pub fn init_platform() -> PlatformResult<()> {
    Ok(())
}

pub fn ensure_runtime_available() -> PlatformResult<()> {
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return Err(PlatformError::Other(
            "Wayland session not detected. Linux support currently targets Wayland compositors such as Hyprland.".to_string(),
        ));
    }

    ensure_gtk_ready()?;

    if !gtk_layer_shell::is_supported() {
        return Err(PlatformError::Other(
            "Wayland layer-shell protocol is not available. This build currently requires a layer-shell capable compositor such as Hyprland.".to_string(),
        ));
    }

    Ok(())
}

pub(super) fn ensure_gtk_ready() -> PlatformResult<()> {
    match GTK_INIT.get_or_init(|| gtk::init().map_err(|e| e.to_string())) {
        Ok(()) => Ok(()),
        Err(msg) => Err(PlatformError::LinuxError(format!(
            "Failed to initialize GTK: {}",
            msg
        ))),
    }
}
