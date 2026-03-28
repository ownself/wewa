//! Windows WebView2 + Win32 API wallpaper implementation
//!
//! Creates and manages wallpaper windows using WebView2 for rendering
//! and Win32 API for window styling (click-through, Z-order, etc.)
//!
//! Uses the WorkerW technique to embed wallpaper as a child of the desktop,
//! making it immune to window managers like komorebi.

use crate::ipc::{IpcCommand, IpcServer};
use crate::renderer::NativeRenderer;
use crate::wallpaper::{RenderMode, WallpaperConfig, WallpaperError, WallpaperResult};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle, Win32WindowHandle, WindowsDisplayHandle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Duration;
use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::platform::windows::WindowExtWindows;
use tao::window::{Window, WindowBuilder};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, COLORREF, HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowExW, FindWindowW, GetWindowLongW, SendMessageTimeoutW,
    SetLayeredWindowAttributes, SetParent, SetWindowLongW, SetWindowPos, SystemParametersInfoW,
    GWL_EXSTYLE, GWL_STYLE, HWND_BOTTOM, LAYERED_WINDOW_ATTRIBUTES_FLAGS, SMTO_NORMAL,
    SPIF_UPDATEINIFILE, SPI_SETDESKWALLPAPER, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_SHOWWINDOW, WS_CHILD, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TRANSPARENT, WS_POPUP,
};
use wry::{WebView, WebViewBuilder};

// LWA_ALPHA constant
const LWA_ALPHA: LAYERED_WINDOW_ATTRIBUTES_FLAGS = LAYERED_WINDOW_ATTRIBUTES_FLAGS(2u32);

/// Helper: create a null HWND
fn null_hwnd() -> HWND {
    HWND(std::ptr::null_mut())
}

/// Helper: check if HWND is null/invalid
fn is_hwnd_valid(hwnd: HWND) -> bool {
    !hwnd.0.is_null()
}

/// Desktop layer info for WorkerW technique
struct DesktopLayer {
    worker_w: HWND,
    #[allow(dead_code)]
    progman: HWND,
}

/// Setup the desktop layer using the WorkerW technique
///
/// This sends message 0x052C to Progman to spawn a WorkerW window behind
/// the desktop icons, then finds that WorkerW handle.
fn setup_desktop_layer(verbose: bool) -> Option<DesktopLayer> {
    unsafe {
        if verbose {
            println!("[INFO] Setting up desktop layer (WorkerW technique)...");
        }

        // Find Progman window
        let progman_class: Vec<u16> = "Progman\0".encode_utf16().collect();
        let progman = FindWindowW(PCWSTR::from_raw(progman_class.as_ptr()), PCWSTR::null())
            .unwrap_or(null_hwnd());

        if !is_hwnd_valid(progman) {
            if verbose {
                println!("[WARN] Could not find Progman window");
            }
            return None;
        }

        if verbose {
            println!("[INFO] Found Progman: {:?}", progman);
        }

        // Send 0x052C to Progman to spawn WorkerW behind desktop icons
        // Parameters: wParam=0xD, lParam=0x1
        let mut _result: usize = 0;
        let _ = SendMessageTimeoutW(
            progman,
            0x052C,
            windows::Win32::Foundation::WPARAM(0xD),
            LPARAM(0x1),
            SMTO_NORMAL,
            1000,
            Some(&mut _result),
        );

        if verbose {
            println!("[INFO] Sent spawn WorkerW message to Progman");
        }

        // Find WorkerW by enumerating windows
        // We need to find the window that has SHELLDLL_DefView as child,
        // then get its next sibling WorkerW
        let mut worker_w = null_hwnd();
        let worker_w_ptr = &mut worker_w as *mut HWND;

        let _ = EnumWindows(Some(enum_windows_callback), LPARAM(worker_w_ptr as isize));

        if !is_hwnd_valid(worker_w) {
            if verbose {
                println!("[WARN] Could not find WorkerW window");
            }
            return None;
        }

        if verbose {
            println!("[INFO] Found WorkerW: {:?}", worker_w);
        }

        Some(DesktopLayer { worker_w, progman })
    }
}

/// Callback for EnumWindows to find WorkerW
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let shell_class: Vec<u16> = "SHELLDLL_DefView\0".encode_utf16().collect();

    // Check if this window has SHELLDLL_DefView as child
    let shell_view = FindWindowExW(
        hwnd,
        null_hwnd(),
        PCWSTR::from_raw(shell_class.as_ptr()),
        PCWSTR::null(),
    )
    .unwrap_or(null_hwnd());

    if is_hwnd_valid(shell_view) {
        // Found SHELLDLL_DefView, now get the WorkerW sibling
        let worker_class: Vec<u16> = "WorkerW\0".encode_utf16().collect();
        let worker_w = FindWindowExW(
            null_hwnd(),
            hwnd,
            PCWSTR::from_raw(worker_class.as_ptr()),
            PCWSTR::null(),
        )
        .unwrap_or(null_hwnd());

        if is_hwnd_valid(worker_w) {
            // Store the WorkerW handle
            let worker_w_ptr = lparam.0 as *mut HWND;
            *worker_w_ptr = worker_w;
            return BOOL(0); // Stop enumeration
        }
    }

    BOOL(1) // Continue enumeration
}

/// Refresh the desktop wallpaper to clear any ghost images
/// This is called when shutting down to clean up WorkerW remnants
fn refresh_desktop(verbose: bool) {
    unsafe {
        if verbose {
            println!("[INFO] Refreshing desktop wallpaper to clear remnants...");
        }
        // This triggers Windows to redraw the desktop wallpaper
        // Passing null for the wallpaper path causes Windows to refresh with the current wallpaper
        let _ = SystemParametersInfoW(SPI_SETDESKWALLPAPER, 0, None, SPIF_UPDATEINIFILE);
    }
}

/// Clean up all wallpaper windows before exit
///
/// This explicitly hides windows, detaches them from WorkerW, and destroys them
/// to prevent ghost window frames from remaining on screen after the process exits.
fn cleanup_windows(
    windows_and_webviews: &[(Window, WebView, HWND, i32, i32, u32, u32)],
    using_worker_w: bool,
    verbose: bool,
) {
    use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;
    use windows::Win32::UI::WindowsAndMessaging::ShowWindow;
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    for (_window, _, hwnd, _, _, _, _) in windows_and_webviews {
        unsafe {
            // 1. Hide the window immediately to prevent visual artifacts
            let _ = ShowWindow(*hwnd, SW_HIDE);

            // 2. Detach from WorkerW to prevent orphaned child window artifacts
            if using_worker_w {
                let _ = SetParent(*hwnd, null_hwnd());
            }

            // 3. Explicitly destroy the window
            let _ = DestroyWindow(*hwnd);
        }

        if verbose {
            println!("[INFO] Cleaned up window {:?}", hwnd);
        }
    }

    // 4. Refresh desktop wallpaper to clear any remaining WorkerW remnants
    if using_worker_w {
        refresh_desktop(verbose);
    }
}

/// Attach a window to WorkerW as a child window
///
/// This makes the window truly part of the desktop, immune to window managers.
fn attach_to_worker_w(hwnd: HWND, worker_w: HWND, verbose: bool) -> WallpaperResult<()> {
    unsafe {
        if verbose {
            println!("[INFO] Attaching window to WorkerW...");
        }

        // Set window style to WS_CHILD (required for SetParent)
        // Also remove any popup/overlapped styles
        let style = WS_CHILD.0 as i32;
        SetWindowLongW(hwnd, GWL_STYLE, style);

        if verbose {
            println!("[INFO] Set WS_CHILD style");
        }

        // Set extended styles for wallpaper behavior
        let ex_style = WS_EX_TOOLWINDOW.0      // Hide from taskbar
            | WS_EX_NOACTIVATE.0               // Never receive focus
            | WS_EX_TRANSPARENT.0              // Click-through
            | WS_EX_LAYERED.0; // Enable layered window
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style as i32);

        if verbose {
            println!(
                "[INFO] Applied extended styles (TOOLWINDOW, NOACTIVATE, TRANSPARENT, LAYERED)"
            );
        }

        // Set layered window attributes (nearly fully opaque)
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 254, LWA_ALPHA);

        // Attach to WorkerW using SetParent
        let result = SetParent(hwnd, worker_w).unwrap_or(null_hwnd());
        if !is_hwnd_valid(result) {
            return Err(WallpaperError::WindowCreationFailed(
                "SetParent to WorkerW failed".to_string(),
            ));
        }

        if verbose {
            println!("[INFO] Successfully attached to WorkerW");
        }

        Ok(())
    }
}

/// Create and run wallpaper windows for multiple displays with IPC support
///
/// This function blocks and runs the event loop. It should be called
/// from the main thread. All windows share the same event loop.
pub fn create_wallpapers(configs: Vec<WallpaperConfig>) -> WallpaperResult<()> {
    if configs.is_empty() {
        return Ok(());
    }

    // Dispatch based on render mode
    let render_mode = configs[0].render_mode.clone();
    match render_mode {
        RenderMode::NativeGpu => create_wallpapers_native_gpu(configs),
        RenderMode::WebView => create_wallpapers_webview(configs),
    }
}

/// Create wallpapers using native wgpu GPU rendering for shader files.
fn create_wallpapers_native_gpu(configs: Vec<WallpaperConfig>) -> WallpaperResult<()> {
    let verbose = configs[0].verbose;

    if verbose {
        println!(
            "[INFO] Creating native GPU wallpaper for {} display(s)",
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

    // Setup desktop layer (WorkerW technique)
    let desktop_layer = setup_desktop_layer(verbose);

    // Create event loop
    let event_loop = EventLoop::new();

    // Create windows and renderers for each display
    struct NativeWallpaperWindow {
        window: Window,
        renderer: NativeRenderer,
        hwnd: HWND,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        scale: f32,
    }

    let mut native_windows: Vec<NativeWallpaperWindow> = Vec::new();

    for config in &configs {
        let shader_source = config.shader_source.as_deref().ok_or_else(|| {
            WallpaperError::WindowCreationFailed("No shader source for NativeGpu mode".to_string())
        })?;

        if verbose {
            println!(
                "[INFO] Creating native GPU window for display {}",
                config.display.index
            );
            println!(
                "[INFO] Position: ({}, {}), Size: {}x{}",
                config.display.x, config.display.y, config.display.width, config.display.height
            );
        }

        // Create the tao window (no WebView needed)
        let window = WindowBuilder::new()
            .with_title(format!("WebWallpaper - Display {}", config.display.index))
            .with_position(PhysicalPosition::new(config.display.x, config.display.y))
            .with_inner_size(PhysicalSize::new(
                config.display.width,
                config.display.height,
            ))
            .with_decorations(false)
            .with_resizable(false)
            .with_always_on_top(false)
            .with_visible(false)
            .build(&event_loop)
            .map_err(|e| WallpaperError::WindowCreationFailed(e.to_string()))?;

        let hwnd = HWND(window.hwnd() as *mut std::ffi::c_void);

        // Build raw window/display handles for wgpu surface creation
        let mut win_handle = Win32WindowHandle::new(
            std::num::NonZero::new(window.hwnd() as isize)
                .expect("HWND should not be zero"),
        );
        let hinstance = unsafe {
            windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
                .expect("GetModuleHandleW failed")
        };
        win_handle.hinstance = std::num::NonZero::new(hinstance.0 as isize);
        let raw_window = RawWindowHandle::Win32(win_handle);
        let raw_display = RawDisplayHandle::Windows(WindowsDisplayHandle::new());

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

        native_windows.push(NativeWallpaperWindow {
            window,
            renderer,
            hwnd,
            x: config.display.x,
            y: config.display.y,
            width: config.display.width,
            height: config.display.height,
            scale: config.scale,
        });
    }

    // Attach to WorkerW and show windows
    for nw in &native_windows {
        if let Some(ref layer) = desktop_layer {
            if let Err(e) = attach_to_worker_w(nw.hwnd, layer.worker_w, verbose) {
                if verbose {
                    println!("[WARN] Failed to attach to WorkerW: {}", e);
                    println!("[INFO] Falling back to standard wallpaper styles...");
                }
                let _ = apply_wallpaper_styles(nw.hwnd, verbose);
            }
        } else {
            let _ = apply_wallpaper_styles(nw.hwnd, verbose);
        }

        unsafe {
            let _ = SetWindowPos(
                nw.hwnd,
                HWND_BOTTOM,
                nw.x,
                nw.y,
                nw.width as i32,
                nw.height as i32,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
        nw.window.set_visible(true);
    }

    if desktop_layer.is_none() {
        std::thread::sleep(Duration::from_millis(200));
        for nw in &native_windows {
            apply_z_order(nw.hwnd, false);
        }
    }

    if verbose {
        println!("[INFO] Native GPU wallpaper is now running. Press Ctrl+C to stop.");
    }

    let using_worker_w = desktop_layer.is_some();

    // Run the event loop — render on each frame
    event_loop.run(move |event, _, control_flow| {
        if shutdown_flag.load(Ordering::Relaxed) {
            if verbose {
                println!("[INFO] Shutdown flag detected, closing...");
            }
            // Clean up native windows
            for nw in &native_windows {
                unsafe {
                    use windows::Win32::UI::WindowsAndMessaging::{
                        DestroyWindow, ShowWindow, SW_HIDE,
                    };
                    let _ = ShowWindow(nw.hwnd, SW_HIDE);
                    if using_worker_w {
                        let _ = SetParent(nw.hwnd, null_hwnd());
                    }
                    let _ = DestroyWindow(nw.hwnd);
                }
            }
            if using_worker_w {
                refresh_desktop(verbose);
            }
            ipc_server.shutdown();
            *control_flow = ControlFlow::Exit;
            return;
        }

        *control_flow = ControlFlow::Poll;

        match event {
            Event::MainEventsCleared => {
                // Request redraw for all windows
                for nw in &native_windows {
                    nw.window.request_redraw();
                }
            }
            Event::RedrawRequested(_) => {
                // Render all windows on any redraw request. WorkerW child
                // windows may not receive individual RedrawRequested events,
                // so we render all of them together. Mailbox present mode
                // prevents VSync blocking from halving FPS on multi-monitor.
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
                // Find the renderer for this window and resize its surface
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
                event: WindowEvent::CloseRequested,
                ..
            }
            | Event::WindowEvent {
                event: WindowEvent::Destroyed,
                ..
            } => {
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

    // Setup desktop layer (WorkerW technique) for proper wallpaper embedding
    let desktop_layer = setup_desktop_layer(verbose);

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
                config.display.x, config.display.y, config.display.width, config.display.height
            );
        }

        // Build the window using full display dimensions (not work area)
        // CRITICAL: Create window as HIDDEN first to prevent window managers from capturing it
        // We'll show it after attaching to WorkerW
        let window = WindowBuilder::new()
            .with_title(format!("WebWallpaper - Display {}", config.display.index))
            .with_position(PhysicalPosition::new(config.display.x, config.display.y))
            .with_inner_size(PhysicalSize::new(
                config.display.width,
                config.display.height,
            ))
            .with_decorations(false)
            .with_resizable(false)
            .with_always_on_top(false)
            .with_visible(false) // Hidden initially!
            .build(&event_loop)
            .map_err(|e| WallpaperError::WindowCreationFailed(e.to_string()))?;

        // Get the HWND
        let hwnd = HWND(window.hwnd() as *mut std::ffi::c_void);

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
            .with_background_color((0, 0, 0, 255)) // Black background
            .build()
            .map_err(|e| WallpaperError::WindowCreationFailed(format!("WebView error: {}", e)))?;

        if verbose {
            println!(
                "[INFO] WebView created for display {}",
                config.display.index
            );
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
            "[INFO] All {} wallpaper window(s) created (hidden)",
            windows_and_webviews.len()
        );
        println!("[INFO] Attaching to desktop and showing windows...");
    }

    // Short delay for WebView initialization
    std::thread::sleep(Duration::from_millis(500));

    // Attach to WorkerW BEFORE showing windows - this prevents komorebi from ever seeing them
    for (window, _, hwnd, x, y, width, height) in &windows_and_webviews {
        // Apply wallpaper styles based on whether we have WorkerW
        if let Some(ref layer) = desktop_layer {
            // Use WorkerW technique - attach as child of WorkerW
            if let Err(e) = attach_to_worker_w(*hwnd, layer.worker_w, verbose) {
                if verbose {
                    println!("[WARN] Failed to attach to WorkerW: {}", e);
                    println!("[INFO] Falling back to standard wallpaper styles...");
                }
                // Fallback to standard styles
                let _ = apply_wallpaper_styles(*hwnd, verbose);
            }
        } else {
            // No WorkerW available, use standard styles
            if let Err(e) = apply_wallpaper_styles(*hwnd, verbose) {
                if verbose {
                    println!("[WARN] Failed to apply wallpaper styles: {}", e);
                }
            }
        }

        // Set position and size, then show the window
        // Offset by -1 to hide the 1-pixel WebView2 border on top/left edges
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

        // Also use tao's set_visible to ensure proper state
        window.set_visible(true);
    }

    // Apply Z-order again after a short delay (only if not using WorkerW)
    if desktop_layer.is_none() {
        std::thread::sleep(Duration::from_millis(200));
        for (_, _, hwnd, _, _, _, _) in &windows_and_webviews {
            apply_z_order(*hwnd, false);
        }
    }

    if verbose {
        println!("[INFO] Wallpaper is now running. Press Ctrl+C to stop.");
    }

    // Track if we're using WorkerW for cleanup
    let using_worker_w = desktop_layer.is_some();
    let mut cleaned_up = false;

    // Run the event loop with polling to check shutdown flag
    event_loop.run(move |event, _, control_flow| {
        // Check shutdown flag periodically
        if shutdown_flag.load(Ordering::Relaxed) {
            if verbose {
                println!("[INFO] Shutdown flag detected, closing...");
            }

            if !cleaned_up {
                cleanup_windows(&windows_and_webviews, using_worker_w, verbose);
                cleaned_up = true;
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
                if !cleaned_up {
                    cleanup_windows(&windows_and_webviews, using_worker_w, verbose);
                    cleaned_up = true;
                }
                ipc_server.shutdown();
                *control_flow = ControlFlow::Exit;
            }
            Event::LoopDestroyed => {
                // Final cleanup opportunity - ensures no ghost windows remain
                if !cleaned_up {
                    cleanup_windows(&windows_and_webviews, using_worker_w, verbose);
                    cleaned_up = true;
                }
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
        )
        .unwrap_or(null_hwnd());

        if is_hwnd_valid(desktop_hwnd) {
            if verbose {
                println!(
                    "[INFO] Found desktop window (Progman), positioning after it in Z-order..."
                );
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
