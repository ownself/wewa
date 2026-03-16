//! macOS display enumeration using Core Graphics.

use crate::display::Display;
use crate::platform::{PlatformError, PlatformResult};
use core_graphics::display::CGDisplay;

/// Enumerate all active displays.
pub fn enumerate_displays() -> PlatformResult<Vec<Display>> {
    let display_ids = CGDisplay::active_displays().map_err(|e| {
        PlatformError::DisplayEnumerationFailed(format!("CGGetActiveDisplayList failed: {}", e))
    })?;

    if display_ids.is_empty() {
        return Err(PlatformError::DisplayEnumerationFailed(
            "No active displays found".to_string(),
        ));
    }

    let main_id = CGDisplay::main().id;

    let mut displays = Vec::with_capacity(display_ids.len());
    for (index, &id) in display_ids.iter().enumerate() {
        let cg = CGDisplay::new(id);
        let bounds = cg.bounds();

        // Core Graphics returns logical (point) coordinates — this matches
        // what tao expects for window positioning.
        let x = bounds.origin.x as i32;
        let y = bounds.origin.y as i32;
        let width = bounds.size.width as u32;
        let height = bounds.size.height as u32;

        // macOS does not expose a separate "work area" through Core Graphics.
        // The menu bar and Dock insets vary and are managed by AppKit.
        // For a wallpaper that covers the full screen this is irrelevant,
        // so we set work area equal to the full bounds.
        displays.push(Display::new(
            index as u32,
            x,
            y,
            width,
            height,
            x,
            y,
            width,
            height,
            id == main_id,
        ));
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
    }
}
