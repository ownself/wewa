//! Windows display enumeration using EnumDisplayMonitors
//!
//! Enumerates all connected monitors and retrieves their geometry and work areas.

use crate::display::Display;
use crate::platform::{PlatformError, PlatformResult};
use std::mem;
use windows::Win32::Foundation::{BOOL, LPARAM, RECT, TRUE};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};

// MONITORINFOF_PRIMARY constant value
const MONITORINFOF_PRIMARY: u32 = 1;

/// Enumerate all connected displays
pub fn enumerate_displays() -> PlatformResult<Vec<Display>> {
    let mut displays: Vec<Display> = Vec::new();

    // Safety: We're passing a valid callback and user data pointer
    unsafe {
        let result = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(monitor_enum_callback),
            LPARAM(&mut displays as *mut Vec<Display> as isize),
        );

        if result.as_bool() {
            // Sort displays by index to ensure consistent ordering
            displays.sort_by_key(|d| d.index);
            Ok(displays)
        } else {
            Err(PlatformError::DisplayEnumerationFailed(
                "EnumDisplayMonitors failed".to_string(),
            ))
        }
    }
}

/// Callback function for EnumDisplayMonitors
unsafe extern "system" fn monitor_enum_callback(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let displays = &mut *(lparam.0 as *mut Vec<Display>);

    // Get monitor info
    let mut monitor_info: MONITORINFO = mem::zeroed();
    monitor_info.cbSize = mem::size_of::<MONITORINFO>() as u32;

    if GetMonitorInfoW(hmonitor, &mut monitor_info).as_bool() {
        let index = displays.len() as u32;

        // Full monitor rectangle
        let rc_monitor = monitor_info.rcMonitor;
        let x = rc_monitor.left;
        let y = rc_monitor.top;
        let width = (rc_monitor.right - rc_monitor.left) as u32;
        let height = (rc_monitor.bottom - rc_monitor.top) as u32;

        // Work area rectangle (excludes taskbar)
        let rc_work = monitor_info.rcWork;
        let work_x = rc_work.left;
        let work_y = rc_work.top;
        let work_width = (rc_work.right - rc_work.left) as u32;
        let work_height = (rc_work.bottom - rc_work.top) as u32;

        // Check if primary monitor
        let is_primary = (monitor_info.dwFlags & MONITORINFOF_PRIMARY) != 0;

        let display = Display::new(
            index,
            x,
            y,
            width,
            height,
            work_x,
            work_y,
            work_width,
            work_height,
            is_primary,
        );

        displays.push(display);
    }

    TRUE // Continue enumeration
}

/// Print display information for verbose output
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_displays() {
        // This test will only pass on Windows with at least one display
        let result = enumerate_displays();
        assert!(result.is_ok());

        let displays = result.unwrap();
        assert!(!displays.is_empty(), "Should find at least one display");

        // Check that at least one is primary
        let has_primary = displays.iter().any(|d| d.is_primary);
        assert!(has_primary, "Should have at least one primary display");
    }
}
