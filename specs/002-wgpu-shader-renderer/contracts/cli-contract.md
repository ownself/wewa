# CLI Contract: Native wgpu Shader Renderer

**Feature**: 002-wgpu-shader-renderer
**Date**: 2026-03-27

## Contract: No CLI Changes

The native wgpu renderer introduces **zero changes** to the user-facing CLI interface. All existing commands, arguments, and behaviors remain identical.

## Shader Mode Invocation

```
webwallpaper <shader-path> [options]
```

### Arguments (unchanged)

| Argument | Type | Default | Range | Description |
|----------|------|---------|-------|-------------|
| `<input>` | string | required | — | Path to `.shader` file (or URL for web mode) |
| `--scale` | f32 | 1.0 | 0.1–2.0 | Render resolution scale factor |
| `--time-scale` | f32 | 1.0 | 0.0–100.0 | Animation time scale factor |
| `--display` | usize | all | 0–N | Target display index |
| `--stop` | usize | — | 0–N | Stop wallpaper on display N |
| `--stopall` | flag | — | — | Stop all wallpaper instances |
| `--verbose` | flag | — | — | Enable verbose logging |

### Rendering Mode Selection (internal, transparent to user)

| Input | Detected As | Renderer |
|-------|-------------|----------|
| `*.shader` file | Shader file | **wgpu native** (new) |
| URL (http/https) | Web page | WebView (existing) |
| Local HTML file | Web page | WebView (existing) |
| ShaderToy URL | Web page (transformed) | WebView (existing) |

## Exit Codes (unchanged)

| Code | Meaning |
|------|---------|
| 0 | Success / clean shutdown |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | File not found |
| 4 | Shader compilation error (new, previously unreachable for native mode) |
| 5 | No compatible GPU (new) |

## Error Output Contract

### Shader Compilation Error (stderr)

```
Error: shader compilation failed
  --> myshader.shader:12:5
   |
12 |     vec3 color = undeclaredVar;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^ unknown identifier 'undeclaredVar'
```

- Errors are written to stderr
- Source locations reference the user's `.shader` file (line numbers adjusted for wrapper offset)
- Exit code: 4

### No Compatible GPU (stderr)

```
Error: no compatible GPU adapter found
  Requires one of: Vulkan 1.0, Direct3D 12, Metal, or OpenGL 3.3+
  Please update your GPU drivers and try again
```

- Exit code: 5

## IPC Protocol (unchanged)

The IPC control protocol remains identical:

| Command | Response | Behavior |
|---------|----------|----------|
| `STOP:{display}` | `OK` | Stop rendering on specified display |
| `STOP:ALL` | `OK:{count}` | Stop all rendering instances |
| `PING` | `PONG` | Health check |
