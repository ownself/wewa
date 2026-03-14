# Research: Rust WebWallpaper CLI

**Feature Branch**: `001-rust-webwallpaper-cli`
**Date**: 2026-03-14

## 1. Webview Library Selection

### Decision: `wry` (by Tauri team)

### Rationale:
- **Active maintenance**: Backed by the Tauri project with large community
- **WebView2 support**: Uses Microsoft's modern Chromium-based webview on Windows
- **Cross-platform**: Same API works on Windows (WebView2), Linux (WebKitGTK), macOS (WKWebView)
- **Window management**: Uses `tao` crate (fork of winit) for native window handling

### Alternatives Considered:

| Library | Status | Windows Backend | Why Rejected |
|---------|--------|-----------------|--------------|
| `web-view` | Abandoned (~2020) | MSHTML/EdgeHTML | No WebView2, deprecated browser engine |
| Direct WebView2 | Manual | WebView2 | Too low-level, no cross-platform path |

### Key Features for This Project:
- `WindowBuilder::with_decorations(false)` - Frameless window
- `WindowBuilder::with_position()` - Multi-monitor positioning
- `with_cursor_hittest(false)` - Partial click-through support (needs Win32 augmentation)

---

## 2. Windows Wallpaper Behavior

### Decision: Direct Win32 API via `windows-rs` crate

### Rationale:
- **Official support**: Microsoft's official Rust bindings
- **Complete API coverage**: Access to all required functions
- **Active maintenance**: Regular updates matching Windows SDK

### Required Window Styles:

| Style | Purpose |
|-------|---------|
| `WS_POPUP` | Remove all window decorations |
| `WS_EX_TOOLWINDOW` | Prevent taskbar button |
| `WS_EX_NOACTIVATE` | Never receive focus |
| `WS_EX_TRANSPARENT` | Click-through (mouse passthrough) |
| `WS_EX_LAYERED` | Enable layered window features |

### Z-Order Strategy:
1. Use `SetWindowPos` with `HWND_BOTTOM` to place at bottom
2. Optionally position relative to `Progman` (Program Manager) window for stability
3. Remove `WS_EX_APPWINDOW` flag to prevent taskbar appearance

### Click-Through Implementation:
```
WS_EX_TRANSPARENT + WS_EX_LAYERED + SetLayeredWindowAttributes(alpha=252)
```
Alpha value <255 ensures mouse events pass through reliably.

---

## 3. CLI Argument Parsing

### Decision: `clap` (v4)

### Rationale:
- **Extensibility**: Better for future growth (subcommands, plugins)
- **Documentation**: Automatic help generation with rich formatting
- **Shell completions**: Built-in support for bash/zsh/fish/PowerShell
- **Ecosystem**: Most widely used, excellent documentation

### Alternatives Considered:

| Crate | Binary Size | Why Not Chosen |
|-------|-------------|----------------|
| `argh` | ~50KB | Simpler but less extensible |
| Manual | Minimal | Too much boilerplate |

### CLI Structure:
```
webwallpaper [URL_OR_PATH]           # Start wallpaper
webwallpaper --display 0 [URL]       # Specific monitor
webwallpaper --stop 0                # Stop on monitor 0
webwallpaper --stopall               # Stop all instances
webwallpaper --help                  # Help text
```

---

## 4. Inter-Process Communication (IPC)

### Decision: `interprocess` crate with named pipes

### Rationale:
- **Cross-platform abstraction**: Same API for Windows pipes and Unix sockets
- **Lightweight**: No runtime overhead like tokio
- **Simple API**: Straightforward listener/stream pattern

### Windows Implementation:
- Named pipe at `\\.\pipe\webwallpaper_control`
- Protocol: Simple text commands (`STOP:0`, `STOP:ALL`)
- Synchronous I/O (no async runtime needed)

### Instance Tracking:
- PID file in `%TEMP%\webwallpaper\` directory
- One file per display: `display_0.pid`, `display_1.pid`
- Contains process ID for graceful shutdown

---

## 5. Local HTTP Server

### Decision: `tiny_http`

### Rationale:
- **Minimal footprint**: ~100KB binary size impact
- **No async runtime**: Synchronous API, simple threading
- **Perfect fit**: Exactly what's needed for local file serving

### Alternatives Considered:

| Crate | Size Impact | Why Not Chosen |
|-------|-------------|----------------|
| `axum` | ~500KB-1MB | Requires tokio, overkill for local serving |
| `actix-web` | ~1-2MB | Too heavy for embedded use |

### Implementation:
- Serve files from local directory
- Bind to `127.0.0.1:8080` (configurable port)
- Add cache-control headers to prevent stale content
- Security: Validate paths to prevent directory traversal

---

## 6. Monitor Enumeration

### Decision: Windows `EnumDisplayMonitors` API via `windows-rs`

### Required Information:
- Monitor handle (`HMONITOR`)
- Full rectangle (`rcMonitor`) - entire display area
- Work area (`rcWork`) - excludes taskbar
- Primary monitor flag

### Implementation Pattern:
```rust
EnumDisplayMonitors(HDC(0), None, callback, LPARAM(user_data))
```

Callback receives `HMONITOR`, then call `GetMonitorInfoW` for details.

---

## 7. DPI Awareness

### Decision: Per-monitor DPI awareness via manifest + API fallback

### Implementation:
1. Application manifest declares DPI awareness (preferred)
2. Runtime fallback: `SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE)`

This ensures correct sizing on high-DPI displays and mixed-DPI multi-monitor setups.

---

## Summary: Technology Stack

| Component | Choice | Crate/API |
|-----------|--------|-----------|
| Webview | WebView2 | `wry` + `tao` |
| CLI parsing | Derive-based | `clap` v4 |
| Windows API | Official bindings | `windows` |
| IPC | Named pipes | `interprocess` |
| HTTP server | Embedded | `tiny_http` |
| Serialization | JSON (for config) | `serde_json` |

### Cargo.toml Dependencies:
```toml
[dependencies]
wry = "0.35"
tao = "0.24"
clap = { version = "4", features = ["derive"] }
interprocess = "1.2"
tiny_http = "0.12"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[target.'cfg(windows)'.dependencies]
windows = { version = "0.52", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_Graphics_Gdi",
    "Win32_UI_HiDpi"
]}
```
