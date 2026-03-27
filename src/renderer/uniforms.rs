use std::time::Instant;

/// ShaderToy-compatible uniform buffer, std140 layout (64 bytes total).
///
/// This struct is uploaded to the GPU each frame and maps directly to the
/// GLSL uniform block in the fragment shader wrapper.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShaderToyUniforms {
    /// Viewport resolution (width, height, pixel_aspect_ratio=1.0)
    pub i_resolution: [f32; 3],
    /// Padding for std140 vec3 alignment
    pub _pad0: f32,
    /// Elapsed time in seconds (scaled by time_scale)
    pub i_time: f32,
    /// Frame delta time in seconds (scaled by time_scale)
    pub i_time_delta: f32,
    /// Frame counter (starts at 0)
    pub i_frame: i32,
    /// Current frames per second
    pub i_frame_rate: f32,
    /// Mouse state: (x, y, click_x, click_y) in pixels
    pub i_mouse: [f32; 4],
    /// Date: (year, month-1, day, seconds_since_midnight)
    pub i_date: [f32; 4],
}

impl ShaderToyUniforms {
    pub fn new() -> Self {
        Self {
            i_resolution: [0.0, 0.0, 1.0],
            _pad0: 0.0,
            i_time: 0.0,
            i_time_delta: 0.0,
            i_frame: 0,
            i_frame_rate: 0.0,
            i_mouse: [0.0; 4],
            i_date: [0.0; 4],
        }
    }
}

/// Mutable state tracked across frames for computing uniform values.
pub struct UniformState {
    pub start_time: Instant,
    pub last_frame_time: Instant,
    pub frame_count: i32,
    pub time_scale: f32,
    pub render_scale: f32,
}

impl UniformState {
    pub fn new(time_scale: f32, render_scale: f32) -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_frame_time: now,
            frame_count: 0,
            time_scale,
            render_scale,
        }
    }

    /// Compute updated uniforms for the current frame.
    pub fn update(
        &mut self,
        window_width: u32,
        window_height: u32,
        mouse_x: f32,
        mouse_y: f32,
        mouse_click_x: f32,
        mouse_click_y: f32,
    ) -> ShaderToyUniforms {
        let now = Instant::now();
        let elapsed = now.duration_since(self.start_time).as_secs_f32();
        let delta = now.duration_since(self.last_frame_time).as_secs_f32();

        let fps = if delta > 0.0 { 1.0 / delta } else { 0.0 };

        // Resolution is passed in at the correct render size by the caller
        let render_w = window_width as f32;
        let render_h = window_height as f32;

        // Compute date components
        let date = chrono::Local::now();
        let i_date = [
            date.format("%Y").to_string().parse::<f32>().unwrap_or(0.0),
            date.format("%m").to_string().parse::<f32>().unwrap_or(1.0) - 1.0,
            date.format("%d").to_string().parse::<f32>().unwrap_or(1.0),
            date.format("%H").to_string().parse::<f32>().unwrap_or(0.0) * 3600.0
                + date.format("%M").to_string().parse::<f32>().unwrap_or(0.0) * 60.0
                + date.format("%S").to_string().parse::<f32>().unwrap_or(0.0),
        ];

        let uniforms = ShaderToyUniforms {
            i_resolution: [render_w, render_h, 1.0],
            _pad0: 0.0,
            i_time: elapsed * self.time_scale,
            i_time_delta: delta * self.time_scale,
            i_frame: self.frame_count,
            i_frame_rate: fps,
            i_mouse: [mouse_x, mouse_y, mouse_click_x, mouse_click_y],
            i_date,
        };

        self.last_frame_time = now;
        self.frame_count += 1;

        uniforms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniforms_size() {
        assert_eq!(std::mem::size_of::<ShaderToyUniforms>(), 64);
    }

    #[test]
    fn test_uniforms_new() {
        let u = ShaderToyUniforms::new();
        assert_eq!(u.i_time, 0.0);
        assert_eq!(u.i_frame, 0);
        assert_eq!(u.i_resolution[2], 1.0);
    }

    #[test]
    fn test_uniform_state_update() {
        let mut state = UniformState::new(1.0, 1.0);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let u = state.update(1920, 1080, 0.0, 0.0, 0.0, 0.0);
        assert!(u.i_time > 0.0);
        assert_eq!(u.i_frame, 0);
        assert_eq!(u.i_resolution[0], 1920.0);
        assert_eq!(u.i_resolution[1], 1080.0);

        let u2 = state.update(1920, 1080, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(u2.i_frame, 1);
    }

    #[test]
    fn test_render_scale() {
        // render_scale is applied by the caller before passing width/height,
        // so update() uses the values as-is.
        let mut state = UniformState::new(1.0, 0.5);
        let u = state.update(960, 540, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(u.i_resolution[0], 960.0);
        assert_eq!(u.i_resolution[1], 540.0);
    }

    #[test]
    fn test_time_scale() {
        let mut state = UniformState::new(2.0, 1.0);
        std::thread::sleep(std::time::Duration::from_millis(50));
        let u = state.update(100, 100, 0.0, 0.0, 0.0, 0.0);
        // time_scale=2.0 means i_time should be roughly 2x the elapsed time
        // elapsed is ~0.05s, so i_time should be ~0.1
        assert!(u.i_time > 0.05);
    }
}
