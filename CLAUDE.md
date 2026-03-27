# webwallpaper Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-03-27

## Active Technologies
- Rust 1.87+ (2021 edition) — bumped from 1.75+ due to wgpu 29.x MSRV requirement + wgpu 29 (with `glsl` feature), tao 0.30 (existing), pollster 0.4, bytemuck 1.x, raw-window-handle 0.6 (002-wgpu-shader-renderer)
- N/A (no persistent storage changes) (002-wgpu-shader-renderer)

- Rust 1.75+ (2021 edition) (001-rust-webwallpaper-cli)

## Project Structure

```text
src/
tests/
```

## Commands

cargo test; cargo clippy

## Code Style

Rust 1.75+ (2021 edition): Follow standard conventions

## Recent Changes
- 002-wgpu-shader-renderer: Added Rust 1.87+ (2021 edition) — bumped from 1.75+ due to wgpu 29.x MSRV requirement + wgpu 29 (with `glsl` feature), tao 0.30 (existing), pollster 0.4, bytemuck 1.x, raw-window-handle 0.6

- 001-rust-webwallpaper-cli: Added Rust 1.75+ (2021 edition)

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
