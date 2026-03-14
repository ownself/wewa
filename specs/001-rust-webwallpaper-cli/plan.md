# Implementation Plan: Rust Cross-Platform WebWallpaper CLI

**Branch**: `001-rust-webwallpaper-cli` | **Date**: 2026-03-14 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-rust-webwallpaper-cli/spec.md`
**Phase 1 Focus**: Windows implementation

## Summary

Build a Rust-based CLI tool that displays web content (URLs or local HTML files) as desktop wallpaper. The wallpaper window is fullscreen, frameless, positioned at the bottom of the window stack, and transparent to mouse/keyboard input. Phase 1 focuses on Windows 10+ implementation using WebView2, with cross-platform architecture prepared for future Linux/macOS support.

## Technical Context

**Language/Version**: Rust 1.75+ (2021 edition)
**Primary Dependencies**:
- `wry` (webview rendering with WebView2 on Windows)
- `tao` (window management, comes with wry)
- `clap` (CLI argument parsing)
- `interprocess` (named pipes for IPC)
- `tiny_http` (local HTTP server for file serving)
- `windows` (Windows API bindings)

**Storage**: File-based PID tracking in user's temp directory
**Testing**: `cargo test` with integration tests for CLI commands
**Target Platform**: Windows 10+ (Phase 1), Linux/macOS (future phases)
**Project Type**: CLI / Desktop App
**Performance Goals**: <5s startup, <200MB memory
**Constraints**: Click-through input, always-on-bottom Z-order, no taskbar presence
**Scale/Scope**: Single-user desktop tool, up to 6 monitors

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Status**: PASS (no project constitution defined - using default best practices)

- Simple single-binary architecture
- Clear separation of platform-specific code
- Standard Rust testing practices

## Project Structure

### Documentation (this feature)

```text
specs/001-rust-webwallpaper-cli/
├── plan.md              # This file
├── research.md          # Phase 0 output - technology research
├── data-model.md        # Phase 1 output - entity definitions
├── quickstart.md        # Phase 1 output - getting started guide
├── contracts/           # Phase 1 output - CLI interface contract
│   └── cli.md
└── tasks.md             # Phase 2 output (created by /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── main.rs              # Entry point, CLI dispatch
├── cli.rs               # Argument parsing with clap
├── config.rs            # Configuration and instance tracking
├── server.rs            # Local HTTP server for file serving
├── ipc.rs               # Inter-process communication (named pipes)
├── display.rs           # Monitor enumeration (cross-platform trait)
├── wallpaper.rs         # Wallpaper window management (cross-platform trait)
└── platform/
    ├── mod.rs           # Platform detection and dispatch
    └── windows/
        ├── mod.rs       # Windows module exports
        ├── display.rs   # EnumDisplayMonitors implementation
        └── wallpaper.rs # WebView2 + Win32 API implementation

tests/
├── cli_test.rs          # CLI argument parsing tests
├── server_test.rs       # HTTP server tests
└── integration/
    └── start_stop_test.rs  # Full workflow integration tests

Cargo.toml               # Dependencies and build configuration
```

**Structure Decision**: Single Rust project with platform-specific modules under `src/platform/`. This allows code reuse for cross-platform logic while isolating Windows-specific Win32 API calls.

## Implementation Phases

### Phase 1: Windows Core (Current Focus)

1. **Project Setup**: Initialize Cargo project, configure dependencies
2. **CLI Interface**: Implement argument parsing with clap
3. **Display Enumeration**: Windows EnumDisplayMonitors implementation
4. **Webview Window**: Create frameless WebView2 window with wry/tao
5. **Wallpaper Behavior**: Apply Win32 window styles for click-through and Z-order
6. **Local Server**: tiny_http server for serving local HTML files
7. **Instance Management**: PID tracking and IPC via named pipes
8. **Stop Commands**: Implement --stop and --stopall functionality

### Phase 2: Polish & Testing (Future)

- Error handling improvements
- Comprehensive integration tests
- Documentation and help text

### Phase 3: Cross-Platform (Future)

- Linux X11/Wayland support
- macOS WKWebView support

## Complexity Tracking

> No constitution violations - simple single-project architecture.

| Decision | Rationale |
|----------|-----------|
| Single binary | Simplest deployment, no runtime dependencies beyond WebView2 |
| Platform modules | Clean separation allows adding Linux/macOS without refactoring |
| Named pipes IPC | Native Windows solution, no external dependencies |
