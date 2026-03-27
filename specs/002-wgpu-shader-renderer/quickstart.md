# Quickstart: Native wgpu Shader Renderer

**Feature**: 002-wgpu-shader-renderer
**Date**: 2026-03-27

## Prerequisites

- Rust 1.87+ (required by wgpu 29.x)
- GPU with Vulkan 1.0, Direct3D 12, Metal, or OpenGL 3.3+ support
- Platform-specific requirements:
  - **Windows**: Windows 10+ with up-to-date GPU drivers
  - **Linux**: Vulkan drivers installed, Wayland or X11
  - **macOS**: macOS 10.15+ (Metal support)

## New Dependencies

Add to `Cargo.toml`:

```toml
wgpu = { version = "29", features = ["glsl"] }
pollster = "0.4"
bytemuck = { version = "1", features = ["derive"] }
raw-window-handle = "0.6"
```

## Architecture Overview

```
CLI Input (.shader file)
    │
    ├── URL/web page → existing WebView pipeline (unchanged)
    │
    └── .shader file → NEW native GPU pipeline:
            │
            ├── Read .shader source
            ├── Wrap in GLSL 450 template (uniform block + main())
            ├── Create platform window (tao) → attach to desktop
            ├── Create wgpu surface on window handle
            ├── Compile shader via naga (GLSL → backend)
            ├── Build render pipeline (fullscreen triangle)
            └── Render loop:
                  ├── Update uniform buffer (time, mouse, etc.)
                  ├── Execute render pass
                  └── Present frame (VSync)
```

## Key Module Changes

### New: `src/renderer/`

```
src/renderer/
├── mod.rs          # Public API: create_renderer(), RenderState
├── shader.rs       # GLSL wrapping, compilation, error mapping
├── uniforms.rs     # ShaderToyUniforms struct + update logic
└── pipeline.rs     # wgpu pipeline setup (surface, device, pipeline)
```

### Modified: `src/main.rs`

The shader detection branch in `handle_start()` now calls the native renderer instead of creating an HTML bundle + HTTP server.

### Modified: `src/platform/*/wallpaper.rs`

Each platform's wallpaper creation gains a second code path:
- **WebView mode** (URLs): existing wry/WebView2/WebKitGTK/WKWebView path
- **NativeGpu mode** (shaders): create tao window → extract handle → create wgpu surface → run render loop

### Unchanged

- `src/cli.rs` — no CLI changes
- `src/config.rs` — no config changes
- `src/ipc.rs` — no IPC protocol changes
- `src/server.rs` — still used for URL mode; not used for shader mode
- `src/display.rs` — no display model changes

## Build & Test

```bash
# Build
cargo build

# Run a shader
cargo run -- path/to/shader.shader

# Run with options
cargo run -- path/to/shader.shader --scale 0.5 --time-scale 1.5

# Run tests
cargo test

# Lint
cargo clippy
```

## Validation

1. **Visual correctness**: Compare shader output side-by-side with current WebView rendering
2. **Memory usage**: Check process memory in Task Manager / htop (target: < 80 MB)
3. **All sample shaders**: Run each of the 38 shaders in `shaders/` directory
4. **Lifecycle**: Verify `--stop`, `--stopall`, and Ctrl+C work correctly
