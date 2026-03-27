# Tasks: Native wgpu Shader Renderer

**Input**: Design documents from `/specs/002-wgpu-shader-renderer/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Not explicitly requested in spec. Test tasks are omitted. Manual visual verification is the primary validation method.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add wgpu dependencies and create the renderer module skeleton

- [x] T001 Add wgpu 29 (with `glsl` feature), pollster 0.4, bytemuck 1 (with `derive` feature), and raw-window-handle 0.6 to `Cargo.toml`
- [x] T002 Create `src/renderer/` module directory and `src/renderer/mod.rs` with module declarations for `shader`, `uniforms`, and `pipeline` submodules
- [x] T003 Update Rust edition/MSRV documentation to reflect 1.87+ requirement in `Cargo.toml` and project docs
- [x] T004 Verify `cargo build` succeeds with new dependencies on the target platform

**Checkpoint**: Project compiles with wgpu dependencies. No functional changes yet.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core renderer components that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T005 [P] Implement `ShaderToyUniforms` struct with `#[repr(C)]`, `bytemuck::Pod`, and `bytemuck::Zeroable` derives in `src/renderer/uniforms.rs` — fields: `i_resolution` ([f32; 3]), `_pad0` (f32), `i_time` (f32), `i_time_delta` (f32), `i_frame` (i32), `i_frame_rate` (f32), `i_mouse` ([f32; 4]), `i_date` ([f32; 4]); total 64 bytes std140-compatible
- [x] T006 [P] Implement uniform update function in `src/renderer/uniforms.rs` — `update_uniforms(&mut self, start_time, last_frame_time, frame_count, render_scale, time_scale, window_size, mouse_state)` that computes all ShaderToy uniform values per frame
- [x] T007 [P] Implement GLSL wrapper template in `src/renderer/shader.rs` — function `wrap_shadertoy_glsl(user_source: &str) -> String` that wraps user shader in `#version 450` template with uniform block declaration (`layout(set=0, binding=0)`), inserts user source, and appends `void main()` calling `mainImage(outColor, gl_FragCoord.xy)`; also store `wrapper_line_offset` for error mapping
- [x] T008 [P] Implement fullscreen triangle vertex shader as a WGSL constant string in `src/renderer/shader.rs` — generates 3 vertices covering the entire viewport using `vertex_index`, no vertex buffer required
- [x] T009 Implement `create_wgpu_pipeline()` in `src/renderer/pipeline.rs` — takes raw window handle, display size, and compiled shader source; creates wgpu `Instance` (PRIMARY backends), requests adapter with `PowerPreference::LowPower`, requests device, creates surface via `SurfaceTargetUnsafe::RawHandle`, configures surface with `PresentMode::Fifo` and sRGB format, creates bind group layout for uniform buffer, creates render pipeline with vertex (WGSL) and fragment (GLSL) shaders, returns `PipelineState` struct containing device, queue, surface, surface_config, pipeline, uniform_buffer, and bind_group
- [x] T010 Implement `NativeRenderer` struct and lifecycle API in `src/renderer/mod.rs` — `new(hwnd, display_size, shader_source, scale, time_scale) -> Result<Self>`, `render_frame(&mut self) -> Result<()>` (update uniforms → begin render pass → set pipeline → set bind group → draw 3 vertices → present), `resize(&mut self, new_size)` (reconfigure surface), `shutdown(self)` (drop all GPU resources)

**Checkpoint**: Foundation ready — renderer can be instantiated with a window handle and shader source, renders frames, but not yet integrated into the wallpaper system.

---

## Phase 3: User Story 1 — Run ShaderToy Shaders Natively (Priority: P1) 🎯 MVP

**Goal**: A single `.shader` file renders correctly as desktop wallpaper on Windows using native GPU rendering instead of WebView.

**Independent Test**: Run `cargo run -- shaders/plasma.shader` and verify the shader displays as wallpaper with visually correct output and memory usage under 80 MB.

### Implementation for User Story 1

- [x] T011 [US1] Modify `src/platform/windows/wallpaper.rs` to add a `RenderMode` enum (`WebView` | `NativeGpu`) and branch the wallpaper creation logic — when `NativeGpu`: create tao window (same as current), attach to WorkerW (reuse existing `attach_to_worker_w()`), extract HWND, but skip WebView creation entirely
- [x] T012 [US1] Implement the native GPU render loop in `src/platform/windows/wallpaper.rs` — after attaching window to WorkerW and extracting HWND, create `NativeRenderer` with HWND and display size, then run a render loop using tao's event loop: on `RedrawRequested` call `renderer.render_frame()`, request redraw continuously via `window.request_redraw()` in `MainEventsCleared`
- [x] T013 [US1] Modify `src/main.rs` `handle_start()` function — when `is_shader_file()` is true: read shader source from file, validate `mainImage` presence (reuse existing validation from `shader.rs`), pass shader source string and render parameters directly to the platform wallpaper creation function with `RenderMode::NativeGpu` instead of creating HTML bundle + HTTP server
- [x] T014 [US1] Modify `src/shader.rs` — keep `is_shader_file()`, `validate_scale()`, `validate_time_scale()`, and the `mainImage` validation logic; add a new public function `read_shader_source(path: &Path) -> Result<String>` that reads and validates the shader file; the HTML bundle generation (`build_shader_html`, `create_shader_bundle`, `cleanup_shader_bundle`) remains available for potential fallback but is no longer called in the primary shader path
- [x] T015 [US1] Update `src/wallpaper.rs` `WallpaperConfig` struct to include an optional `shader_source: Option<String>` field and `render_mode: RenderMode` field, so the platform layer knows whether to use WebView or NativeGpu rendering
- [x] T016 [US1] Wire up the complete shader→native pipeline end-to-end: verify `cargo run -- shaders/plasma.shader` creates a wallpaper window attached to WorkerW, renders the shader via wgpu at display resolution with all uniforms updating each frame (iTime advancing, iResolution matching window size)

**Checkpoint**: Single shader renders as wallpaper via wgpu on Windows. Memory < 80 MB. Core MVP is complete.

---

## Phase 4: User Story 2 — Resolution Scaling and Time Control (Priority: P2)

**Goal**: The `--scale` and `--time-scale` CLI options work correctly with the native renderer.

**Independent Test**: Run `cargo run -- shaders/plasma.shader --scale 0.5 --time-scale 2.0` and verify the shader renders at half resolution (upscaled) and animates at double speed.

### Implementation for User Story 2

- [x] T017 [US2] Implement render-scale support in `src/renderer/pipeline.rs` — when `scale != 1.0`, configure the wgpu surface at `(width * scale, height * scale)` so the GPU renders at reduced resolution; wgpu's present will stretch the smaller texture to fill the window automatically
- [x] T018 [US2] Implement time-scale support in `src/renderer/uniforms.rs` `update_uniforms()` — multiply `i_time` by `time_scale`, multiply `i_time_delta` by `time_scale`; when `time_scale == 0.0`, `i_time` and `i_time_delta` remain at 0.0 (frozen)
- [x] T019 [US2] Pass `scale` and `time_scale` parameters through from `WallpaperConfig` to `NativeRenderer::new()` and store them in `RenderState` for use during uniform updates and surface configuration

**Checkpoint**: Scale and time-scale produce equivalent visual results to the current WebView implementation.

---

## Phase 5: User Story 3 — Multi-Monitor Support (Priority: P2)

**Goal**: Each connected display independently renders the shader wallpaper with correct per-display resolution.

**Independent Test**: On a multi-monitor system, run `cargo run -- shaders/plasma.shader` and verify each display shows the shader at its own native resolution.

### Implementation for User Story 3

- [x] T020 [US3] Modify `src/platform/windows/wallpaper.rs` multi-display creation loop — for each display in the `WallpaperConfig` list, create an independent tao window, attach to WorkerW, extract HWND, and create a separate `NativeRenderer` instance with that display's size and position
- [x] T021 [US3] Ensure each `NativeRenderer` instance has independent `ShaderToyUniforms` with correct per-display `iResolution` values matching that display's dimensions (accounting for DPI scale factor)
- [x] T022 [US3] Handle display resize events in the render loop — listen for tao `Resized` events and call `renderer.resize(new_size)` to reconfigure the wgpu surface for the affected display

**Checkpoint**: Multi-monitor shader wallpaper works with per-display resolution and DPI awareness.

---

## Phase 6: User Story 4 — Shader Compilation Error Reporting (Priority: P3)

**Goal**: Users see clear, actionable error messages when a shader fails to compile.

**Independent Test**: Run `cargo run -- broken.shader` (with a syntax error) and verify the error message on stderr includes the error type and line number in the user's shader file.

### Implementation for User Story 4

- [x] T023 [US4] Implement shader compilation error handling in `src/renderer/shader.rs` — catch `wgpu::Error` and naga validation errors from `device.create_shader_module()`, extract error messages and source spans
- [x] T024 [US4] Implement source-line mapping in `src/renderer/shader.rs` — subtract `wrapper_line_offset` from naga-reported line numbers to map errors back to the user's original `.shader` file lines; format errors to stderr in the pattern: `Error: shader compilation failed\n  --> {filename}:{line}:{col}\n   |\n{line} | {source_line}\n   | {error_description}`
- [x] T025 [US4] Add exit code 4 for shader compilation failure in `src/main.rs` — when `NativeRenderer::new()` returns a shader compilation error, print the formatted error to stderr and exit with code 4

**Checkpoint**: Shader compilation errors are human-readable with correct source locations.

---

## Phase 7: User Story 5 — Graceful Lifecycle Management (Priority: P3)

**Goal**: The native renderer integrates with existing IPC stop/stopall and Ctrl+C handling for clean shutdown.

**Independent Test**: Start a shader wallpaper, run `cargo run -- --stop 0`, and verify the process exits cleanly. Also test Ctrl+C.

### Implementation for User Story 5

- [x] T026 [US5] Integrate shutdown flag into the native render loop in `src/platform/windows/wallpaper.rs` — check the existing `Arc<AtomicBool>` shutdown flag each frame; when set, exit the event loop and call `renderer.shutdown()`
- [x] T027 [US5] Ensure IPC server thread starts alongside native renderer in `src/platform/windows/wallpaper.rs` — reuse the existing IPC server spawn logic so that `--stop` and `--stopall` commands set the shutdown flag
- [x] T028 [US5] Ensure Ctrl+C handler integration — verify the existing `ctrlc` handler sets the same shutdown flag that the native render loop checks; on shutdown, drop `NativeRenderer` (which releases GPU resources), detach window from WorkerW, and clean up instance tracking files
- [x] T029 [US5] Add GPU adapter detection with exit code 5 in `src/renderer/pipeline.rs` — when `instance.request_adapter()` returns `None`, return an error with message "no compatible GPU adapter found — requires Vulkan 1.0, Direct3D 12, Metal, or OpenGL 3.3+"; handle this in `src/main.rs` to print to stderr and exit with code 5
- [x] T030 [US5] Handle GPU device lost in `src/renderer/mod.rs` — when `surface.get_current_texture()` returns `SurfaceError::Lost` or `SurfaceError::OutOfMemory`, log the error to stderr and set the shutdown flag to trigger clean exit

**Checkpoint**: Full lifecycle integration — IPC, Ctrl+C, and GPU error conditions all handled cleanly.

---

## Phase 8: Cross-Platform Support (Priority: P2-P3)

**Purpose**: Extend native rendering to Linux and macOS

- [x] T031 [P] Modify `src/platform/linux/wallpaper.rs` — add `NativeGpu` rendering path: create GTK/tao window with gtk-layer-shell on BOTTOM layer, extract X11 window ID or Wayland surface handle, create wgpu surface via `SurfaceTargetUnsafe::RawHandle`, run native render loop
- [x] T032 [P] Modify `src/platform/macos/wallpaper.rs` — add `NativeGpu` rendering path: create tao NSWindow at desktop level, extract NSView handle, create wgpu surface via `SurfaceTargetUnsafe::RawHandle`, run native render loop
- [x] T033 Run all 36 sample shaders in `shaders/` directory on Windows — 30/36 pass natively; 3 fail due to iChannel0 texture sampling (not yet supported, deferred), 2 fail due to naga GLSL parsing limits (array constructors, nested assignments), 1 triggers naga internal panic
- [x] T034 [P] Added `const in` → `in` GLSL preprocessing in `src/renderer/shader.rs` to fix pinkvoid.shader; iChannel support deferred to a dedicated design pass

**Checkpoint**: Native rendering works on all three platforms with full shader compatibility.

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Final quality improvements across all stories

- [ ] T035 Verify process memory usage under 80 MB for single-display shader rendering (SC-001) using system task manager
- [ ] T036 Verify shader wallpaper startup time under 2 seconds (SC-003) by timing from command execution to first visible frame
- [ ] T037 Verify CPU usage under 5% in steady state (SC-004) while rendering a standard-complexity shader
- [ ] T038 Run `cargo clippy` and fix all warnings introduced by the new `src/renderer/` module
- [ ] T039 Run `cargo test` and verify all existing tests still pass
- [ ] T040 Clean up `src/shader.rs` — remove or gate the HTML bundle generation code (`build_shader_html`, `create_shader_bundle`, `cleanup_shader_bundle`) behind a feature flag or remove entirely if WebView fallback for shaders is not needed

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Phase 2 — core MVP, should be completed first
- **US2 (Phase 4)**: Depends on Phase 2 — can run in parallel with US3, US4, US5
- **US3 (Phase 5)**: Depends on Phase 2 — can run in parallel with US2, US4, US5
- **US4 (Phase 6)**: Depends on Phase 2 — can run in parallel with US2, US3, US5
- **US5 (Phase 7)**: Depends on Phase 2 — can run in parallel with US2, US3, US4
- **Cross-Platform (Phase 8)**: Depends on at least US1 (Phase 3) being complete
- **Polish (Phase 9)**: Depends on all desired user stories being complete

### User Story Dependencies

- **US1 (P1)**: Depends only on Foundational — no dependencies on other stories
- **US2 (P2)**: Depends only on Foundational — independent of other stories (scale/time-scale are parameter-level concerns)
- **US3 (P2)**: Depends only on Foundational — multi-monitor is an extension of the per-display rendering in US1 but can be developed independently
- **US4 (P3)**: Depends only on Foundational — error handling is orthogonal to rendering
- **US5 (P3)**: Depends only on Foundational — lifecycle management is orthogonal to rendering

### Within Each User Story

- Core data structures before pipeline setup
- Pipeline setup before render loop integration
- Render loop before platform integration
- Platform integration before end-to-end validation

### Parallel Opportunities

- T005, T006, T007, T008 can all run in parallel (different files, no dependencies)
- US2, US3, US4, US5 can all run in parallel after Foundational completes
- T031 (Linux) and T032 (macOS) can run in parallel
- T033 (shader testing) and T034 (fixes) are sequential

---

## Parallel Example: Foundational Phase

```text
# Launch all foundational tasks in parallel (different files):
Task T005: "Implement ShaderToyUniforms struct in src/renderer/uniforms.rs"
Task T006: "Implement uniform update function in src/renderer/uniforms.rs"
Task T007: "Implement GLSL wrapper template in src/renderer/shader.rs"
Task T008: "Implement fullscreen triangle vertex shader in src/renderer/shader.rs"

# Then sequentially (dependencies):
Task T009: "Create wgpu pipeline in src/renderer/pipeline.rs" (depends on T005, T007, T008)
Task T010: "Implement NativeRenderer API in src/renderer/mod.rs" (depends on T009, T006)
```

## Parallel Example: User Stories After Foundational

```text
# After Foundational completes, these can run in parallel:
Stream A (US1): T011 → T012 → T013 → T014 → T015 → T016
Stream B (US2): T017 → T018 → T019
Stream C (US4): T023 → T024 → T025
Stream D (US5): T026 → T027 → T028 → T029 → T030
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T004)
2. Complete Phase 2: Foundational (T005-T010)
3. Complete Phase 3: User Story 1 (T011-T016)
4. **STOP and VALIDATE**: Run `plasma.shader` as wallpaper, verify visual output, check memory < 80 MB
5. This is the minimum viable feature — a single shader renders natively on Windows

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add US1 → Test: single shader renders as wallpaper (MVP!)
3. Add US2 → Test: `--scale 0.5 --time-scale 2.0` works correctly
4. Add US3 → Test: multi-monitor renders independently
5. Add US4 → Test: broken shader shows clear error
6. Add US5 → Test: `--stop` and Ctrl+C work cleanly
7. Add Cross-Platform → Test: Linux and macOS rendering
8. Polish → Verify all success criteria metrics

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable after Foundational phase
- No test tasks generated (not explicitly requested in spec)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- The HTML bundle generation in `src/shader.rs` is retained until US1 is proven stable, then cleaned up in T040
