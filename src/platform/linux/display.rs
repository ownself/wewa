//! Linux display enumeration using GDK monitor APIs.

use super::ensure_gtk_ready;
use crate::display::Display;
use crate::platform::{PlatformError, PlatformResult};
use gdk::prelude::*;

/// Enumerate all connected displays on the current Wayland session.
pub fn enumerate_displays() -> PlatformResult<Vec<Display>> {
    ensure_gtk_ready()?;

    let gdk_display = gdk::Display::default().ok_or_else(|| {
        PlatformError::DisplayEnumerationFailed("No active GDK display available".to_string())
    })?;

    let mut displays = Vec::new();
    for index in 0..gdk_display.n_monitors() {
        if let Some(monitor) = gdk_display.monitor(index) {
            let geometry = monitor.geometry();
            let workarea = monitor.workarea();

            displays.push(Display::new(
                index as u32,
                geometry.x(),
                geometry.y(),
                geometry.width() as u32,
                geometry.height() as u32,
                workarea.x(),
                workarea.y(),
                workarea.width() as u32,
                workarea.height() as u32,
                monitor.is_primary(),
            ));
        }
    }

    if displays.is_empty() {
        return Err(PlatformError::DisplayEnumerationFailed(
            "No monitors reported by GDK".to_string(),
        ));
    }

    if !displays.iter().any(|display| display.is_primary) {
        if let Some(first) = displays.first_mut() {
            first.is_primary = true;
        }
    }

    Ok(displays)
}

/// Print display information for verbose output.
pub fn print_display_info(displays: &[Display]) {
    println!("[INFO] Found {} display(s)", displays.len());
    for display in displays {
        println!(
            "[INFO] Display {}: {}x{} at ({}, {}){}",
            display.index,
            display.width,
            display.height,
            display.x,
            display.y,
            if display.is_primary { " [Primary]" } else { "" }
        );
        println!(
            "[INFO]   Work area: {}x{} at ({}, {})",
            display.work_width, display.work_height, display.work_x, display.work_y
        );
    }
}
