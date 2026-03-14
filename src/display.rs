//! Display/monitor enumeration
//!
//! Defines the Display struct representing a physical or virtual monitor,
//! and provides cross-platform abstractions for display enumeration.

/// Represents a physical or virtual monitor
#[derive(Debug, Clone)]
pub struct Display {
    /// 0-based display index
    pub index: u32,
    /// Left position in virtual screen coordinates
    pub x: i32,
    /// Top position in virtual screen coordinates
    pub y: i32,
    /// Display width in pixels
    pub width: u32,
    /// Display height in pixels
    pub height: u32,
    /// Work area left (excludes taskbar)
    pub work_x: i32,
    /// Work area top
    pub work_y: i32,
    /// Work area width
    pub work_width: u32,
    /// Work area height
    pub work_height: u32,
    /// Whether this is the primary display
    pub is_primary: bool,
}

impl Display {
    /// Create a new Display instance
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        index: u32,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        work_x: i32,
        work_y: i32,
        work_width: u32,
        work_height: u32,
        is_primary: bool,
    ) -> Self {
        Self {
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
        }
    }

    /// Get the full display rectangle (x, y, width, height)
    pub fn full_rect(&self) -> (i32, i32, u32, u32) {
        (self.x, self.y, self.width, self.height)
    }

    /// Get the work area rectangle (x, y, width, height)
    pub fn work_rect(&self) -> (i32, i32, u32, u32) {
        (self.work_x, self.work_y, self.work_width, self.work_height)
    }
}

impl std::fmt::Display for Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Display {}: {}x{} at ({}, {}){}",
            self.index,
            self.width,
            self.height,
            self.x,
            self.y,
            if self.is_primary { " [Primary]" } else { "" }
        )
    }
}

/// Find a display by index from a list of displays
pub fn find_display_by_index(displays: &[Display], index: u32) -> Option<&Display> {
    displays.iter().find(|d| d.index == index)
}

/// Get the primary display from a list of displays
pub fn find_primary_display(displays: &[Display]) -> Option<&Display> {
    displays.iter().find(|d| d.is_primary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_creation() {
        let display = Display::new(0, 0, 0, 1920, 1080, 0, 0, 1920, 1040, true);
        assert_eq!(display.index, 0);
        assert_eq!(display.width, 1920);
        assert_eq!(display.height, 1080);
        assert!(display.is_primary);
    }

    #[test]
    fn test_display_formatting() {
        let display = Display::new(0, 0, 0, 1920, 1080, 0, 0, 1920, 1040, true);
        let formatted = format!("{}", display);
        assert!(formatted.contains("Display 0"));
        assert!(formatted.contains("1920x1080"));
        assert!(formatted.contains("[Primary]"));
    }

    #[test]
    fn test_find_display() {
        let displays = vec![
            Display::new(0, 0, 0, 1920, 1080, 0, 0, 1920, 1040, true),
            Display::new(1, 1920, 0, 1920, 1080, 1920, 0, 1920, 1080, false),
        ];

        let found = find_display_by_index(&displays, 1);
        assert!(found.is_some());
        assert_eq!(found.unwrap().index, 1);

        let not_found = find_display_by_index(&displays, 99);
        assert!(not_found.is_none());
    }
}
