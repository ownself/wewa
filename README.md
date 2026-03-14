# WebWallpaper

Display web content as desktop wallpaper on Windows.

A Rust CLI tool that renders web pages (URLs or local HTML files) as fullscreen, click-through desktop wallpaper with multi-monitor support.

## Features

- Display any URL or local HTML file as desktop wallpaper
- Fullscreen, frameless windows positioned behind all other windows
- Complete click-through (mouse/keyboard passes through to desktop)
- Multi-monitor support with per-display targeting
- Local HTTP server for serving local HTML files
- IPC-based stop commands for remote control
- Graceful Ctrl+C handling

## Requirements

- **Windows 10** (April 2018 Update or later) or **Windows 11**
- **WebView2 Runtime** (usually pre-installed, or [download here](https://developer.microsoft.com/microsoft-edge/webview2/))

## Installation

### Build from Source

```bash
# Clone the repository
git clone https://github.com/user/webwallpaper.git
cd webwallpaper

# Build release binary
cargo build --release

# Binary is at target/release/webwallpaper.exe
```

## Usage

### Display a URL as Wallpaper

```bash
# Apply to all monitors
webwallpaper https://example.com/wallpaper.html

# Apply to specific monitor (0-based index)
webwallpaper https://example.com --display 0
```

### Display a Local HTML File

```bash
# Single file
webwallpaper ./my-wallpaper/index.html

# Directory with index.html
webwallpaper ./my-wallpaper/

# Use custom port for HTTP server
webwallpaper ./wallpaper.html --port 9000
```

### Stop Running Wallpapers

```bash
# Stop wallpaper on display 0
webwallpaper --stop 0

# Stop all running wallpapers
webwallpaper --stopall
```

### Verbose Mode

```bash
# Show detailed output
webwallpaper --verbose https://example.com
```

## Command Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--display <N>` | `-d` | Target specific display (0-based index) |
| `--stop <N>` | | Stop wallpaper on display N |
| `--stopall` | | Stop all running wallpapers |
| `--port <PORT>` | `-p` | HTTP server port for local files (default: 8080) |
| `--verbose` | `-v` | Enable verbose output |
| `--help` | `-h` | Show help message |
| `--version` | `-V` | Show version |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Display not found |
| 3 | No running instance to stop |
| 4 | WebView2 runtime not available |
| 5 | Server startup failed (port in use) |

## Creating a Web Wallpaper

Create an `index.html` file with your wallpaper design:

```html
<!DOCTYPE html>
<html>
<head>
  <style>
    body {
      margin: 0;
      overflow: hidden;
      background: linear-gradient(45deg, #1a1a2e, #16213e);
    }
    .container {
      width: 100vw;
      height: 100vh;
      display: flex;
      justify-content: center;
      align-items: center;
    }
    .clock {
      font-family: 'Segoe UI', sans-serif;
      font-size: 8vw;
      color: rgba(255, 255, 255, 0.8);
    }
  </style>
</head>
<body>
  <div class="container">
    <div class="clock" id="clock"></div>
  </div>
  <script>
    function updateClock() {
      const now = new Date();
      document.getElementById('clock').textContent =
        now.toLocaleTimeString('en-US', { hour12: false });
    }
    setInterval(updateClock, 1000);
    updateClock();
  </script>
</body>
</html>
```

Then run:

```bash
webwallpaper ./index.html
```

## Troubleshooting

### WebView2 Runtime Not Available

Install WebView2 from: https://developer.microsoft.com/microsoft-edge/webview2/

### Port Already in Use

Use a different port:

```bash
webwallpaper ./wallpaper.html --port 9000
```

### Finding Display Numbers

Use verbose mode to see available displays:

```bash
webwallpaper --verbose https://example.com
# [INFO] Found 2 display(s)
# [INFO] Display 0: 1920x1080 at (0, 0) [Primary]
# [INFO] Display 1: 1920x1080 at (1920, 0)
```

## Architecture

```
src/
├── main.rs           # Entry point and CLI dispatch
├── cli.rs            # Argument parsing (clap)
├── config.rs         # Configuration and instance tracking
├── server.rs         # Local HTTP server (tiny_http)
├── ipc.rs            # Inter-process communication (named pipes)
├── display.rs        # Monitor enumeration trait
├── wallpaper.rs      # Wallpaper window trait
└── platform/
    └── windows/      # Windows-specific implementation
        ├── display.rs    # EnumDisplayMonitors
        └── wallpaper.rs  # WebView2 + Win32 API
```

## License

MIT
