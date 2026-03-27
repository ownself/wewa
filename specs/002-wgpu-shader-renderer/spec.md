# Feature Specification: Native wgpu Shader Renderer

**Feature Branch**: `002-wgpu-shader-renderer`
**Created**: 2026-03-27
**Status**: Draft
**Input**: User description: "Implement native wgpu renderer for .shader mode to replace WebView-based WebGL rendering with direct GPU access, achieving significantly lower memory usage, reduced frame latency, and better power efficiency while maintaining full ShaderToy GLSL compatibility"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Run ShaderToy Shaders Natively (Priority: P1)

As a user, I want to run `.shader` files (ShaderToy-style GLSL fragment shaders) as desktop wallpaper using native GPU rendering instead of a browser engine, so that my system uses significantly less memory and CPU while displaying the same visual output.

The user experience remains identical to the current workflow: the user provides a `.shader` file path via CLI, and the program renders it as a live wallpaper on the desktop. The difference is entirely internal — rendering happens through direct GPU access rather than through a WebView + WebGL pipeline.

**Why this priority**: This is the core value proposition of the feature. Without native shader rendering, none of the performance benefits are realized. A single shader rendering correctly on the desktop via native GPU proves the entire architecture works.

**Independent Test**: Can be fully tested by running `webwallpaper plasma.shader` and verifying the shader displays correctly as wallpaper with visually identical output to the current WebView approach, while using substantially less memory.

**Acceptance Scenarios**:

1. **Given** a valid `.shader` file containing a `mainImage()` function, **When** the user runs the program with that file, **Then** the shader renders as a fullscreen animated wallpaper on the desktop using native GPU rendering.
2. **Given** a shader that uses standard ShaderToy uniforms (`iResolution`, `iTime`, `iTimeDelta`, `iFrame`, `iFrameRate`, `iMouse`, `iDate`), **When** the shader is rendered, **Then** all uniforms are correctly populated and updated each frame.
3. **Given** the shader is running as wallpaper, **When** the user observes system resource usage, **Then** the process memory footprint is under 80 MB (compared to the current 200-500 MB with WebView).

---

### User Story 2 - Resolution Scaling and Time Control (Priority: P2)

As a user, I want the existing `--scale` and `--time-scale` CLI options to work with the native renderer, so that I can control rendering resolution and animation speed just as I can today.

**Why this priority**: These are existing user-facing features that must continue to work. Scale directly affects performance (lower resolution = less GPU work), making it essential for users on low-power systems.

**Independent Test**: Can be tested by running a shader with `--scale 0.5 --time-scale 2.0` and verifying the output renders at half resolution and animates at double speed.

**Acceptance Scenarios**:

1. **Given** the user specifies `--scale 0.5`, **When** the shader renders, **Then** the rendering resolution is 50% of the display resolution, and the output is upscaled to fill the screen.
2. **Given** the user specifies `--time-scale 0.0`, **When** the shader renders, **Then** the animation freezes at time zero.
3. **Given** the user specifies `--time-scale 3.0`, **When** the shader renders, **Then** the animation plays at triple speed.

---

### User Story 3 - Multi-Monitor Support (Priority: P2)

As a user with multiple monitors, I want each display to independently render the shader wallpaper using native rendering, just as the current WebView implementation does.

**Why this priority**: Multi-monitor is a commonly expected feature for wallpaper applications. The current implementation already supports it, so the native renderer must maintain parity.

**Independent Test**: Can be tested on a multi-monitor setup by verifying that each display shows the shader wallpaper with correct resolution and positioning.

**Acceptance Scenarios**:

1. **Given** a system with multiple displays, **When** the user runs a shader as wallpaper, **Then** each display renders the shader independently at its own native resolution.
2. **Given** displays with different resolutions or DPI settings, **When** the shader renders, **Then** each display correctly uses its own resolution for `iResolution` and renders accordingly.

---

### User Story 4 - Shader Compilation Error Reporting (Priority: P3)

As a user, I want to see clear error messages when a shader fails to compile, so that I can diagnose and fix issues in my shader code.

**Why this priority**: Error reporting is important for usability but not critical for the core rendering feature. Users authoring or modifying shaders need feedback when compilation fails.

**Independent Test**: Can be tested by running a shader with a syntax error and verifying the error message clearly indicates the problem and location.

**Acceptance Scenarios**:

1. **Given** a `.shader` file with a GLSL syntax error, **When** the user runs the program, **Then** the system displays a human-readable error message indicating the error type and approximate location in the shader source.
2. **Given** a `.shader` file that uses unsupported GLSL features, **When** the shader fails to compile, **Then** the error message indicates the specific unsupported feature.

---

### User Story 5 - Graceful Lifecycle Management (Priority: P3)

As a user, I want the native renderer to integrate with the existing IPC stop/stopall commands and Ctrl+C handling, so that I can control the wallpaper lifecycle as I do today.

**Why this priority**: Lifecycle management is essential for a production-quality application, but is secondary to getting the rendering working correctly.

**Independent Test**: Can be tested by starting a shader wallpaper, then running `webwallpaper --stop` or pressing Ctrl+C, and verifying the process exits cleanly.

**Acceptance Scenarios**:

1. **Given** a shader wallpaper is running, **When** the user sends `--stop` via IPC, **Then** the renderer shuts down cleanly and the desktop wallpaper is restored.
2. **Given** a shader wallpaper is running, **When** the user presses Ctrl+C, **Then** the renderer shuts down cleanly.

---

### Edge Cases

- What happens when the GPU driver does not support any compatible backend (no Vulkan, D3D12, Metal, or OpenGL)?
  - The system should display a clear error message indicating the minimum GPU requirements and exit gracefully.
- What happens when the shader compiles successfully but produces a runtime GPU error (e.g., infinite loop, out-of-memory)?
  - The system should detect the GPU device lost condition and exit with an error message rather than hanging.
- What happens when the user's system has multiple GPUs (integrated + discrete)?
  - The system should prefer the low-power (integrated) GPU by default for power efficiency, with the option to override this behavior.
- What happens when the display resolution changes while the shader is running (e.g., monitor connected/disconnected)?
  - The renderer should detect the change and resize the rendering surface accordingly.
- What happens when the shader file is modified while the wallpaper is running?
  - No hot-reload is required; the user must restart the program to apply changes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST render `.shader` files (ShaderToy-style GLSL fragment shaders) using native GPU access without requiring a browser engine or WebView.
- **FR-002**: System MUST support all standard ShaderToy uniforms: `iResolution`, `iTime`, `iTimeDelta`, `iFrame`, `iFrameRate`, `iMouse`, `iDate`.
- **FR-003**: System MUST accept GLSL shader source code and translate it to the appropriate GPU backend format internally.
- **FR-004**: System MUST render the shader output as a desktop wallpaper using the platform-specific wallpaper integration technique (WorkerW on Windows, layer-shell on Linux, NSWindow desktop level on macOS).
- **FR-005**: System MUST support the `--scale` parameter (range 0.1-2.0) to control rendering resolution relative to display resolution.
- **FR-006**: System MUST support the `--time-scale` parameter (range 0.0-100.0) to control animation speed.
- **FR-007**: System MUST render on each connected display independently with correct per-display resolution.
- **FR-008**: System MUST report shader compilation errors with human-readable messages that include the error type and approximate source location.
- **FR-009**: System MUST integrate with the existing IPC control system (`--stop`, `--stopall`) and Ctrl+C signal handling for clean shutdown.
- **FR-010**: System MUST prefer the low-power GPU on multi-GPU systems by default.
- **FR-011**: System MUST use vertical sync to avoid rendering frames faster than the display refresh rate.
- **FR-012**: System MUST maintain backward compatibility — all existing `.shader` files in the repository (38 sample shaders) should render correctly with the native renderer.
- **FR-013**: System MUST detect when no compatible GPU backend is available and display a clear error message with minimum requirements.
- **FR-014**: The existing URL/web page wallpaper mode MUST continue to use WebView and remain unaffected by this change.

### Key Entities

- **Shader Source**: The user-provided `.shader` file containing a ShaderToy-compatible GLSL fragment shader with a `mainImage(out vec4 fragColor, in vec2 fragCoord)` entry point.
- **Uniform Buffer**: A per-frame data structure containing all ShaderToy-compatible uniform values (`iResolution`, `iTime`, etc.) passed to the GPU.
- **Render Surface**: A platform-specific GPU rendering target attached to the desktop wallpaper window, one per display.
- **GPU Backend**: The automatically selected graphics API (Vulkan, D3D12, Metal, or OpenGL fallback) used to communicate with the GPU hardware.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Shader wallpaper process memory usage is under 80 MB for a single-display setup (compared to the current 200-500 MB with WebView).
- **SC-002**: All 38 existing sample shaders render visually identical output compared to the current WebView-based rendering.
- **SC-003**: Shader wallpaper startup time (from command execution to first frame visible) is under 2 seconds.
- **SC-004**: The native renderer maintains a stable frame rate at the display's refresh rate (typically 60fps) for standard-complexity shaders on mid-range hardware, without exceeding 5% CPU usage in steady state.
- **SC-005**: Users can use the exact same CLI commands and `.shader` file format as today — no changes to the user-facing interface.
- **SC-006**: The `--scale` and `--time-scale` options produce equivalent visual results to the current implementation.

## Assumptions

- Users have GPU hardware and drivers that support at least one of: Vulkan 1.0, Direct3D 12, Metal, or OpenGL 3.3+. Systems without any GPU support are out of scope.
- The `.shader` file format remains unchanged — files contain a ShaderToy-compatible `mainImage()` GLSL function.
- ShaderToy multipass rendering (Buffer A/B/C/D), texture/cubemap channel inputs, audio inputs, and keyboard inputs are out of scope for this initial implementation. Only single-pass fragment shaders with time/resolution/mouse uniforms are supported.
- Mouse tracking for `iMouse` uniform may have limited functionality when the wallpaper window is behind desktop icons, depending on platform constraints. Basic position tracking is expected; click tracking is best-effort.
- Hot-reload of shader files is not required; users restart the program to apply shader changes.

## Scope Boundaries

**In Scope**:
- Native GPU rendering of single-pass ShaderToy GLSL fragment shaders
- All standard ShaderToy uniforms (iResolution, iTime, iTimeDelta, iFrame, iFrameRate, iMouse, iDate)
- Cross-platform support (Windows, Linux, macOS)
- Integration with existing wallpaper attachment, IPC, and lifecycle management
- Resolution scaling and time scaling
- Multi-monitor support
- Shader compilation error reporting
- Automatic GPU backend selection with low-power preference

**Out of Scope**:
- ShaderToy multipass rendering (Buffer A/B/C/D)
- ShaderToy texture/cubemap/audio/video channel inputs
- ShaderToy keyboard input
- Hot-reload of shader files
- Any changes to the URL/web page wallpaper mode (remains WebView-based)
- Custom WGSL shader support (only GLSL via ShaderToy format)
- GPU compute shader workloads
