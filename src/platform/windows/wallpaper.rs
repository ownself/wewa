//! Windows WebView2 + Win32 API wallpaper implementation
//!
//! Creates and manages wallpaper windows using WebView2 for rendering
//! and Win32 API for window styling (click-through, Z-order, etc.)

use crate::ipc::{IpcCommand, IpcServer};
use crate::wallpaper::{WallpaperConfig, WallpaperError, WallpaperResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Duration;
use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::platform::windows::WindowExtWindows;
use tao::window::{Window, WindowBuilder};
use windows::Win32::Foundation::{COLORREF, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW, SetWindowPos,
    GWL_EXSTYLE, GWL_STYLE, HWND_BOTTOM, LAYERED_WINDOW_ATTRIBUTES_FLAGS, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::PCWSTR;
use wry::{WebView, WebViewBuilder};

// LWA_ALPHA constant
const LWA_ALPHA: LAYERED_WINDOW_ATTRIBUTES_FLAGS = LAYERED_WINDOW_ATTRIBUTES_FLAGS(2u32);

/// Create and run wallpaper windows for multiple displays with IPC support
///
/// This function blocks and runs the event loop. It should be called
/// from the main thread. All windows share the same event loop.
pub fn create_wallpapers(configs: Vec<WallpaperConfig>) -> WallpaperResult<()> {
    if configs.is_empty() {
        return Ok(());
    }

    let verbose = configs.first().map(|c| c.verbose).unwrap_or(false);

    if verbose {
        println!(
            "[INFO] Creating wallpaper windows for {} display(s)",
            configs.len()
        );
    }

    // Shutdown flag shared between threads
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

    // Set up Ctrl+C handler
    let ctrlc_shutdown = shutdown_flag.clone();
    let ctrlc_verbose = verbose;
    let _ = ctrlc::set_handler(move || {
        if ctrlc_verbose {
            println!("\n[INFO] Ctrl+C received, initiating shutdown...");
        }
        ctrlc_shutdown.store(true, Ordering::Relaxed);
    });

    // Start IPC command processor thread
    if let Some(rx) = ipc_rx {
        let ipc_shutdown = shutdown_flag.clone();
        start_ipc_processor(rx, ipc_shutdown, verbose);
    }

    // Create event loop
    let event_loop = EventLoop::new();

    // Create all windows and webviews (store display info for later positioning)
    let mut windows_and_webviews: Vec<(Window, WebView, HWND, i32, i32, u32, u32)> = Vec::new();

    for config in &configs {
        if verbose {
            println!(
                "[INFO] Creating wallpaper window for display {}",
                config.display.index
            );
            println!("[INFO] URL: {}", config.url);
            println!(
                "[INFO] Position: ({}, {}), Size: {}x{}",
                config.display.x,
                config.display.y,
                config.display.width,
                config.display.height
            );
        }

        // Build the window using full display dimensions (not work area)
        // This allows the wallpaper to extend behind the taskbar
        let window = WindowBuilder::new()
            .with_title(format!("WebWallpaper - Display {}", config.display.index))
            .with_position(PhysicalPosition::new(
                config.display.x,
                config.display.y,
            ))
            .with_inner_size(PhysicalSize::new(
                config.display.width,
                config.display.height,
            ))
            .with_decorations(false)
            .with_resizable(false)
            .with_always_on_top(false)
            .with_visible(true)
            .build(&event_loop)
            .map_err(|e| WallpaperError::WindowCreationFailed(e.to_string()))?;

        // Get the HWND
        let hwnd = HWND(window.hwnd() as isize);

        if verbose {
            println!(
                "[INFO] Window created for display {}, deferring style application...",
                config.display.index
            );
        }

        // NOTE: We don't apply wallpaper styles yet - we need to wait for the window
        // to be fully initialized before applying styles to avoid window manager interference

        // Create the webview with black background to avoid white border artifacts
        let webview = WebViewBuilder::new(&window)
            .with_url(&config.url)
            .with_devtools(false)
            .with_background_color((0, 0, 0, 255))  // Black background
            .build()
            .map_err(|e| WallpaperError::WindowCreationFailed(format!("WebView error: {}", e)))?;

        if verbose {
            println!("[INFO] WebView created for display {}", config.display.index);
        }

        // Store window info along with display coordinates for later positioning
        windows_and_webviews.push((
            window,
            webview,
            hwnd,
            config.display.x,
            config.display.y,
            config.display.width,
            config.display.height,
        ));
    }

    if verbose {
        println!(
            "[INFO] All {} wallpaper window(s) created successfully",
            windows_and_webviews.len()
        );
        println!("[INFO] Waiting for windows to fully initialize before applying wallpaper styles...");
    }

    // CRITICAL: Wait for windows to fully initialize before applying styles
    // This delay is essential to prevent window managers (like komorebi) from
    // capturing and managing our window. The Python implementation uses 1 second.
    std::thread::sleep(Duration::from_millis(1000));

    // Now apply all wallpaper styles and set exact position/size after the delay
    for (_, _, hwnd, x, y, width, height) in &windows_and_webviews {
        if let Err(e) = apply_wallpaper_styles(*hwnd, verbose) {
            if verbose {
                println!("[WARN] Failed to apply wallpaper styles: {}", e);
            }
        }

        // Use SetWindowPos to set the position and size after applying styles
        // Offset by -1 to hide the 1-pixel WebView2 border on top/left edges
        // This also helps prevent overflow to adjacent displays
        unsafe {
            let _ = SetWindowPos(
                *hwnd,
                HWND_BOTTOM,
                *x - 1,
                *y - 1,
                *width as i32,
                *height as i32,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }

    // Apply Z-order again after a short additional delay
    std::thread::sleep(Duration::from_millis(200));
    for (_, _, hwnd, _, _, _, _) in &windows_and_webviews {
        apply_z_order(*hwnd, false);
    }

    if verbose {
        println!("[INFO] Wallpaper is now running. Press Ctrl+C to stop.");
    }

    // Run the event loop with polling to check shutdown flag
    event_loop.run(move |event, _, control_flow| {
        // Check shutdown flag periodically
        if shutdown_flag.load(Ordering::Relaxed) {
            if verbose {
                println!("[INFO] Shutdown flag detected, closing windows...");
            }
            ipc_server.shutdown();
            *control_flow = ControlFlow::Exit;
            return;
        }

        // Use Poll to periodically check shutdown flag
        *control_flow = ControlFlow::Poll;

        // Keep windows_and_webviews alive
        let _ = &windows_and_webviews;

        match event {
            Event::MainEventsCleared => {
                // Small sleep to avoid busy-waiting
                std::thread::sleep(Duration::from_millis(50));
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                ipc_server.shutdown();
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Destroyed,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

/// Start a thread to process IPC commands
fn start_ipc_processor(
    rx: Receiver<IpcCommand>,
    shutdown_flag: Arc<AtomicBool>,
    verbose: bool,
) {
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
                IpcCommand::Ping => {
                    // Already handled in IPC server
                }
            }
        }
    });
}

/// Create and run a single wallpaper window
///
/// This function blocks and runs the event loop. It should be called
/// from the main thread.
#[allow(dead_code)]
pub fn create_wallpaper(config: WallpaperConfig) -> WallpaperResult<()> {
    // Delegate to multi-window implementation with single config
    create_wallpapers(vec![config])
}

/// Apply wallpaper-specific window styles
///
/// This applies:
/// - WS_POPUP: Pure popup window (invisible to window managers like komorebi)
/// - WS_EX_TOOLWINDOW: Hide from taskbar
/// - WS_EX_NOACTIVATE: Never receive focus
/// - WS_EX_TRANSPARENT + WS_EX_LAYERED: Click-through
/// - SetLayeredWindowAttributes: Proper transparency
fn apply_wallpaper_styles(hwnd: HWND, verbose: bool) -> WallpaperResult<()> {
    unsafe {
        // Set WS_POPUP style - this makes it invisible to window managers
        // This is crucial for avoiding interference from komorebi and similar tools
        if verbose {
            println!("[INFO] Setting WS_POPUP style to avoid window manager interference...");
        }
        SetWindowLongW(hwnd, GWL_STYLE, WS_POPUP.0 as i32);

        // Get current extended style
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;

        // Build new extended style
        let new_ex_style = (ex_style & !WS_EX_APPWINDOW.0) // Remove taskbar appearance
            | WS_EX_TOOLWINDOW.0      // Hide from taskbar
            | WS_EX_NOACTIVATE.0      // Never receive focus
            | WS_EX_TRANSPARENT.0     // Click-through
            | WS_EX_LAYERED.0; // Enable layered window

        if verbose {
            println!("[INFO] Applying extended window styles...");
            println!("[INFO]   WS_EX_TOOLWINDOW: hide from taskbar");
            println!("[INFO]   WS_EX_NOACTIVATE: prevent focus");
            println!("[INFO]   WS_EX_TRANSPARENT: click-through");
            println!("[INFO]   WS_EX_LAYERED: layered window");
        }

        // Apply the new extended style
        SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex_style as i32);

        // Set layered window attributes for click-through
        // Alpha value <255 ensures mouse events pass through reliably
        let result = SetLayeredWindowAttributes(hwnd, COLORREF(0), 252, LWA_ALPHA);
        if result.is_err() {
            if verbose {
                println!("[WARN] SetLayeredWindowAttributes failed");
            }
        }

        if verbose {
            println!("[INFO] Window styles applied successfully");
        }
    }

    // Apply Z-order (try to place behind desktop)
    apply_z_order(hwnd, verbose);

    Ok(())
}

/// Set window Z-order to position as wallpaper (behind normal windows, in front of desktop)
fn apply_z_order(hwnd: HWND, verbose: bool) {
    unsafe {
        if verbose {
            println!("[INFO] Setting window Z-order...");
        }

        // Try to find the desktop window (Progman) and place our window right after it
        // This makes the window act as a wallpaper - behind all normal windows but visible
        let progman_class: Vec<u16> = "Progman\0".encode_utf16().collect();
        let progman_title: Vec<u16> = "Program Manager\0".encode_utf16().collect();

        let desktop_hwnd = FindWindowW(
            PCWSTR::from_raw(progman_class.as_ptr()),
            PCWSTR::from_raw(progman_title.as_ptr()),
        );

        if desktop_hwnd.0 != 0 {
            if verbose {
                println!("[INFO] Found desktop window (Progman), positioning after it in Z-order...");
            }
            // Place our window right after the desktop window in Z-order
            // This puts us in front of the desktop but behind all other windows
            let _ = SetWindowPos(
                hwnd,
                desktop_hwnd,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        } else {
            if verbose {
                println!("[INFO] Desktop window not found, using HWND_BOTTOM as fallback...");
            }
            // Fallback: just place at bottom of Z-order
            let _ = SetWindowPos(
                hwnd,
                HWND_BOTTOM,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }
}
