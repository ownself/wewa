pub mod pipeline;
pub mod shader;
pub mod uniforms;

use pipeline::{PipelineError, PipelineState};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use shader::wrap_shadertoy_glsl;
use uniforms::UniformState;

/// Error type for the native renderer.
#[derive(Debug)]
pub enum RendererError {
    Pipeline(PipelineError),
    Surface(String),
}

impl RendererError {
    /// Returns true if this error is a shader compilation failure.
    pub fn is_shader_error(&self) -> bool {
        matches!(self, RendererError::Pipeline(PipelineError::ShaderError(_)))
    }

    /// Returns true if this error is a missing GPU adapter.
    pub fn is_no_adapter(&self) -> bool {
        matches!(self, RendererError::Pipeline(PipelineError::NoAdapter))
    }
}

impl std::fmt::Display for RendererError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RendererError::Pipeline(e) => write!(f, "{}", e),
            RendererError::Surface(e) => write!(f, "surface error: {}", e),
        }
    }
}

impl std::error::Error for RendererError {}

impl From<PipelineError> for RendererError {
    fn from(e: PipelineError) -> Self {
        RendererError::Pipeline(e)
    }
}

/// Native GPU renderer for ShaderToy-compatible GLSL shaders.
///
/// Ties together the wgpu pipeline, uniform state, and render loop.
pub struct NativeRenderer {
    state: PipelineState,
    uniform_state: UniformState,
}

impl NativeRenderer {
    /// Create a new native renderer from raw window handles.
    ///
    /// # Safety
    /// The raw window and display handles must be valid for the lifetime of
    /// the returned `NativeRenderer`.
    pub unsafe fn new(
        raw_window: RawWindowHandle,
        raw_display: RawDisplayHandle,
        width: u32,
        height: u32,
        shader_source: &str,
        render_scale: f32,
        time_scale: f32,
    ) -> Result<Self, RendererError> {
        let wrapped = wrap_shadertoy_glsl(shader_source);

        let state = pipeline::create_wgpu_pipeline(
            raw_window,
            raw_display,
            width,
            height,
            render_scale,
            &wrapped,
        )?;
        let uniform_state = UniformState::new(time_scale, render_scale);

        Ok(Self {
            state,
            uniform_state,
        })
    }

    /// Render a single frame: update uniforms, run render pass, present.
    ///
    /// When render_scale < 1.0, performs two passes:
    /// 1. Render shader to offscreen texture at scaled resolution
    /// 2. Blit offscreen texture to full-resolution surface with linear filtering
    pub fn render_frame(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        mouse_click_x: f32,
        mouse_click_y: f32,
    ) -> Result<(), RendererError> {
        // Uniform resolution should reflect the actual shader render size
        let (shader_width, shader_height) = if let Some(ref offscreen) = self.state.offscreen {
            (offscreen.scaled_width, offscreen.scaled_height)
        } else {
            (self.state.surface_config.width, self.state.surface_config.height)
        };

        // Update uniforms
        let uniforms =
            self.uniform_state
                .update(shader_width, shader_height, mouse_x, mouse_y, mouse_click_x, mouse_click_y);

        self.state.queue.write_buffer(
            &self.state.uniform_buffer,
            0,
            bytemuck::bytes_of(&uniforms),
        );

        // Acquire next frame, handling surface errors
        let output = match self.state.surface.get_current_texture() {
            Ok(output) => output,
            Err(wgpu::SurfaceError::Lost) => {
                // Reconfigure surface and skip this frame
                self.state
                    .surface
                    .configure(&self.state.device, &self.state.surface_config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return Err(RendererError::Surface(
                    "GPU out of memory".to_string(),
                ));
            }
            Err(e) => {
                return Err(RendererError::Surface(e.to_string()));
            }
        };

        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        if let Some(ref offscreen) = self.state.offscreen {
            // Pass 1: Render shader to offscreen texture at scaled resolution
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("shader_render_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &offscreen.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                render_pass.set_pipeline(&self.state.pipeline);
                render_pass.set_bind_group(0, &self.state.bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }

            // Pass 2: Blit offscreen texture to surface with linear filtering
            {
                let mut blit_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("blit_render_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &surface_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                blit_pass.set_pipeline(&offscreen.blit_pipeline);
                blit_pass.set_bind_group(0, &offscreen.blit_bind_group, &[]);
                blit_pass.draw(0..3, 0..1);
            }
        } else {
            // Single pass: render directly to surface (scale == 1.0)
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("shader_render_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &surface_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                render_pass.set_pipeline(&self.state.pipeline);
                render_pass.set_bind_group(0, &self.state.bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }
        }

        self.state.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Reconfigure the surface after a window resize.
    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        // Surface always at full window resolution
        self.state.surface_config.width = new_width.max(1);
        self.state.surface_config.height = new_height.max(1);
        self.state
            .surface
            .configure(&self.state.device, &self.state.surface_config);

        // Resize offscreen texture if scaling is active
        if let Some(ref mut offscreen) = self.state.offscreen {
            let scaled_w = (new_width as f32 * self.uniform_state.render_scale).max(1.0) as u32;
            let scaled_h = (new_height as f32 * self.uniform_state.render_scale).max(1.0) as u32;
            pipeline::recreate_offscreen(&self.state.device, offscreen, scaled_w, scaled_h);
        }
    }

    /// Consume the renderer and drop all GPU resources.
    pub fn shutdown(self) {
        // All resources are dropped when self is consumed
        drop(self);
    }
}
