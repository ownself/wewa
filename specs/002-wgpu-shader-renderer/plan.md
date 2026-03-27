# Implementation Plan: Native wgpu Shader Renderer

**Branch**: `002-wgpu-shader-renderer` | **Date**: 2026-03-27 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/002-wgpu-shader-renderer/spec.md`

## Summary

Replace the WebView-based WebGL rendering pipeline for `.shader` files with a native wgpu renderer that directly accesses the GPU via Vulkan/D3D12/Metal/OpenGL backends. This eliminates the browser engine overhead (200-500 MB memory, multi-process architecture, JS runtime) and replaces it with a lightweight native render loop (~10-30 MB). ShaderToy GLSL compatibility is maintained by wrapping user shaders in a GLSL 450 template and using naga for shader translation. The existing URL/web page wallpaper mode remains entirely on WebView.

## Technical Context

**Language/Version**: Rust 1.87+ (2021 edition) — bumped from 1.75+ due to wgpu 29.x MSRV requirement
**Primary Dependencies**: wgpu 29 (with `glsl` feature), tao 0.30 (existing), pollster 0.4, bytemuck 1.x, raw-window-handle 0.6
**Storage**: N/A (no persistent storage changes)
**Testing**: cargo test, cargo clippy, manual visual verification against WebView output
**Target Platform**: Windows 10+, Linux (Wayland/X11), macOS 10.15+
**Project Type**: CLI / desktop-app (hybrid)
**Performance Goals**: 60fps at display refresh rate, <5% CPU steady state, <80 MB memory
**Constraints**: <80 MB memory, <2s startup, must coexist with WebView mode for URLs
**Scale/Scope**: Single-user desktop application, 38 existing shader files as compatibility baseline

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Constitution is unconfigured (template placeholders only). No gates to evaluate. Proceeding.

## Project Structure

### Documentation (this feature)

```text
specs/002-wgpu-shader-renderer/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0: technical decisions and rationale
├── data-model.md        # Phase 1: entity definitions and relationships
├── quickstart.md        # Phase 1: build and run guide
├── contracts/
│   └── cli-contract.md  # Phase 1: CLI interface contract (no changes)
└── checklists/
    └── requirements.md  # Spec quality checklist
```

### Source Code (repository root)

```text
src/
├── main.rs                         # Modified: shader branch calls native renderer
├── cli.rs                          # Unchanged
├── config.rs                       # Unchanged
├── server.rs                       # Unchanged (used only for URL mode)
├── ipc.rs                          # Unchanged
├── display.rs                      # Unchanged
├── wallpaper.rs                    # Unchanged
├── shader.rs                       # Modified: retain validation, remove HTML generation
├── renderer/                       # NEW: native GPU rendering module
│   ├── mod.rs                      #   Public API and RenderState lifecycle
│   ├── shader.rs                   #   GLSL wrapping, compilation, error mapping
│   ├── uniforms.rs                 #   ShaderToyUniforms struct and per-frame updates
│   └── pipeline.rs                 #   wgpu instance/adapter/device/surface/pipeline setup
└── platform/
    ├── windows/
    │   ├── wallpaper.rs            # Modified: add NativeGpu rendering path
    │   ├── display.rs              # Unchanged
    │   └── mod.rs                  # Unchanged
    ├── linux/
    │   ├── wallpaper.rs            # Modified: add NativeGpu rendering path
    │   ├── display.rs              # Unchanged
    │   └── mod.rs                  # Unchanged
    └── macos/
        ├── wallpaper.rs            # Modified: add NativeGpu rendering path
        ├── display.rs              # Unchanged
        └── mod.rs                  # Unchanged

tests/
├── shader_compilation_tests.rs     # NEW: GLSL wrapping and compilation tests
└── uniform_tests.rs                # NEW: uniform buffer layout and update tests
```

**Structure Decision**: Follows existing `src/` flat module structure. The new `renderer/` module is the only structural addition, containing all wgpu-specific code isolated from the rest of the codebase. Platform-specific wallpaper files gain a second rendering path (NativeGpu) alongside the existing WebView path.

## Key Technical Decisions

All decisions are documented in detail in [research.md](research.md). Summary:

| # | Decision | Choice | Key Rationale |
|---|----------|--------|---------------|
| 1 | Window library | Keep tao, extract raw handles | Preserves WebView integration for URL mode |
| 2 | GLSL strategy | wgpu `glsl` feature + naga | Direct GLSL ingestion, no external tools |
| 3 | Render geometry | Fullscreen triangle (3 verts) | More efficient than quad, no vertex buffer |
| 4 | GLSL version | `#version 450` with UBO | Best naga support, std140 layout |
| 5 | Surface creation | `SurfaceTargetUnsafe::RawHandle` | Works with tao + WorkerW on Windows |
| 6 | Power management | LowPower + Fifo VSync | Integrated GPU preference, natural fps cap |
| 7 | Integration | New `renderer` module, conditional branch | Additive architecture, WebView untouched |
| 8 | Uniform layout | 64-byte `#[repr(C)]` struct | std140-compatible, Pod/Zeroable via bytemuck |
| 9 | Error handling | stderr + source-mapped line numbers | No visual overlay needed for native mode |
| 10 | Dependencies | wgpu 29, pollster 0.4, bytemuck 1 | Latest stable, minimal additions |

## Implementation Phases

### Phase 1: Core Renderer (P1 — MVP)

**Goal**: A single shader renders correctly on a single Windows display via wgpu.

1. Add wgpu dependencies to `Cargo.toml`
2. Create `src/renderer/uniforms.rs` — `ShaderToyUniforms` struct with bytemuck derive
3. Create `src/renderer/shader.rs` — GLSL wrapper template, shader compilation, error mapping
4. Create `src/renderer/pipeline.rs` — wgpu instance, adapter, device, surface, render pipeline setup
5. Create `src/renderer/mod.rs` — public API: `NativeRenderer::new()`, `render_frame()`, `resize()`, `shutdown()`
6. Modify `src/platform/windows/wallpaper.rs` — add NativeGpu path: create tao window → attach to WorkerW → extract HWND → create wgpu surface → run render loop
7. Modify `src/main.rs` — shader branch calls native renderer instead of HTML bundle + HTTP server
8. Modify `src/shader.rs` — keep `is_shader_file()` and validation; extract HTML generation to a legacy code path or remove

**Validation**: Run `plasma.shader` as wallpaper, verify visual output, check memory < 80 MB.

### Phase 2: Parameters and Quality (P2)

**Goal**: Scale/time-scale support and multi-monitor.

1. Implement `--scale` in renderer — render to smaller texture, wgpu presents upscaled
2. Implement `--time-scale` in uniform update logic — multiply elapsed time
3. Multi-monitor: create per-display RenderState, each with independent surface and uniform buffer
4. Handle display resize events — reconfigure surface on size change
5. DPI-awareness — use display scale factor for correct resolution

**Validation**: Run with `--scale 0.5 --time-scale 2.0` on multi-monitor setup.

### Phase 3: Error Handling and Lifecycle (P3)

**Goal**: Production-quality error reporting and lifecycle integration.

1. Shader compilation errors — map naga error spans back to user source lines, format for stderr
2. No GPU adapter — detect and report with clear message and exit code 5
3. GPU device lost — detect and exit gracefully with error message
4. IPC integration — existing `--stop`/`--stopall` triggers shutdown of render loop
5. Ctrl+C integration — existing signal handler triggers render loop exit
6. Instance tracking — save/cleanup `WallpaperInstance` metadata as before

**Validation**: Test with broken shaders, `--stop`, Ctrl+C, and on systems without GPU.

### Phase 4: Cross-Platform and Compatibility (P2-P3)

**Goal**: Linux and macOS support, full shader compatibility.

1. Linux — extract window handle from GTK/tao window, create wgpu surface, integrate with layer-shell
2. macOS — extract NSView from tao window, create wgpu surface, set desktop level
3. Run all 38 sample shaders on each platform
4. Fix any naga GLSL translation issues found during shader testing

**Validation**: All 38 shaders render correctly on Windows, Linux, macOS.

## Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Naga GLSL translation fails for some ShaderToy idioms | High | Medium | Test all 38 shaders early; fall back to WebView for incompatible shaders |
| wgpu surface on WorkerW child has rendering artifacts | Medium | Low | Proven technique in existing Rust wallpaper engines; test early |
| MSRV bump to 1.87 breaks CI/user builds | Low | Low | Document clearly; Rust 1.87 is widely available |
| tao + wgpu raw-window-handle version mismatch | Medium | Medium | Pin versions; verify at build time |
| Mouse input unavailable behind desktop icons | Low | High | Document as known limitation; `iMouse` works best-effort |

## Complexity Tracking

No constitution violations to justify.
