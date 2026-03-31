//! Linux platform implementation.
//!
//! Supports Wayland compositors via two strategies:
//! - **Layer-shell** (Hyprland, Sway, …) – uses `zwlr_layer_shell_v1` to place
//!   the window in the background layer.
//! - **GNOME** (Mutter) – creates a standard GTK window and relies on a
//!   companion GNOME Shell extension (`wewa-wallpaper@priceless.dev`) to pin it
//!   below all other windows and hide it from Alt+Tab / Activities.

pub mod display;
pub mod wallpaper;

use crate::platform::{PlatformError, PlatformResult};
use std::sync::OnceLock;

/// The compositor strategy detected at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositorType {
    /// wlr-layer-shell capable compositor (Hyprland, Sway, etc.)
    LayerShell,
    /// GNOME Mutter – requires companion GNOME Shell extension.
    Gnome,
}

static GTK_INIT: OnceLock<Result<(), String>> = OnceLock::new();
static COMPOSITOR: OnceLock<CompositorType> = OnceLock::new();

pub fn init_platform() -> PlatformResult<()> {
    Ok(())
}

/// Returns the detected compositor type. Must be called after
/// [`ensure_runtime_available`].
pub fn compositor_type() -> CompositorType {
    *COMPOSITOR
        .get()
        .expect("compositor_type() called before ensure_runtime_available()")
}

pub fn ensure_runtime_available() -> PlatformResult<()> {
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return Err(PlatformError::Other(
            "Wayland session not detected. Linux support currently targets Wayland compositors such as Hyprland.".to_string(),
        ));
    }

    ensure_gtk_ready()?;

    let ct = if gtk_layer_shell::is_supported() {
        CompositorType::LayerShell
    } else if is_gnome_session() {
        ensure_gnome_extension_available()?;
        CompositorType::Gnome
    } else {
        return Err(PlatformError::Other(
            "Unsupported compositor. wewa requires either a layer-shell capable compositor \
             (Hyprland, Sway, …) or GNOME with the wewa companion extension."
                .to_string(),
        ));
    };

    COMPOSITOR.get_or_init(|| ct);
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

/// Check whether the current session is running GNOME.
fn is_gnome_session() -> bool {
    for var in ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP", "DESKTOP_SESSION"] {
        if let Ok(val) = std::env::var(var) {
            if val.to_ascii_uppercase().contains("GNOME") {
                return true;
            }
        }
    }
    false
}

/// Verify the companion GNOME Shell extension is installed and enabled.
fn ensure_gnome_extension_available() -> PlatformResult<()> {
    const EXT_UUID: &str = "wewa-wallpaper@priceless.dev";

    let output = std::process::Command::new("gnome-extensions")
        .args(["info", EXT_UUID])
        .output();

    let is_active = match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.contains("ACTIVE") || stdout.contains("ENABLED")
        }
        Err(_) => false,
    };

    if is_active {
        return Ok(());
    }

    Err(PlatformError::Other(format!(
        "GNOME detected but the wewa companion extension is not active.\n\n\
         Install it:\n\
         \x20 1. Copy the extension:\n\
         \x20    cp -r gnome-extension/{uuid} \\\n\
         \x20      ~/.local/share/gnome-shell/extensions/\n\
         \x20 2. Enable it:\n\
         \x20    gnome-extensions enable {uuid}\n\
         \x20 3. Restart GNOME Shell (log out and back in on Wayland)\n",
        uuid = EXT_UUID,
    )))
}
