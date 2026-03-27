//! macOS wallpaper implementation using tao + wry with NSWindow desktop-level ordering.
//!
//! Places a borderless, click-through WKWebView window at `kCGDesktopWindowLevel + 1`,
//! which sits above the real desktop wallpaper but below Finder's desktop icons.

// The `cocoa` crate marks its API as deprecated in favor of `objc2-app-kit`,
// but `tao` still depends on `cocoa`, so we suppress these warnings here.
#![allow(deprecated)]

use crate::ipc::{IpcCommand, IpcServer};
use crate::wallpaper::{RenderMode, WallpaperConfig, WallpaperError, WallpaperResult};
use cocoa::appkit::{NSWindow, NSWindowCollectionBehavior};
use cocoa::base::{id, YES};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Duration;
use tao::dpi::{LogicalPosition, LogicalSize};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS, WindowExtMacOS};
use tao::window::{Window, WindowBuilder};
use wry::{WebView, WebViewBuilder};

// CoreGraphics window-level key for the desktop layer.
// CGWindowLevelForKey(kCGDesktopWindowLevelKey) returns the compositor
// level at which the actual desktop wallpaper is drawn.
// We place our window one level above that so it covers the wallpaper
// but stays below Finder desktop icons (kCGDesktopIconWindowLevel).
extern "C" {
    fn CGWindowLevelForKey(key: i32) -> i32;
}
const K_CG_DESKTOP_WINDOW_LEVEL_KEY: i32 = 2;

/// Create and run wallpaper windows for all requested displays.
///
/// This function blocks on the tao event loop and does not return until
/// shutdown is requested (Ctrl+C or IPC stop command).
pub fn create_wallpapers(configs: Vec<WallpaperConfig>) -> WallpaperResult<()> {
    if configs.is_empty() {
        return Ok(());
    }

    let render_mode = configs[0].render_mode.clone();
    match render_mode {
        RenderMode::NativeGpu => create_wallpapers_native_gpu(configs),
        RenderMode::WebView => create_wallpapers_webview(configs),
    }
}

/// Create wallpapers using native wgpu GPU rendering for shader files on macOS.
fn create_wallpapers_native_gpu(configs: Vec<WallpaperConfig>) -> WallpaperResult<()> {
    use crate::renderer::NativeRenderer;
    use raw_window_handle::{AppKitDisplayHandle, AppKitWindowHandle, RawDisplayHandle, RawWindowHandle};

    let verbose = configs[0].verbose;

    if verbose {
        println!(
            "[INFO] Creating native GPU wallpaper for {} display(s) on macOS",
            configs.len()
        );
    }

    let shutdown_flag = Arc::new(AtomicBool::new(false));

    // Start IPC server
    let mut ipc_server = IpcServer::new();
    let ipc_rx = if let Err(e) = ipc_server.start() {
        if verbose {
            println!("[WARN] Failed to start IPC server: {}", e);
        }
        None
    } else {
        if verbose {
            println!("[INFO] IPC server started");
        }
        ipc_server.command_receiver()
    };

    // Ctrl+C handler
    let ctrlc_shutdown = shutdown_flag.clone();
    let ctrlc_verbose = verbose;
    let _ = ctrlc::set_handler(move || {
        if ctrlc_verbose {
            println!("\n[INFO] Ctrl+C received, initiating shutdown...");
        }
        ctrlc_shutdown.store(true, Ordering::Relaxed);
    });

    // IPC command processor thread
    if let Some(rx) = ipc_rx {
        let ipc_shutdown = shutdown_flag.clone();
        start_ipc_processor(rx, ipc_shutdown, verbose);
    }

    // Create the event loop with Accessory policy so no Dock icon appears
    let mut event_loop = EventLoop::new();
    event_loop.set_activation_policy(ActivationPolicy::Accessory);

    struct NativeWallpaperWindow {
        window: Window,
        renderer: NativeRenderer,
    }

    let mut native_windows: Vec<NativeWallpaperWindow> = Vec::new();

    for config in &configs {
        let shader_source = config.shader_source.as_deref().ok_or_else(|| {
            WallpaperError::WindowCreationFailed("No shader source for NativeGpu mode".to_string())
        })?;

        if verbose {
            println!(
                "[INFO] Creating native GPU window for display {} ({}x{} at ({}, {}))",
                config.display.index,
                config.display.width,
                config.display.height,
                config.display.x,
                config.display.y,
            );
        }

        let window = WindowBuilder::new()
            .with_title(format!("WebWallpaper - Display {}", config.display.index))
            .with_position(LogicalPosition::new(config.display.x, config.display.y))
            .with_inner_size(LogicalSize::new(
                config.display.width,
                config.display.height,
            ))
            .with_decorations(false)
            .with_resizable(false)
            .with_visible(false)
            .build(&event_loop)
            .map_err(|e| WallpaperError::WindowCreationFailed(e.to_string()))?;

        // Apply desktop-level styles via the raw NSWindow pointer
        apply_wallpaper_styles(&window, verbose)?;

        // Extract raw NSView handle for wgpu surface creation
        let ns_view = window.ns_view();
        let raw_window = RawWindowHandle::AppKit(AppKitWindowHandle::new(
            std::num::NonZero::new(ns_view as isize).expect("NSView should not be null"),
        ));
        let raw_display = RawDisplayHandle::AppKit(AppKitDisplayHandle::new());

        // Create the native renderer
        let renderer = unsafe {
            NativeRenderer::new(
                raw_window,
                raw_display,
                config.display.width,
                config.display.height,
                shader_source,
                config.scale,
                config.time_scale,
            )
        }
        .map_err(|e| WallpaperError::WindowCreationFailed(format!("GPU renderer error: {}", e)))?;

        if verbose {
            println!(
                "[INFO] Native GPU renderer created for display {}",
                config.display.index
            );
        }

        window.set_visible(true);

        native_windows.push(NativeWallpaperWindow { window, renderer });
    }

    if verbose {
        println!("[INFO] Native GPU wallpaper is now running. Press Ctrl+C to stop.");
    }

    // Event loop — render on each frame
    event_loop.run(move |event, _, control_flow| {
        if shutdown_flag.load(Ordering::Relaxed) {
            if verbose {
                println!("[INFO] Shutdown flag detected, closing...");
            }
            ipc_server.shutdown();
            *control_flow = ControlFlow::Exit;
            return;
        }

        *control_flow = ControlFlow::Poll;

        match event {
            Event::MainEventsCleared => {
                for nw in &native_windows {
                    nw.window.request_redraw();
                }
            }
            Event::RedrawRequested(_) => {
                for nw in &mut native_windows {
                    if let Err(e) = nw.renderer.render_frame(0.0, 0.0, 0.0, 0.0) {
                        eprintln!("[ERROR] Render error: {}", e);
                        shutdown_flag.store(true, Ordering::Relaxed);
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                window_id,
                ..
            } => {
                for nw in &mut native_windows {
                    if nw.window.id() == window_id {
                        nw.renderer.resize(new_size.width, new_size.height);
                        if verbose {
                            println!(
                                "[INFO] Resized renderer to {}x{}",
                                new_size.width, new_size.height
                            );
                        }
                        break;
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested | WindowEvent::Destroyed,
                ..
            } => {
                ipc_server.shutdown();
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

/// Create wallpapers using WebView rendering for URLs and HTML files.
fn create_wallpapers_webview(configs: Vec<WallpaperConfig>) -> WallpaperResult<()> {
    let verbose = configs.first().map(|c| c.verbose).unwrap_or(false);

    if verbose {
        println!(
            "[INFO] Creating wallpaper windows for {} display(s)",
            configs.len()
        );
    }

    let shutdown_flag = Arc::new(AtomicBool::new(false));

    // Start IPC server
    let mut ipc_server = IpcServer::new();
    let ipc_rx = if let Err(e) = ipc_server.start() {
        if verbose {
            println!("[WARN] Failed to start IPC server: {}", e);
        }
        None
    } else {
        if verbose {
            println!("[INFO] IPC server started");
        }
        ipc_server.command_receiver()
    };

    // Ctrl+C handler
    let ctrlc_shutdown = shutdown_flag.clone();
    let ctrlc_verbose = verbose;
    let _ = ctrlc::set_handler(move || {
        if ctrlc_verbose {
            println!("\n[INFO] Ctrl+C received, initiating shutdown...");
        }
        ctrlc_shutdown.store(true, Ordering::Relaxed);
    });

    // IPC command processor thread
    if let Some(rx) = ipc_rx {
        let ipc_shutdown = shutdown_flag.clone();
        start_ipc_processor(rx, ipc_shutdown, verbose);
    }

    // Create the event loop with Accessory policy so no Dock icon appears
    let mut event_loop = EventLoop::new();
    event_loop.set_activation_policy(ActivationPolicy::Accessory);

    // Build windows + webviews
    let mut windows_and_webviews: Vec<(Window, WebView)> = Vec::new();

    for config in &configs {
        if verbose {
            println!(
                "[INFO] Creating wallpaper window for display {} ({}x{} at ({}, {}))",
                config.display.index,
                config.display.width,
                config.display.height,
                config.display.x,
                config.display.y,
            );
        }

        let window = WindowBuilder::new()
            .with_title(format!("WebWallpaper - Display {}", config.display.index))
            .with_position(LogicalPosition::new(config.display.x, config.display.y))
            .with_inner_size(LogicalSize::new(
                config.display.width,
                config.display.height,
            ))
            .with_decorations(false)
            .with_resizable(false)
            .with_visible(false)
            .build(&event_loop)
            .map_err(|e| WallpaperError::WindowCreationFailed(e.to_string()))?;

        // Apply desktop-level styles via the raw NSWindow pointer
        apply_wallpaper_styles(&window, verbose)?;

        let webview = WebViewBuilder::new(&window)
            .with_url(&config.url)
            .with_devtools(false)
            .with_background_color((0, 0, 0, 255))
            .build()
            .map_err(|e| WallpaperError::WindowCreationFailed(format!("WebView error: {}", e)))?;

        window.set_visible(true);

        if verbose {
            println!(
                "[INFO] Wallpaper window visible on display {}",
                config.display.index
            );
        }

        windows_and_webviews.push((window, webview));
    }

    if verbose {
        println!("[INFO] Wallpaper is now running. Press Ctrl+C to stop.");
    }

    // Event loop
    event_loop.run(move |event, _, control_flow| {
        if shutdown_flag.load(Ordering::Relaxed) {
            if verbose {
                println!("[INFO] Shutdown flag detected, closing...");
            }
            ipc_server.shutdown();
            *control_flow = ControlFlow::Exit;
            return;
        }

        *control_flow = ControlFlow::Poll;

        // Keep webviews alive
        let _ = &windows_and_webviews;

        match event {
            Event::MainEventsCleared => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested | WindowEvent::Destroyed,
                ..
            } => {
                ipc_server.shutdown();
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

/// Apply macOS-specific window styles that turn a normal window into a wallpaper.
fn apply_wallpaper_styles(window: &Window, verbose: bool) -> WallpaperResult<()> {
    let ns_window = window.ns_window() as id;

    unsafe {
        // Window level: one above the desktop wallpaper, below desktop icons.
        let desktop_level = CGWindowLevelForKey(K_CG_DESKTOP_WINDOW_LEVEL_KEY);
        let wallpaper_level = desktop_level + 1;
        ns_window.setLevel_(wallpaper_level as i64);
        if verbose {
            println!(
                "[INFO] Window level set to {} (desktop={}, +1)",
                wallpaper_level, desktop_level
            );
        }

        // Collection behavior:
        //   CanJoinAllSpaces — appear on every Space / virtual desktop
        //   Stationary       — do not move when switching Spaces
        //   IgnoresCycle     — exclude from Cmd+Tab / Mission Control
        ns_window.setCollectionBehavior_(
            NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorIgnoresCycle,
        );
        if verbose {
            println!("[INFO] Collection behavior: CanJoinAllSpaces | Stationary | IgnoresCycle");
        }

        // Click-through: all mouse events pass to whatever is underneath
        ns_window.setIgnoresMouseEvents_(YES);
        if verbose {
            println!("[INFO] Mouse events: ignored (click-through)");
        }

        // Remove shadow
        window.set_has_shadow(false);

        // Opaque — no need for transparency compositing
        ns_window.setOpaque_(YES);
    }

    Ok(())
}

/// Background thread that forwards IPC commands to the shutdown flag.
fn start_ipc_processor(rx: Receiver<IpcCommand>, shutdown_flag: Arc<AtomicBool>, verbose: bool) {
    std::thread::spawn(move || {
        while let Ok(cmd) = rx.recv() {
            if verbose {
                println!("[IPC] Received command: {:?}", cmd);
            }
            match cmd {
                IpcCommand::Stop(_) | IpcCommand::StopAll => {
                    shutdown_flag.store(true, Ordering::Relaxed);
                    break;
                }
                IpcCommand::Ping => {}
            }
        }
    });
}
