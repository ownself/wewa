# Research: Native wgpu Shader Renderer

**Feature**: 002-wgpu-shader-renderer
**Date**: 2026-03-27

## Decision 1: Window Library (tao vs winit)

**Decision**: Keep tao for window management; create wgpu surfaces via raw HWND/handle extraction.

**Rationale**: The project already uses tao v0.30 for cross-platform window management and it integrates with the existing WebView (wry) workflow for URL mode. Switching to winit would require rewriting all three platform wallpaper implementations. Since wgpu can create surfaces from any raw window handle via `SurfaceTargetUnsafe::RawHandle`, tao windows work fine as wgpu render targets. The WorkerW technique on Windows requires raw HWND manipulation regardless of which window library is used.

**Alternatives considered**:
- **winit**: First-class wgpu support, but would require replacing tao throughout the codebase and breaking wry (WebView) integration for URL mode. Too disruptive.
- **Raw Win32 only**: Possible but would lose cross-platform window abstraction and event loop handling.

## Decision 2: GLSL Support Strategy

**Decision**: Use wgpu's `glsl` feature flag with `ShaderSource::Glsl` for direct GLSL ingestion via naga.

**Rationale**: Naga's GLSL frontend supports `#version 450` and handles translation to all GPU backends internally (SPIR-V for Vulkan, HLSL/DXIL for D3D12, MSL for Metal, GLSL ES for OpenGL). This avoids any manual shader transpilation step. ShaderToy shaders need a wrapper that adds `#version 450`, uniform block declarations, and a `void main()` that calls the user's `mainImage()` function — identical in concept to the current JavaScript wrapper but now in GLSL.

**Alternatives considered**:
- **Pre-convert to WGSL via naga-cli**: Adds a build step and makes error messages harder to map back to user source. Rejected.
- **SPIR-V via shaderc/glslang**: External C dependency, larger binary, slower compilation. Rejected.
- **Rust GPU (write shaders in Rust)**: Completely incompatible with user-provided GLSL shaders. Rejected.

## Decision 3: Rendering Architecture

**Decision**: Fullscreen triangle (3 vertices, no vertex buffer) with a single render pass and one uniform buffer.

**Rationale**: A fullscreen triangle is more efficient than a quad (3 vs 6 vertex shader invocations, no wasted fragment work outside the viewport). The uniform buffer contains all ShaderToy uniforms (`iResolution`, `iTime`, etc.) in a single `#[repr(C)]` struct, updated via `queue.write_buffer()` each frame. No vertex buffer is needed — vertex positions are computed from `vertex_index` in the vertex shader.

**Alternatives considered**:
- **Fullscreen quad (6 vertices)**: Works but wastes 3 vertex invocations and requires a vertex buffer. Rejected for minor efficiency gain.
- **Push constants**: Simpler than a uniform buffer but not supported on all backends (notably OpenGL). Rejected for compatibility.

## Decision 4: GLSL Version and Shader Wrapper

**Decision**: Use `#version 450` GLSL with a uniform block (UBO) for ShaderToy uniforms.

**Rationale**: Naga's GLSL frontend supports `#version 450` reliably. The wrapper template will:
1. Declare a uniform block with `layout(set = 0, binding = 0)` containing all ShaderToy uniforms
2. Include the user's shader source verbatim (which defines `mainImage`)
3. Define `void main()` that calls `mainImage(outColor, gl_FragCoord.xy)`

This approach mirrors the current JavaScript wrapper pattern but targets naga instead of WebGL.

**Wrapper template**:
```glsl
#version 450
precision highp float;

layout(set = 0, binding = 0) uniform Uniforms {
    vec3 iResolution;
    float _pad0;
    float iTime;
    float iTimeDelta;
    int iFrame;
    float iFrameRate;
    vec4 iMouse;
    vec4 iDate;
};

// --- User shader source inserted here ---

layout(location = 0) out vec4 outColor;

void main() {
    mainImage(outColor, gl_FragCoord.xy);
}
```

**Alternatives considered**:
- **`#version 300 es`**: Current WebGL2 version, but naga support is less mature for ESSL. Rejected.
- **`#version 460`**: Partial naga support. Rejected for compatibility.
- **Individual uniforms (not UBO)**: Not supported in wgpu/naga's GLSL path — must use uniform blocks. Rejected.

## Decision 5: Surface Creation on WorkerW Child Windows

**Decision**: Use `wgpu::SurfaceTargetUnsafe::RawHandle` with HWND extracted from tao windows attached to WorkerW.

**Rationale**: The WorkerW technique requires calling `SetParent()` to attach a window as a child of the desktop layer. The resulting HWND is valid and can host a wgpu surface. Using `SurfaceTargetUnsafe::RawHandle` with `raw-window-handle` 0.6.x is the documented way to create surfaces on pre-existing or externally-managed windows. The same approach works on Linux (X11 window ID) and macOS (NSView pointer).

**Safety**: The HWND must remain valid for the lifetime of the Surface. Since the wallpaper window is created before the surface and destroyed after, this is guaranteed by the existing lifecycle management.

## Decision 6: Power and Frame Rate Management

**Decision**: Use `PowerPreference::LowPower` by default, `PresentMode::Fifo` for VSync, no additional frame capping.

**Rationale**: `LowPower` selects integrated GPUs when available, reducing power consumption for always-on wallpaper rendering. `Fifo` (VSync) is the only present mode guaranteed on all platforms and naturally caps frame rate to the display refresh rate (e.g., 60fps). For a wallpaper application, matching the display refresh rate is sufficient — sub-refresh-rate capping (e.g., 30fps) is a future optimization and out of scope.

**Alternatives considered**:
- **Manual 30fps cap**: Reduces GPU work but introduces visible stuttering on 60Hz displays. Rejected for initial implementation.
- **`PowerPreference::HighPerformance`**: Wastes energy on discrete GPU for a background process. Rejected as default.

## Decision 7: Renderer Integration Architecture

**Decision**: Add a new `renderer` module that is conditionally invoked when the input is a `.shader` file, bypassing the WebView/HTTP server pipeline entirely.

**Rationale**: The current pipeline for shaders is: `.shader` → HTML bundle → HTTP server → WebView → WebGL. The native renderer replaces this entire chain with: `.shader` → GLSL wrapper → wgpu pipeline → GPU surface. The shader detection logic in `main.rs` already branches on `is_shader_file()`, so the native renderer can be invoked at this branch point. The HTTP server and HTML template generation become unnecessary for shader mode.

The WebView pipeline remains fully intact for URL/web page mode — this is purely additive architecture.

## Decision 8: Uniform Buffer Layout

**Decision**: Use a single `#[repr(C)]` struct with explicit padding to match GLSL `std140` layout rules.

**Rationale**: GLSL uniform blocks use `std140` layout by default, which requires specific alignment:
- `vec3` is aligned to 16 bytes (takes 12 bytes + 4 padding)
- `float` is 4-byte aligned
- `int` is 4-byte aligned
- `vec4` is 16-byte aligned

```rust
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderToyUniforms {
    i_resolution: [f32; 3],  // vec3, 12 bytes
    _pad0: f32,              // 4 bytes padding (vec3 alignment)
    i_time: f32,             // float, 4 bytes
    i_time_delta: f32,       // float, 4 bytes
    i_frame: i32,            // int, 4 bytes
    i_frame_rate: f32,       // float, 4 bytes
    i_mouse: [f32; 4],       // vec4, 16 bytes
    i_date: [f32; 4],        // vec4, 16 bytes
}
// Total: 64 bytes, std140-compatible
```

## Decision 9: Error Handling for Shader Compilation

**Decision**: Report naga/wgpu shader compilation errors to stderr with source location mapping, then exit with a non-zero exit code.

**Rationale**: The current implementation shows errors in a visual overlay panel in the WebView. For the native renderer, since there is no HTML rendering surface when the shader fails to compile, errors must be reported to the terminal. Naga provides detailed error messages with source spans that can be mapped back to the user's original shader code (accounting for the wrapper lines).

## Decision 10: Dependency Versions

**Decision**: Use wgpu 29.x with the `glsl` feature, keep tao 0.30, add pollster, bytemuck, and raw-window-handle 0.6.

**New Cargo.toml dependencies for shader mode**:
```toml
wgpu = { version = "29", features = ["glsl"] }
pollster = "0.4"
bytemuck = { version = "1", features = ["derive"] }
raw-window-handle = "0.6"
```

**Note**: wgpu 29 requires Rust 1.87+ MSRV. The project currently specifies Rust 1.75+ in CLAUDE.md — this will need to be updated.
