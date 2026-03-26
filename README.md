# WebWallpaper

Display web content as desktop wallpaper on Windows, Linux and macOS.

A cross-platform Rust CLI tool that renders web pages (URLs or local HTML files) as fullscreen desktop wallpaper with multi-monitor support.

---

**[English](#english)** | **[中文](#中文)**

---

## English

### Features

- **Cross-Platform** — Windows, Linux (Wayland) and macOS
- **Multi-Monitor** — Apply to all displays or target specific ones
- **Local File Support** — Built-in HTTP server for local HTML/JS/CSS projects
- **ShaderToy Integration** — Automatically converts ShaderToy URLs to fullscreen embed format
- **Local `.shader` Support** — Wrap single-pass ShaderToy snippets into a fullscreen WebGL runtime
- **IPC Control** — Stop wallpapers remotely via named pipes (Windows) or Unix domain sockets (Linux/macOS)
- **Graceful Shutdown** — Ctrl+C handling with platform-specific cleanup

### Platform Details

| Platform | Technique | WebView Backend |
|----------|-----------|-----------------|
| Windows | WorkerW desktop embedding | WebView2 (Edge) |
| Linux | GTK layer-shell background surface | WebKitGTK |
| macOS | NSWindow desktop-level ordering | WKWebView |

### Requirements

- **Windows** — Windows 10 (April 2018+) or Windows 11; WebView2 Runtime (usually pre-installed)
- **Linux** — Wayland session with layer-shell support (e.g. Hyprland); GTK 3, WebKitGTK, gtk-layer-shell
- **macOS** — macOS 10.10+; no additional dependencies (WKWebView is a system framework)

### Installation

```bash
git clone https://github.com/user/webwallpaper.git
cd webwallpaper

# Linux dependencies (Debian/Ubuntu)
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libgtk-layer-shell-dev

cargo build --release
# Binary: target/release/webwallpaper(.exe)
```

### Usage

```bash
# URL as wallpaper (all monitors)
webwallpaper https://example.com/wallpaper.html

# Target a specific monitor (0-based index)
webwallpaper https://example.com --display 0

# ShaderToy (auto-converted to fullscreen embed)
webwallpaper "https://www.shadertoy.com/view/tlVGDt"

# Local ShaderToy-style shader with default full-resolution rendering
webwallpaper ./octagrams.shader

# Lower shader render scale for better performance
webwallpaper ./octagrams.shader --scale 0.5

# Slow down shader animation without changing shader source
webwallpaper ./octagrams.shader --time-scale 0.5

# Speed up shader animation for comparison or debugging
webwallpaper ./octagrams.shader --time-scale 2.0

# Local HTML file
webwallpaper ./my-wallpaper/index.html

# Directory containing index.html
webwallpaper ./my-wallpaper/

# Custom HTTP server port
webwallpaper ./wallpaper.html --port 9000

# Stop wallpaper on display 0
webwallpaper --stop 0

# Stop all wallpapers
webwallpaper --stopall

# Verbose output
webwallpaper --verbose https://example.com
```

### Command Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--display <N>` | `-d` | Target specific display (0-based index) |
| `--stop <N>` | | Stop wallpaper on display N |
| `--stopall` | | Stop all running wallpapers |
| `--port <PORT>` | `-p` | HTTP server port for local files (default: 8080) |
| `--scale <FACTOR>` | | Shader render scale for `.shader` inputs (default: 1.0) |
| `--time-scale <FACTOR>` | | Shader animation time scale for `.shader` inputs (default: 1.0) |
| `--verbose` | `-v` | Enable verbose output |
| `--help` | `-h` | Show help message |
| `--version` | `-V` | Show version |

### Shader Tuning

- `--scale` changes render resolution for local `.shader` inputs. Lower values usually improve performance at the cost of sharpness.
- `--time-scale` changes the speed of the injected `iTime` and `iTimeDelta` values for local `.shader` inputs.
- Use `--time-scale 1.0` for original speed, values below `1.0` to slow animation down, values above `1.0` to speed it up, and `0` to freeze time for debugging.
- `--time-scale` avoids editing shader source when you only want to compare animation pacing across different shaders.

### Troubleshooting

- **WebView2 not available (Windows)** — Install from [Microsoft](https://developer.microsoft.com/microsoft-edge/webview2/)
- **Layer-shell not available (Linux)** — Ensure you are on a Wayland session with a layer-shell compositor (e.g. Hyprland), and install `libgtk-3-dev libwebkit2gtk-4.1-dev libgtk-layer-shell-dev`
- **Port already in use** — Use `--port 9000` to pick a different port
- **Shader performance is poor** — Use `--scale 0.5` or `--scale 0.75` for `.shader` inputs
- **Finding display numbers** — Run with `--verbose` to list detected displays

### Architecture

```
src/
├── main.rs           # Entry point, CLI dispatch, URL transformation
├── cli.rs            # Argument parsing (clap)
├── config.rs         # Configuration and instance tracking
├── server.rs         # Local HTTP server (tiny_http)
├── ipc.rs            # IPC (named pipes / Unix sockets)
├── display.rs        # Shared monitor data model
├── wallpaper.rs      # Shared wallpaper config / errors
└── platform/
    ├── windows/      # WorkerW technique + WebView2
    ├── linux/        # GTK layer-shell + WebKitGTK
    └── macos/        # NSWindow desktop-level + WKWebView
```

### License

MIT

---

## 中文

### 功能特性

- **跨平台** — 支持 Windows、Linux (Wayland) 和 macOS
- **多显示器** — 应用到所有显示器或指定特定显示器
- **本地文件支持** — 内置 HTTP 服务器，支持本地 HTML/JS/CSS 项目
- **ShaderToy 集成** — 自动将 ShaderToy URL 转换为全屏嵌入格式
- **本地 `.shader` 支持** — 将单文件 ShaderToy 片段包装成全屏 WebGL 运行时
- **IPC 控制** — 通过命名管道 (Windows) 或 Unix Domain Socket (Linux/macOS) 远程停止壁纸
- **优雅关闭** — Ctrl+C 处理并执行平台相关清理

### 平台实现

| 平台 | 技术方案 | WebView 后端 |
|------|----------|-------------|
| Windows | WorkerW 桌面嵌入 | WebView2 (Edge) |
| Linux | GTK layer-shell 背景层 | WebKitGTK |
| macOS | NSWindow 桌面层级窗口 | WKWebView |

### 系统要求

- **Windows** — Windows 10 (2018年4月更新+) 或 Windows 11；WebView2 运行时（通常已预装）
- **Linux** — 支持 layer-shell 的 Wayland 会话（如 Hyprland）；GTK 3、WebKitGTK、gtk-layer-shell
- **macOS** — macOS 10.10+；无额外依赖（WKWebView 为系统框架）

### 安装

```bash
git clone https://github.com/user/webwallpaper.git
cd webwallpaper

# Linux 依赖（Debian/Ubuntu 系）
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libgtk-layer-shell-dev

cargo build --release
# 二进制文件：target/release/webwallpaper(.exe)
```

### 使用方法

```bash
# 将 URL 设为壁纸（所有显示器）
webwallpaper https://example.com/wallpaper.html

# 指定显示器（从 0 开始的索引）
webwallpaper https://example.com --display 0

# ShaderToy（自动转换为全屏嵌入格式）
webwallpaper "https://www.shadertoy.com/view/tlVGDt"

# 本地 ShaderToy 风格着色器
webwallpaper ./octagrams.shader

# 降低渲染缩放以提升性能
webwallpaper ./octagrams.shader --scale 0.5

# 不修改 shader 源码，直接放慢动画速度
webwallpaper ./octagrams.shader --time-scale 0.5

# 加快动画速度，便于对比或调试
webwallpaper ./octagrams.shader --time-scale 2.0

# 本地 HTML 文件
webwallpaper ./my-wallpaper/index.html

# 包含 index.html 的目录
webwallpaper ./my-wallpaper/

# 自定义 HTTP 服务器端口
webwallpaper ./wallpaper.html --port 9000

# 停止显示器 0 上的壁纸
webwallpaper --stop 0

# 停止所有壁纸
webwallpaper --stopall

# 详细输出
webwallpaper --verbose https://example.com
```

### 命令行选项

| 选项 | 简写 | 描述 |
|------|------|------|
| `--display <N>` | `-d` | 指定目标显示器（从 0 开始的索引） |
| `--stop <N>` | | 停止显示器 N 上的壁纸 |
| `--stopall` | | 停止所有运行中的壁纸 |
| `--port <PORT>` | `-p` | 本地文件 HTTP 服务器端口（默认：8080） |
| `--scale <FACTOR>` | | `.shader` 输入的渲染缩放（默认：1.0） |
| `--time-scale <FACTOR>` | | `.shader` 输入的动画时间缩放（默认：1.0） |
| `--verbose` | `-v` | 启用详细输出 |
| `--help` | `-h` | 显示帮助信息 |
| `--version` | `-V` | 显示版本信息 |

### Shader 调节

- `--scale` 用于调整本地 `.shader` 输入的渲染分辨率；值越小通常性能越好，但画面会更模糊。
- `--time-scale` 用于调整本地 `.shader` 输入注入的 `iTime` 和 `iTimeDelta` 速度。
- `--time-scale 1.0` 表示原始速度，小于 `1.0` 表示减速，大于 `1.0` 表示加速，`0` 可用于冻结时间以便调试。
- 当你只是想对比不同 shader 的动画节奏时，`--time-scale` 可以避免逐个修改 shader 源码。

### 故障排除

- **WebView2 不可用 (Windows)** — 从 [Microsoft](https://developer.microsoft.com/microsoft-edge/webview2/) 安装
- **Layer-shell 不可用 (Linux)** — 确认在支持 layer-shell 的 Wayland 会话中运行（如 Hyprland），并安装 `libgtk-3-dev libwebkit2gtk-4.1-dev libgtk-layer-shell-dev`
- **端口被占用** — 使用 `--port 9000` 指定其他端口
- **Shader 性能较差** — 对 `.shader` 输入使用 `--scale 0.5` 或 `--scale 0.75`
- **查找显示器编号** — 使用 `--verbose` 运行以查看检测到的显示器

### 项目结构

```
src/
├── main.rs           # 入口点、CLI 分发、URL 转换
├── cli.rs            # 参数解析 (clap)
├── config.rs         # 配置和实例跟踪
├── server.rs         # 本地 HTTP 服务器 (tiny_http)
├── ipc.rs            # 进程间通信（命名管道 / Unix Socket）
├── display.rs        # 共享显示器数据模型
├── wallpaper.rs      # 共享壁纸配置与错误类型
└── platform/
    ├── windows/      # WorkerW 技术 + WebView2
    ├── linux/        # GTK layer-shell + WebKitGTK
    └── macos/        # NSWindow 桌面层级 + WKWebView
```

### 许可证

MIT
