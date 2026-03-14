# Quickstart: Rust WebWallpaper CLI

**Feature Branch**: `001-rust-webwallpaper-cli`
**Date**: 2026-03-14

## Prerequisites

### Windows 10/11
- **WebView2 Runtime**: Required for rendering web content
  - Usually pre-installed on Windows 10 (April 2018 Update+) and Windows 11
  - Manual install: https://developer.microsoft.com/microsoft-edge/webview2/
- **Rust toolchain**: rustc 1.75+ with MSVC target

### Development Setup

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Ensure MSVC target is available (Windows)
rustup target add x86_64-pc-windows-msvc
```

## Building

```bash
# Clone repository
git clone <repository-url>
cd webwallpaper

# Build release binary
cargo build --release

# Binary location
# Windows: target/release/webwallpaper.exe
```

## Quick Usage

### Display a Web Page as Wallpaper

```bash
# From a URL
./webwallpaper https://example.com/animated-wallpaper.html

# From a local file
./webwallpaper ./my-wallpaper/index.html
```

### Target Specific Monitor

```bash
# Display on primary monitor only
./webwallpaper https://example.com --display 0

# Display on secondary monitor
./webwallpaper https://example.com --display 1
```

### Stop Wallpapers

```bash
# Stop wallpaper on display 0
./webwallpaper --stop 0

# Stop all running wallpapers
./webwallpaper --stopall
```

## Creating a Web Wallpaper

Create an `index.html` file:

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
./webwallpaper ./index.html
```

## Project Structure

```
webwallpaper/
├── src/
│   ├── main.rs           # Entry point
│   ├── cli.rs            # Argument parsing
│   ├── config.rs         # Configuration
│   ├── server.rs         # HTTP server
│   ├── ipc.rs            # Inter-process communication
│   ├── display.rs        # Monitor enumeration
│   ├── wallpaper.rs      # Wallpaper window trait
│   └── platform/
│       └── windows/      # Windows-specific implementation
├── tests/
├── Cargo.toml
└── README.md
```

## Development Workflow

### Run in Debug Mode

```bash
cargo run -- https://example.com
```

### Run Tests

```bash
cargo test
```

### Check Formatting

```bash
cargo fmt --check
cargo clippy
```

## Troubleshooting

### "WebView2 runtime not available"

Install WebView2 from:
https://developer.microsoft.com/microsoft-edge/webview2/

### Wallpaper Not Appearing Behind Desktop Icons

The application positions itself at `HWND_BOTTOM`. On some Windows configurations, you may need to click on the desktop once after starting the wallpaper.

### Port Already in Use

```bash
# Use a different port
./webwallpaper ./wallpaper.html --port 9000
```

### Finding Display Numbers

Display indices are 0-based. Use verbose mode to see available displays:

```bash
./webwallpaper --verbose https://example.com
# [INFO] Found 2 display(s)
# [INFO] Display 0: 1920x1080 at (0, 0) [Primary]
# [INFO] Display 1: 1920x1080 at (1920, 0)
```

## Next Steps

- Review [spec.md](./spec.md) for full requirements
- Review [plan.md](./plan.md) for implementation details
- Review [contracts/cli.md](./contracts/cli.md) for CLI interface specification
