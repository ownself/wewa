# WebWallpaper

Display web content as desktop wallpaper on Windows and Linux.

A high-performance Rust CLI tool that renders web pages (URLs or local HTML files) as fullscreen desktop wallpaper with multi-monitor support. Windows uses the WorkerW technique for deep desktop integration, and Linux currently targets Wayland compositors with layer-shell support such as Hyprland.

---

**[English](#english)** | **[中文](#中文)**

---

## English

### Features

- **Cross-Platform Architecture** - Shared CLI, config, URL handling, local file serving, and IPC protocol
- **Windows Desktop Integration** - Uses WorkerW to embed wallpaper into the desktop layer
- **Wayland / Hyprland Support** - Uses layer-shell on Linux Wayland compositors that support background layers
- **Desktop-Friendly Windows** - No decorations, no taskbar/pager entry, no focus stealing
- **Multi-Monitor Support** - Apply to all displays or target specific ones
- **Local File Support** - Built-in HTTP server for local HTML files with symlink support
- **ShaderToy Integration** - Automatically transforms ShaderToy URLs to fullscreen embed format
- **IPC Control** - Stop wallpapers remotely via named pipes on Windows and Unix domain sockets on Linux
- **Graceful Shutdown** - Ctrl+C handling with platform-specific cleanup

### Requirements

- **Windows**
  - **Windows 10** (April 2018 Update or later) or **Windows 11**
  - **WebView2 Runtime** (usually pre-installed, or [download here](https://developer.microsoft.com/microsoft-edge/webview2/))
- **Linux**
  - Wayland session
  - A compositor with `layer-shell` support, currently tested against the Hyprland target path
  - GTK 3, WebKitGTK, and gtk-layer-shell development/runtime libraries

### Installation

#### Build from Source

```bash
# Clone the repository
git clone https://github.com/user/webwallpaper.git
cd webwallpaper

# Linux example dependencies (Debian/Ubuntu family)
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libgtk-layer-shell-dev

# Build release binary
cargo build --release

# Binary is at target/release/webwallpaper (Linux) or target/release/webwallpaper.exe (Windows)
```

### Usage

#### Display a URL as Wallpaper

```bash
# Apply to all monitors
webwallpaper https://example.com/wallpaper.html

# Apply to specific monitor (0-based index)
webwallpaper https://example.com --display 0
```

#### ShaderToy Shaders

```bash
# ShaderToy URLs are automatically converted to fullscreen embed format
webwallpaper "https://www.shadertoy.com/view/tlVGDt"

# Output:
# [INFO] Transformed ShaderToy URL to embed format:
# [INFO]   Original: https://www.shadertoy.com/view/tlVGDt
# [INFO]   Embed: https://www.shadertoy.com/embed/tlVGDt?gui=false&t=0&paused=false&muted=true
```

#### Display a Local HTML File

```bash
# Single file
webwallpaper ./my-wallpaper/index.html

# Directory with index.html
webwallpaper ./my-wallpaper/

# Use custom port for HTTP server
webwallpaper ./wallpaper.html --port 9000
```

#### Stop Running Wallpapers

```bash
# Stop wallpaper on display 0
webwallpaper --stop 0

# Stop all running wallpapers
webwallpaper --stopall
```

#### Verbose Mode

```bash
# Show detailed output including WorkerW setup
webwallpaper --verbose https://example.com
```

### Command Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--display <N>` | `-d` | Target specific display (0-based index) |
| `--stop <N>` | | Stop wallpaper on display N |
| `--stopall` | | Stop all running wallpapers |
| `--port <PORT>` | `-p` | HTTP server port for local files (default: 8080) |
| `--verbose` | `-v` | Enable verbose output |
| `--help` | `-h` | Show help message |
| `--version` | `-V` | Show version |

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Display not found |
| 3 | No running instance to stop |
| 4 | Required webview/runtime support not available |
| 5 | Server startup failed (port in use) |

### How It Works

WebWallpaper now has two platform backends:

- **Windows:** uses the **WorkerW technique** to embed content as part of the desktop.
- **Linux (Wayland-first):** creates GTK layer-shell background windows bound to compositor outputs and embeds web content through `wry`'s GTK path.

Windows flow:

1. Sends message `0x052C` to Progman to spawn a WorkerW window behind desktop icons
2. Finds the WorkerW handle by enumerating windows with SHELLDLL_DefView
3. Uses `SetParent` to attach the wallpaper window as a child of WorkerW
4. Applies WS_CHILD and transparency styles for proper integration

Linux Wayland flow:

1. Enumerates monitors through GDK
2. Creates one GTK window per target output
3. Turns each window into a layer-shell background surface
4. Embeds a `wry` webview through GTK so it works on Wayland compositors such as Hyprland

This makes the wallpaper backend-specific but keeps the CLI and instance-management flow shared.
- Truly part of the desktop (not just a bottom-most window)
- Invisible to window managers like komorebi
- Unaffected by Win+D (Show Desktop)
- Properly cleaned up when closed (desktop refresh)

### Creating a Web Wallpaper

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

### Troubleshooting

#### WebView2 Runtime Not Available

Install WebView2 from: https://developer.microsoft.com/microsoft-edge/webview2/

#### Wayland / Layer-Shell Not Available

If Linux startup reports missing Wayland or layer-shell support:

- Make sure you are running a Wayland session, not X11
- Use a compositor that supports layer-shell, such as Hyprland
- Install the GTK / WebKitGTK / gtk-layer-shell packages listed above

#### Port Already in Use

Use a different port:

```bash
webwallpaper ./wallpaper.html --port 9000
```

#### Finding Display Numbers

Use verbose mode to see available displays:

```bash
webwallpaper --verbose https://example.com
# [INFO] Found 2 display(s)
# [INFO] Display 0: 1920x1080 at (0, 0) [Primary]
# [INFO] Display 1: 1920x1080 at (1920, 0)
```

### Architecture

```
src/
├── main.rs           # Entry point, CLI dispatch, URL transformation
├── cli.rs            # Argument parsing (clap)
├── config.rs         # Configuration and instance tracking
├── server.rs         # Local HTTP server (tiny_http) with symlink support
├── ipc.rs            # Inter-process communication (named pipes / Unix sockets)
├── display.rs        # Shared monitor data model
├── wallpaper.rs      # Shared wallpaper configuration/errors
└── platform/
    ├── windows/      # Windows-specific implementation
    │   ├── mod.rs        # DPI + WebView2 checks
    │   ├── display.rs    # EnumDisplayMonitors
    │   └── wallpaper.rs  # WorkerW technique + WebView2
    └── linux/        # Linux Wayland-first implementation
        ├── mod.rs        # GTK + runtime checks
        ├── display.rs    # GDK monitor enumeration
        └── wallpaper.rs  # GTK layer-shell + wry GTK webview
```

### License

MIT

---

## 中文

### 功能特性

- **跨平台架构** - 共享 CLI、配置、URL 处理、本地文件服务与 IPC 协议
- **Windows 深度桌面集成** - 使用 WorkerW 技术将壁纸嵌入桌面层
- **Wayland / Hyprland 支持** - Linux 下优先支持具备 layer-shell 的 Wayland compositor
- **桌面友好窗口行为** - 无装饰、不抢焦点、不显示在任务栏或分页器中
- **多显示器支持** - 可应用到所有显示器或指定特定显示器
- **本地文件支持** - 内置 HTTP 服务器，支持符号链接
- **ShaderToy 集成** - 自动将 ShaderToy URL 转换为全屏嵌入格式
- **IPC 控制** - Windows 使用命名管道，Linux 使用 Unix Domain Socket
- **优雅关闭** - Ctrl+C 处理并执行平台相关清理

### 系统要求

- **Windows**
  - **Windows 10**（2018年4月更新或更高版本）或 **Windows 11**
  - **WebView2 运行时**（通常已预装，或[点击下载](https://developer.microsoft.com/microsoft-edge/webview2/)）
- **Linux**
  - Wayland 会话
  - 支持 `layer-shell` 的 compositor，当前首要目标是 Hyprland
  - GTK 3、WebKitGTK 与 gtk-layer-shell 运行库 / 开发库

### 安装

#### 从源码编译

```bash
# 克隆仓库
git clone https://github.com/user/webwallpaper.git
cd webwallpaper

# Linux 示例依赖（Debian/Ubuntu 系）
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libgtk-layer-shell-dev

# 编译发布版本
cargo build --release

# 二进制文件位于 target/release/webwallpaper（Linux）或 target/release/webwallpaper.exe（Windows）
```

### 使用方法

#### 将 URL 显示为壁纸

```bash
# 应用到所有显示器
webwallpaper https://example.com/wallpaper.html

# 应用到特定显示器（从0开始的索引）
webwallpaper https://example.com --display 0
```

#### ShaderToy 着色器

```bash
# ShaderToy URL 会自动转换为全屏嵌入格式
webwallpaper "https://www.shadertoy.com/view/tlVGDt"

# 输出：
# [INFO] Transformed ShaderToy URL to embed format:
# [INFO]   Original: https://www.shadertoy.com/view/tlVGDt
# [INFO]   Embed: https://www.shadertoy.com/embed/tlVGDt?gui=false&t=0&paused=false&muted=true
```

#### 显示本地 HTML 文件

```bash
# 单个文件
webwallpaper ./my-wallpaper/index.html

# 包含 index.html 的目录
webwallpaper ./my-wallpaper/

# 使用自定义端口
webwallpaper ./wallpaper.html --port 9000
```

#### 停止运行中的壁纸

```bash
# 停止显示器 0 上的壁纸
webwallpaper --stop 0

# 停止所有运行中的壁纸
webwallpaper --stopall
```

#### 详细模式

```bash
# 显示详细输出，包括 WorkerW 设置信息
webwallpaper --verbose https://example.com
```

### 命令行选项

| 选项 | 简写 | 描述 |
|------|------|------|
| `--display <N>` | `-d` | 指定目标显示器（从0开始的索引） |
| `--stop <N>` | | 停止显示器 N 上的壁纸 |
| `--stopall` | | 停止所有运行中的壁纸 |
| `--port <PORT>` | `-p` | 本地文件 HTTP 服务器端口（默认：8080） |
| `--verbose` | `-v` | 启用详细输出 |
| `--help` | `-h` | 显示帮助信息 |
| `--version` | `-V` | 显示版本信息 |

### 退出码

| 代码 | 含义 |
|------|------|
| 0 | 成功 |
| 1 | 一般错误 |
| 2 | 显示器未找到 |
| 3 | 没有运行中的实例可停止 |
| 4 | 所需 WebView / 运行时支持不可用 |
| 5 | 服务器启动失败（端口被占用） |

### 工作原理

WebWallpaper 现在包含两个平台后端：

- **Windows：** 使用 **WorkerW 技术** 把内容嵌入桌面。
- **Linux（Wayland 优先）：** 创建绑定到输出的 GTK layer-shell 背景窗口，并通过 `wry` 的 GTK 路径嵌入网页内容。

Windows 路径：

1. 向 Progman 发送消息 `0x052C` 以在桌面图标后面生成 WorkerW 窗口
2. 通过枚举具有 SHELLDLL_DefView 的窗口来查找 WorkerW 句柄
3. 使用 `SetParent` 将壁纸窗口附加为 WorkerW 的子窗口
4. 应用 WS_CHILD 和透明度样式以实现正确集成

Linux Wayland 路径：

1. 通过 GDK 枚举显示器
2. 为每个目标输出创建一个 GTK 窗口
3. 将窗口转换为 layer-shell 背景层 surface
4. 通过 `wry` 的 GTK 接口嵌入 WebView，以适配 Hyprland 这类 Wayland compositor

这样平台集成方式虽然不同，但 CLI、实例管理与 URL 处理仍保持共享。

### 创建网页壁纸

创建一个 `index.html` 文件：

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
        now.toLocaleTimeString('zh-CN', { hour12: false });
    }
    setInterval(updateClock, 1000);
    updateClock();
  </script>
</body>
</html>
```

然后运行：

```bash
webwallpaper ./index.html
```

### 故障排除

#### WebView2 运行时不可用

从以下地址安装 WebView2：https://developer.microsoft.com/microsoft-edge/webview2/

#### Wayland / Layer-Shell 不可用

如果 Linux 启动时提示缺少 Wayland 或 layer-shell 支持：

- 确认当前运行的是 Wayland，而不是 X11
- 使用支持 layer-shell 的 compositor，例如 Hyprland
- 安装上文列出的 GTK / WebKitGTK / gtk-layer-shell 依赖

#### 端口已被占用

使用不同的端口：

```bash
webwallpaper ./wallpaper.html --port 9000
```

#### 查找显示器编号

使用详细模式查看可用显示器：

```bash
webwallpaper --verbose https://example.com
# [INFO] Found 2 display(s)
# [INFO] Display 0: 1920x1080 at (0, 0) [Primary]
# [INFO] Display 1: 1920x1080 at (1920, 0)
```

### 项目结构

```
src/
├── main.rs           # 入口点、CLI 分发、URL 转换
├── cli.rs            # 参数解析 (clap)
├── config.rs         # 配置和实例跟踪
├── server.rs         # 本地 HTTP 服务器 (tiny_http)，支持符号链接
├── ipc.rs            # 进程间通信（命名管道 / Unix Socket）
├── display.rs        # 共享显示器数据模型
├── wallpaper.rs      # 共享壁纸配置与错误类型
└── platform/
    ├── windows/      # Windows 特定实现
    │   ├── mod.rs        # DPI + WebView2 检测
    │   ├── display.rs    # EnumDisplayMonitors
    │   └── wallpaper.rs  # WorkerW 技术 + WebView2
    └── linux/        # Linux Wayland 优先实现
        ├── mod.rs        # GTK + 运行时检查
        ├── display.rs    # GDK 显示器枚举
        └── wallpaper.rs  # GTK layer-shell + wry GTK WebView
```

### 许可证

MIT
