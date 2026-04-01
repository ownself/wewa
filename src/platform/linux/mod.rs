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

const EXT_UUID: &str = "wewa-wallpaper@priceless.dev";

const EXT_EXTENSION_JS: &str =
    include_str!("../../../gnome-extension/wewa-wallpaper@priceless.dev/extension.js");
const EXT_METADATA_JSON: &str =
    include_str!("../../../gnome-extension/wewa-wallpaper@priceless.dev/metadata.json");
const EXT_STYLESHEET_CSS: &str =
    include_str!("../../../gnome-extension/wewa-wallpaper@priceless.dev/stylesheet.css");

/// Check if the extension is active. Returns `true` if gnome-extensions
/// reports it as ACTIVE or ENABLED.
fn is_extension_active() -> bool {
    std::process::Command::new("gnome-extensions")
        .args(["info", EXT_UUID])
        .output()
        .map(|out| {
            let s = String::from_utf8_lossy(&out.stdout);
            s.contains("ACTIVE") || s.contains("ENABLED")
        })
        .unwrap_or(false)
}

/// Install the embedded extension files to
/// `~/.local/share/gnome-shell/extensions/<uuid>/`.
fn install_extension() -> PlatformResult<()> {
    let base = dirs::data_dir()
        .ok_or_else(|| PlatformError::Other("Cannot determine XDG data directory".into()))?;
    let ext_dir = base.join("gnome-shell/extensions").join(EXT_UUID);

    std::fs::create_dir_all(&ext_dir).map_err(|e| {
        PlatformError::Other(format!("Failed to create extension directory: {}", e))
    })?;

    for (name, content) in [
        ("extension.js", EXT_EXTENSION_JS),
        ("metadata.json", EXT_METADATA_JSON),
        ("stylesheet.css", EXT_STYLESHEET_CSS),
    ] {
        std::fs::write(ext_dir.join(name), content).map_err(|e| {
            PlatformError::Other(format!("Failed to write {}: {}", name, e))
        })?;
    }

    Ok(())
}

/// Enable the extension via `gnome-extensions enable`.
fn enable_extension() -> PlatformResult<()> {
    let status = std::process::Command::new("gnome-extensions")
        .args(["enable", EXT_UUID])
        .status()
        .map_err(|e| PlatformError::Other(format!("Failed to run gnome-extensions: {}", e)))?;

    if !status.success() {
        return Err(PlatformError::Other(
            "gnome-extensions enable failed".into(),
        ));
    }
    Ok(())
}

/// Ensure the companion GNOME Shell extension is installed and enabled.
/// If missing, automatically install and enable it, then prompt for a
/// GNOME Shell restart.
fn ensure_gnome_extension_available() -> PlatformResult<()> {
    if is_extension_active() {
        return Ok(());
    }

    println!("[INFO] Installing wewa GNOME Shell extension...");
    install_extension()?;

    // Try to enable — this may fail on first install because GNOME Shell
    // hasn't loaded the extension yet. That is expected; a restart is
    // required regardless.
    let _ = enable_extension();

    // Check if it became active immediately (rare but possible).
    if is_extension_active() {
        println!("[INFO] Extension activated successfully.");
        return Ok(());
    }

    Err(PlatformError::Other(
        "The wewa GNOME Shell extension has been installed successfully, \
         but GNOME Shell must be restarted to load it.\n\
         Please log out and log back in, then run wewa again."
            .into(),
    ))
}
