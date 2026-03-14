# Data Model: Rust WebWallpaper CLI

**Feature Branch**: `001-rust-webwallpaper-cli`
**Date**: 2026-03-14

## Entities

### 1. WallpaperInstance

Represents a running wallpaper process tied to a specific display.

| Field | Type | Description |
|-------|------|-------------|
| `display_index` | `u32` | 0-based monitor index |
| `pid` | `u32` | Operating system process ID |
| `url` | `String` | URL or file:// path being displayed |
| `server_port` | `Option<u16>` | Local HTTP server port (if serving local files) |
| `started_at` | `DateTime` | Instance start timestamp |

**Persistence**: Stored as JSON in `%TEMP%\webwallpaper\display_{N}.json`

**Example**:
```json
{
  "display_index": 0,
  "pid": 12345,
  "url": "http://localhost:8080/index.html",
  "server_port": 8080,
  "started_at": "2026-03-14T10:30:00Z"
}
```

---

### 2. Display

Represents a physical or virtual monitor.

| Field | Type | Description |
|-------|------|-------------|
| `index` | `u32` | 0-based display index |
| `x` | `i32` | Left position in virtual screen coordinates |
| `y` | `i32` | Top position in virtual screen coordinates |
| `width` | `u32` | Display width in pixels |
| `height` | `u32` | Display height in pixels |
| `work_x` | `i32` | Work area left (excludes taskbar) |
| `work_y` | `i32` | Work area top |
| `work_width` | `u32` | Work area width |
| `work_height` | `u32` | Work area height |
| `is_primary` | `bool` | Whether this is the primary display |

**Note**: Coordinates can be negative for monitors positioned left of or above the primary monitor.

---

### 3. CliArgs

Parsed command-line arguments.

| Field | Type | Description |
|-------|------|-------------|
| `url_or_path` | `Option<String>` | URL or local file path to display |
| `display` | `Option<u32>` | Target display index (None = all displays) |
| `stop` | `Option<u32>` | Stop instance on specific display |
| `stop_all` | `bool` | Stop all running instances |
| `port` | `u16` | HTTP server port (default: 8080) |
| `verbose` | `bool` | Enable verbose logging |

**Mutual Exclusivity**:
- `url_or_path` conflicts with `stop` and `stop_all`
- `stop` conflicts with `stop_all`

---

### 4. IpcCommand

Commands sent between CLI invocations and running instances.

| Variant | Payload | Description |
|---------|---------|-------------|
| `Stop` | `display_index: u32` | Stop wallpaper on specific display |
| `StopAll` | None | Stop all wallpaper instances |
| `Status` | None | Request status of all instances |
| `Ping` | None | Health check |

**Wire Format**: Text-based for simplicity
- `STOP:0` - Stop display 0
- `STOP:ALL` - Stop all
- `STATUS` - Request status
- `PING` - Health check

---

### 5. Config

Application configuration (future expansion).

| Field | Type | Description |
|-------|------|-------------|
| `default_port` | `u16` | Default HTTP server port |
| `instance_dir` | `PathBuf` | Directory for PID/instance files |
| `pipe_name` | `String` | IPC named pipe name |

**Default Values**:
```rust
Config {
    default_port: 8080,
    instance_dir: env::temp_dir().join("webwallpaper"),
    pipe_name: "webwallpaper_control".to_string(),
}
```

---

## State Transitions

### WallpaperInstance Lifecycle

```
┌─────────────┐
│   Created   │  CLI invoked with URL/path
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Starting   │  Window created, server starting
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Running   │  Wallpaper visible, instance file written
└──────┬──────┘
       │ (--stop or --stopall command)
       ▼
┌─────────────┐
│  Stopping   │  IPC command received, cleanup starting
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Stopped   │  Instance file deleted, process exits
└─────────────┘
```

---

## File Layout

```
%TEMP%\webwallpaper\
├── display_0.json      # Instance info for display 0
├── display_1.json      # Instance info for display 1
└── ...
```

Each file contains a serialized `WallpaperInstance`. Files are:
- Created when wallpaper starts on a display
- Deleted when wallpaper stops
- Used by `--stop` to find the correct process to terminate
