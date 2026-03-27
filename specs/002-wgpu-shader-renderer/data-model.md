# Data Model: Native wgpu Shader Renderer

**Feature**: 002-wgpu-shader-renderer
**Date**: 2026-03-27

## Entities

### ShaderToyUniforms

The uniform buffer passed to the GPU each frame, containing all ShaderToy-compatible values.

| Field | Type | Size | Description |
|-------|------|------|-------------|
| `i_resolution` | vec3 (f32x3) | 12 bytes | Viewport width, height, pixel aspect ratio (always 1.0) |
| `_pad0` | f32 | 4 bytes | Padding for std140 vec3 alignment |
| `i_time` | f32 | 4 bytes | Elapsed time in seconds, scaled by `time_scale` |
| `i_time_delta` | f32 | 4 bytes | Time since last frame in seconds, scaled by `time_scale` |
| `i_frame` | i32 | 4 bytes | Frame counter, starts at 0 |
| `i_frame_rate` | f32 | 4 bytes | Current frames per second |
| `i_mouse` | vec4 (f32x4) | 16 bytes | (x, y, click_x, click_y) in pixel coordinates |
| `i_date` | vec4 (f32x4) | 16 bytes | (year, month-1, day, seconds since midnight) |

**Total size**: 64 bytes
**Layout**: std140 (GLSL default for uniform blocks)

### ShaderSource

The user-provided shader code and its processed form.

| Attribute | Description |
|-----------|-------------|
| `raw_source` | Original content of the `.shader` file |
| `wrapped_source` | Complete GLSL 450 source with uniform block, user code, and `main()` wrapper |
| `file_path` | Path to the original `.shader` file (for error reporting) |
| `wrapper_line_offset` | Number of lines added before user code (for error location mapping) |

### RenderState

Per-display rendering state managed throughout the wallpaper lifecycle.

| Attribute | Description |
|-----------|-------------|
| `surface` | wgpu Surface bound to the wallpaper window |
| `device` | wgpu Device for GPU operations |
| `queue` | wgpu Queue for command submission |
| `pipeline` | wgpu RenderPipeline with compiled shaders |
| `uniform_buffer` | wgpu Buffer containing ShaderToyUniforms |
| `bind_group` | wgpu BindGroup linking uniform buffer to shader |
| `surface_config` | Surface configuration (format, size, present mode) |
| `start_time` | Timestamp when rendering began |
| `last_frame_time` | Timestamp of previous frame |
| `frame_count` | Running frame counter |
| `render_scale` | Resolution scale factor (0.1-2.0) |
| `time_scale` | Time scale factor (0.0-100.0) |

### WallpaperWindow

Extended from existing platform window, now with dual rendering mode.

| Attribute | Description |
|-----------|-------------|
| `window` | tao Window instance |
| `hwnd` | Platform-specific window handle (HWND on Windows) |
| `display` | Target display information (position, size, DPI) |
| `render_mode` | Either `WebView` (for URLs) or `NativeGpu` (for shaders) |
| `render_state` | Optional RenderState (present only in NativeGpu mode) |
| `webview` | Optional WebView (present only in WebView mode) |

## Relationships

```
WallpaperWindow 1──1 Display
WallpaperWindow 1──0..1 RenderState (NativeGpu mode)
WallpaperWindow 1──0..1 WebView (WebView mode)
RenderState 1──1 ShaderToyUniforms (updated per frame)
RenderState 1──1 ShaderSource (compiled once at startup)
```

## State Transitions

### Renderer Lifecycle

```
Uninitialized
    │ create window + attach to desktop layer
    ▼
WindowReady
    │ create wgpu instance, adapter, device, surface
    ▼
GpuReady
    │ compile shader, create pipeline + uniform buffer
    ▼
Rendering (loop: update uniforms → render pass → present)
    │ stop signal (IPC/Ctrl+C) or GPU device lost
    ▼
ShuttingDown
    │ destroy pipeline, surface, window; restore wallpaper
    ▼
Terminated
```

### Error States

```
WindowReady → Error: no compatible GPU adapter found
GpuReady → Error: shader compilation failed (naga error)
Rendering → Error: GPU device lost (timeout, driver crash)
```
