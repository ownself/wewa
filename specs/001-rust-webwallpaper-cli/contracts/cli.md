# CLI Contract: webwallpaper

**Feature Branch**: `001-rust-webwallpaper-cli`
**Date**: 2026-03-14

## Command Syntax

```
webwallpaper [OPTIONS] [URL_OR_PATH]
webwallpaper --stop <DISPLAY>
webwallpaper --stopall
```

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `URL_OR_PATH` | String | No* | URL (http/https) or local file path to display as wallpaper |

*Required unless using `--stop` or `--stopall`

## Options

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--display` | `-d` | Integer | None (all) | Target display index (0-based) |
| `--stop` | None | Integer | None | Stop wallpaper on specified display |
| `--stopall` | None | Flag | false | Stop all running wallpaper instances |
| `--port` | `-p` | Integer | 8080 | HTTP server port for local files |
| `--verbose` | `-v` | Flag | false | Enable verbose output |
| `--help` | `-h` | Flag | false | Show help message |
| `--version` | `-V` | Flag | false | Show version |

## Usage Examples

### Start Wallpaper

```bash
# Display URL as wallpaper on all monitors
webwallpaper https://example.com/wallpaper.html

# Display local HTML file
webwallpaper ./wallpaper/index.html

# Display on specific monitor (0 = primary)
webwallpaper https://example.com --display 0

# Display on secondary monitor
webwallpaper ./wallpaper/index.html --display 1

# Use custom HTTP port for local files
webwallpaper ./wallpaper/index.html --port 9000
```

### Stop Wallpaper

```bash
# Stop wallpaper on display 0
webwallpaper --stop 0

# Stop wallpaper on display 1
webwallpaper --stop 1

# Stop all running wallpapers
webwallpaper --stopall
```

### Information

```bash
# Show help
webwallpaper --help

# Show version
webwallpaper --version
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (invalid arguments, missing file) |
| 2 | Display not found |
| 3 | No running instance to stop |
| 4 | WebView2 runtime not available |
| 5 | Server startup failed (port in use) |

## Output Format

### Standard Output (stdout)

```
# On successful start
Started wallpaper on display 0: https://example.com/wallpaper.html

# On successful stop
Stopped wallpaper on display 0

# With --stopall
Stopped 2 wallpaper instance(s)

# With --verbose
[INFO] Enumerating displays...
[INFO] Found 2 display(s)
[INFO] Display 0: 1920x1080 at (0, 0)
[INFO] Display 1: 1920x1080 at (1920, 0)
[INFO] Starting HTTP server on port 8080...
[INFO] Creating webview window...
[INFO] Applying wallpaper window styles...
[INFO] Started wallpaper on display 0
```

### Standard Error (stderr)

```
# Invalid display
error: Display 99 does not exist (available: 0, 1)

# File not found
error: File not found: ./nonexistent.html

# No instance to stop
error: No wallpaper running on display 0

# WebView2 missing
error: WebView2 runtime not available
hint: Install from https://developer.microsoft.com/microsoft-edge/webview2/

# Port in use
error: Port 8080 is already in use
hint: Use --port to specify a different port
```

## Mutual Exclusivity

The following options cannot be used together:

| Conflict | Error Message |
|----------|---------------|
| `URL_OR_PATH` + `--stop` | Cannot start wallpaper and stop at the same time |
| `URL_OR_PATH` + `--stopall` | Cannot start wallpaper and stop at the same time |
| `--stop` + `--stopall` | Cannot use --stop and --stopall together |
| `--display` + `--stopall` | --display is ignored with --stopall |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `WEBWALLPAPER_PORT` | 8080 | Default HTTP server port |
| `WEBWALLPAPER_VERBOSE` | 0 | Set to 1 for verbose output |

## Signal Handling

| Signal | Behavior |
|--------|----------|
| SIGINT (Ctrl+C) | Graceful shutdown, cleanup instance file |
| SIGTERM | Graceful shutdown, cleanup instance file |

## IPC Protocol

Running instances listen on a named pipe for control commands:

**Pipe Name**: `\\.\pipe\webwallpaper_control` (Windows)

**Commands**:
| Command | Response | Description |
|---------|----------|-------------|
| `STOP:0` | `OK` or `ERR:msg` | Stop display 0 |
| `STOP:ALL` | `OK:N` (N=count) | Stop all |
| `PING` | `PONG` | Health check |
