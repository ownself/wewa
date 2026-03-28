use super::shader::{WrappedShader, BLIT_SHADER_WGSL, FULLSCREEN_QUAD_WGSL};
use super::uniforms::ShaderToyUniforms;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::borrow::Cow;

/// All GPU state needed for rendering.
pub struct PipelineState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    /// Non-sRGB format used for render pass views to bypass gamma conversion.
    pub render_format: wgpu::TextureFormat,
    pub pipeline: wgpu::RenderPipeline,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    /// When render_scale != 1.0, shader renders to this offscreen texture
    /// which is then blitted to the full-resolution surface with linear filtering.
    pub offscreen: Option<OffscreenState>,
}

/// Resources for scaled (offscreen) rendering + upscale blit.
pub struct OffscreenState {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub blit_pipeline: wgpu::RenderPipeline,
    pub blit_bind_group: wgpu::BindGroup,
    pub blit_bind_group_layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
    pub scaled_width: u32,
    pub scaled_height: u32,
    pub format: wgpu::TextureFormat,
}

/// Error type for pipeline creation.
#[derive(Debug)]
pub enum PipelineError {
    NoAdapter,
    DeviceError(String),
    SurfaceError(String),
    ShaderError(String),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineError::NoAdapter => write!(
                f,
                "no compatible GPU adapter found — requires Vulkan 1.0, Direct3D 12, Metal, or OpenGL 3.3+"
            ),
            PipelineError::DeviceError(e) => write!(f, "GPU device error: {}", e),
            PipelineError::SurfaceError(e) => write!(f, "surface error: {}", e),
            PipelineError::ShaderError(e) => write!(f, "shader compilation failed: {}", e),
        }
    }
}

impl std::error::Error for PipelineError {}

/// Create the complete wgpu rendering pipeline from raw window handles.
///
/// When `render_scale` < 1.0, an offscreen texture is created at the scaled
/// resolution and a blit pipeline upscales it to the full surface with linear
/// interpolation.
///
/// # Safety
/// The raw window and display handles must be valid for the lifetime of the returned `PipelineState`.
pub unsafe fn create_wgpu_pipeline(
    raw_window: RawWindowHandle,
    raw_display: RawDisplayHandle,
    width: u32,
    height: u32,
    render_scale: f32,
    wrapped_shader: &WrappedShader,
) -> Result<PipelineState, PipelineError> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });

    // Create surface from raw handles
    let target = wgpu::SurfaceTargetUnsafe::RawHandle {
        raw_display_handle: raw_display,
        raw_window_handle: raw_window,
    };
    let surface = instance
        .create_surface_unsafe(target)
        .map_err(|e| PipelineError::SurfaceError(e.to_string()))?;

    // Request adapter with low-power preference
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .map_err(|_| PipelineError::NoAdapter)?;

    // Request device
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("webwallpaper"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        },
    ))
    .map_err(|e: wgpu::RequestDeviceError| PipelineError::DeviceError(e.to_string()))?;

    // Configure surface at FULL window resolution (always)
    let surface_caps = surface.get_capabilities(&adapter);
    // Prefer sRGB surface for storage precision (more bits in dark tones),
    // but render through a non-sRGB view so the GPU does NOT apply automatic
    // linear→sRGB gamma. ShaderToy shaders output sRGB-space colors directly;
    // writing through a Unorm view stores them as-is with sRGB encoding benefit.
    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    // Derive the non-sRGB (linear/Unorm) variant for the render view.
    // e.g. Bgra8UnormSrgb → Bgra8Unorm, Rgba8UnormSrgb → Rgba8Unorm
    let render_format = match surface_format {
        wgpu::TextureFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8Unorm,
        wgpu::TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8Unorm,
        other => other, // fallback: use as-is
    };

    // Prefer Mailbox (non-blocking triple-buffered VSync) for smooth rendering
    // on multi-monitor setups. Fifo blocks on each surface's VSync independently,
    // which halves FPS with 2 monitors. Mailbox drops stale frames instead.
    let present_mode = if surface_caps
        .present_modes
        .contains(&wgpu::PresentMode::Mailbox)
    {
        wgpu::PresentMode::Mailbox
    } else {
        wgpu::PresentMode::Fifo
    };

    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: width.max(1),
        height: height.max(1),
        present_mode,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![render_format],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &surface_config);

    // Create vertex shader (WGSL)
    let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("fullscreen_triangle_vertex"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(FULLSCREEN_QUAD_WGSL)),
    });

    // Create fragment shader (GLSL via naga) with error handling
    device.push_error_scope(wgpu::ErrorFilter::Validation);
    let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shadertoy_fragment"),
        source: wgpu::ShaderSource::Glsl {
            shader: Cow::Owned(wrapped_shader.source.clone()),
            stage: wgpu::naga::ShaderStage::Fragment,
            defines: Default::default(),
        },
    });

    // Check for shader compilation errors
    if let Some(error) = pollster::block_on(device.pop_error_scope()) {
        // Get detailed compilation info with source locations
        let compilation_info = pollster::block_on(fragment_shader.get_compilation_info());
        let mut error_msg = String::new();

        for msg in &compilation_info.messages {
            if msg.message_type == wgpu::CompilationMessageType::Error {
                if let Some(ref loc) = msg.location {
                    // Map line number back to user's shader source
                    let user_line =
                        super::shader::map_error_line(loc.line_number as usize, wrapped_shader.wrapper_line_offset);
                    error_msg.push_str(&format!(
                        "  --> line {}:{}\n   | {}\n",
                        user_line, loc.line_position, msg.message
                    ));
                } else {
                    error_msg.push_str(&format!("   | {}\n", msg.message));
                }
            }
        }

        if error_msg.is_empty() {
            // Fallback to the general error message
            error_msg = error.to_string();
        }

        return Err(PipelineError::ShaderError(error_msg));
    }

    // Create uniform buffer
    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniform_buffer"),
        size: std::mem::size_of::<ShaderToyUniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Create bind group layout and bind group
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("uniform_bind_group_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("uniform_bind_group"),
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    // Determine render target format: when scaling, shader renders to an offscreen
    // texture so use the same surface format for compatibility.
    let scaled_width = (width as f32 * render_scale).max(1.0) as u32;
    let scaled_height = (height as f32 * render_scale).max(1.0) as u32;
    let needs_offscreen = render_scale < 1.0 && (scaled_width != width || scaled_height != height);

    // Create shader pipeline — renders to offscreen texture (scaled) or surface (1:1)
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("render_pipeline_layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("render_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vertex_shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &fragment_shader,
            entry_point: Some("main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: render_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    // Create offscreen resources when scaling
    let offscreen = if needs_offscreen {
        Some(create_offscreen_state(
            &device,
            scaled_width,
            scaled_height,
            render_format,
        ))
    } else {
        None
    };

    Ok(PipelineState {
        device,
        queue,
        surface,
        surface_config,
        render_format,
        pipeline,
        uniform_buffer,
        bind_group,
        offscreen,
    })
}

/// Create offscreen texture and blit pipeline for upscaling.
fn create_offscreen_state(
    device: &wgpu::Device,
    scaled_width: u32,
    scaled_height: u32,
    format: wgpu::TextureFormat,
) -> OffscreenState {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen_render_target"),
        size: wgpu::Extent3d {
            width: scaled_width.max(1),
            height: scaled_height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("blit_sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let blit_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blit_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

    let blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("blit_bind_group"),
        layout: &blit_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("blit_shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(BLIT_SHADER_WGSL)),
    });

    let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("blit_pipeline_layout"),
        bind_group_layouts: &[&blit_bind_group_layout],
        push_constant_ranges: &[],
    });

    let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("blit_pipeline"),
        layout: Some(&blit_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &blit_shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &blit_shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    OffscreenState {
        texture,
        view,
        blit_pipeline,
        blit_bind_group,
        blit_bind_group_layout,
        sampler,
        scaled_width,
        scaled_height,
        format,
    }
}

/// Recreate the offscreen texture after a window resize.
pub fn recreate_offscreen(
    device: &wgpu::Device,
    offscreen: &mut OffscreenState,
    new_scaled_width: u32,
    new_scaled_height: u32,
) {
    offscreen.scaled_width = new_scaled_width.max(1);
    offscreen.scaled_height = new_scaled_height.max(1);

    offscreen.texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen_render_target"),
        size: wgpu::Extent3d {
            width: offscreen.scaled_width,
            height: offscreen.scaled_height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: offscreen.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    offscreen.view = offscreen
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    // Rebuild bind group with the new texture view
    offscreen.blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("blit_bind_group"),
        layout: &offscreen.blit_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&offscreen.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&offscreen.sampler),
            },
        ],
    });
}
