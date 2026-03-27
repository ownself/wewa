//! Linux Wayland wallpaper implementation using GTK, WebKitGTK, and layer-shell.

use super::ensure_runtime_available;
use crate::ipc::{IpcCommand, IpcServer};
use crate::wallpaper::{RenderMode, WallpaperConfig, WallpaperError, WallpaperResult};
use gdk::prelude::*;
use glib::{source::timeout_add_local, ControlFlow};
use gtk::prelude::*;
use gtk::{Window, WindowType};
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Duration;
use wry::{WebView, WebViewBuilder, WebViewBuilderExtUnix};

const DISABLE_POINTER_REACTIONS_SCRIPT: &str = r#"
(() => {
  const style = document.createElement('style');
  style.id = '__webwallpaper_disable_pointer_reactions';
  style.textContent = `
    html, body, body * {
      cursor: default !important;
      pointer-events: none !important;
    }
  `;

  const install = () => {
    if (!document.head) {
      return;
    }
    if (!document.getElementById(style.id)) {
      document.head.appendChild(style);
    }
  };

  const swallow = (event) => {
    event.preventDefault();
    event.stopImmediatePropagation();
    event.stopPropagation();
  };

  [
    'pointerdown', 'pointerup', 'pointermove', 'pointerenter', 'pointerleave',
    'pointerover', 'pointerout', 'mousedown', 'mouseup', 'mousemove',
    'mouseenter', 'mouseleave', 'mouseover', 'mouseout', 'click',
    'dblclick', 'contextmenu', 'wheel'
  ].forEach((name) => {
    window.addEventListener(name, swallow, { capture: true, passive: false });
    document.addEventListener(name, swallow, { capture: true, passive: false });
  });

  install();
  document.addEventListener('DOMContentLoaded', install, { once: true });
})();
"#;

#[derive(Clone)]
struct ManagedWindow {
    display_index: u32,
    window: Window,
    #[allow(dead_code)]
    webview: Rc<WebView>,
}

/// Create and run wallpaper windows for multiple displays with IPC support.
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

/// Create wallpapers using native wgpu GPU rendering for shader files on Linux.
fn create_wallpapers_native_gpu(configs: Vec<WallpaperConfig>) -> WallpaperResult<()> {
    use crate::renderer::NativeRenderer;
    use raw_window_handle::{
        RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
        XlibDisplayHandle, XlibWindowHandle,
    };

    super::ensure_gtk_ready().map_err(|e| WallpaperError::PlatformError(e.to_string()))?;

    let verbose = configs[0].verbose;
    let gdk_display = gdk::Display::default().ok_or_else(|| {
        WallpaperError::PlatformError("No active GDK display available".to_string())
    })?;

    if verbose {
        println!(
            "[INFO] Creating native GPU wallpaper for {} display(s) on Linux",
            configs.len()
        );
    }

    let shutdown_flag = Arc::new(AtomicBool::new(false));

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

    let ctrlc_shutdown = shutdown_flag.clone();
    let ctrlc_verbose = verbose;
    let _ = ctrlc::set_handler(move || {
        if ctrlc_verbose {
            println!("\n[INFO] Ctrl+C received, initiating shutdown...");
        }
        ctrlc_shutdown.store(true, Ordering::Relaxed);
    });

    struct NativeWallpaperWindow {
        window: Window,
        renderer: NativeRenderer,
    }

    let mut native_windows: Vec<NativeWallpaperWindow> = Vec::new();

    for config in &configs {
        let shader_source = config.shader_source.as_deref().ok_or_else(|| {
            WallpaperError::WindowCreationFailed("No shader source for NativeGpu mode".to_string())
        })?;

        let monitor = monitor_for_display(&gdk_display, &config.display).ok_or_else(|| {
            WallpaperError::PlatformError(format!(
                "Could not resolve monitor for display {}",
                config.display.index
            ))
        })?;

        let window = Window::new(WindowType::Toplevel);
        window.set_title("WebWallpaper");
        window.set_decorated(false);
        window.set_resizable(false);
        window.set_accept_focus(false);
        window.set_focus_on_map(false);
        window.set_skip_taskbar_hint(true);
        window.set_skip_pager_hint(true);
        window.stick();
        window.set_default_size(config.display.width as i32, config.display.height as i32);

        window.init_layer_shell();
        window.set_namespace("webwallpaper");
        window.set_layer(Layer::Background);
        window.set_keyboard_mode(KeyboardMode::None);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_monitor(&monitor);
        window.set_exclusive_zone(-1);

        // Must realize the window to get a valid GDK window for handle extraction
        window.show_all();

        // Extract raw window/display handles for wgpu
        let gdk_window = window.window().ok_or_else(|| {
            WallpaperError::WindowCreationFailed("No GDK window after realize".to_string())
        })?;

        let gdk_display_ref = gdk_window.display();

        // Determine if we're on Wayland or X11
        let (raw_window, raw_display) = unsafe {
            if gdk_display_ref.type_().name() == "GdkWaylandDisplay" {
                use gdk_sys::{
                    gdk_wayland_display_get_wl_display, gdk_wayland_window_get_wl_surface,
                };
                use glib::translate::ToGlibPtr;

                let wl_surface =
                    gdk_wayland_window_get_wl_surface(gdk_window.to_glib_none().0 as *mut _);
                let wl_display =
                    gdk_wayland_display_get_wl_display(gdk_display_ref.to_glib_none().0 as *mut _);

                let raw_window = RawWindowHandle::Wayland(WaylandWindowHandle::new(
                    std::num::NonZero::new(wl_surface as isize).expect("wl_surface is null"),
                ));
                let raw_display = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
                    std::num::NonZero::new(wl_display as isize).expect("wl_display is null"),
                ));
                (raw_window, raw_display)
            } else {
                // X11 fallback
                use gdk_sys::{gdk_x11_display_get_xdisplay, gdk_x11_window_get_xid};
                use glib::translate::ToGlibPtr;

                let xid = gdk_x11_window_get_xid(gdk_window.to_glib_none().0 as *mut _);
                let x_display =
                    gdk_x11_display_get_xdisplay(gdk_display_ref.to_glib_none().0 as *mut _);

                let raw_window = RawWindowHandle::Xlib(XlibWindowHandle::new(xid as u64));
                let mut raw_display_handle = XlibDisplayHandle::new(
                    std::num::NonZero::new(x_display as isize),
                    0,
                );
                let raw_display = RawDisplayHandle::Xlib(raw_display_handle);
                (raw_window, raw_display)
            }
        };

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
        .map_err(|e| {
            WallpaperError::WindowCreationFailed(format!("GPU renderer error: {}", e))
        })?;

        if verbose {
            println!(
                "[INFO] Native GPU renderer created for display {}",
                config.display.index
            );
        }

        native_windows.push(NativeWallpaperWindow { window, renderer });
    }

    // Use a GLib timer for the render loop (~16ms for ~60fps)
    let native_windows = Rc::new(RefCell::new(native_windows));
    let render_windows = Rc::clone(&native_windows);
    let render_shutdown = shutdown_flag.clone();

    timeout_add_local(Duration::from_millis(16), move || {
        if render_shutdown.load(Ordering::Relaxed) {
            let windows = render_windows.borrow();
            for nw in windows.iter() {
                nw.window.close();
            }
            gtk::main_quit();
            return ControlFlow::Break;
        }

        if let Some(receiver) = ipc_rx.as_ref() {
            while let Ok(command) = receiver.try_recv() {
                match command {
                    IpcCommand::Stop(_) | IpcCommand::StopAll => {
                        render_shutdown.store(true, Ordering::Relaxed);
                    }
                    IpcCommand::Ping => {}
                }
            }
        }

        // Render a frame for each window
        let mut windows = render_windows.borrow_mut();
        for nw in windows.iter_mut() {
            if let Err(e) = nw.renderer.render_frame(0.0, 0.0, 0.0, 0.0) {
                eprintln!("[ERROR] Render error: {}", e);
                render_shutdown.store(true, Ordering::Relaxed);
                return ControlFlow::Break;
            }
        }

        ControlFlow::Continue
    });

    if verbose {
        println!("[INFO] Native GPU wallpaper is now running. Press Ctrl+C to stop.");
    }

    gtk::main();
    Ok(())
}

/// Create wallpapers using WebView rendering for URLs and HTML files.
fn create_wallpapers_webview(configs: Vec<WallpaperConfig>) -> WallpaperResult<()> {
    ensure_runtime_available().map_err(|e| WallpaperError::PlatformError(e.to_string()))?;

    let verbose = configs.first().map(|c| c.verbose).unwrap_or(false);
    let gdk_display = gdk::Display::default().ok_or_else(|| {
        WallpaperError::PlatformError("No active GDK display available".to_string())
    })?;

    let shutdown_flag = Arc::new(AtomicBool::new(false));

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

    let ctrlc_shutdown = shutdown_flag.clone();
    let ctrlc_verbose = verbose;
    let _ = ctrlc::set_handler(move || {
        if ctrlc_verbose {
            println!("\n[INFO] Ctrl+C received, initiating shutdown...");
        }
        ctrlc_shutdown.store(true, Ordering::Relaxed);
    });

    let windows = Rc::new(RefCell::new(Vec::<ManagedWindow>::new()));

    for config in configs {
        let window = create_wallpaper_window(&gdk_display, &config)?;
        windows.borrow_mut().push(ManagedWindow {
            display_index: config.display.index,
            window: window.0,
            webview: Rc::new(window.1),
        });
    }

    let windows_ref = Rc::clone(&windows);
    let timeout_shutdown = shutdown_flag.clone();
    timeout_add_local(Duration::from_millis(100), move || {
        if timeout_shutdown.load(Ordering::Relaxed) {
            destroy_all(&windows_ref);
            gtk::main_quit();
            return ControlFlow::Break;
        }

        if let Some(receiver) = ipc_rx.as_ref() {
            process_pending_commands(receiver, &windows_ref, verbose);
        }

        if windows_ref.borrow().is_empty() {
            gtk::main_quit();
            return ControlFlow::Break;
        }

        ControlFlow::Continue
    });

    if verbose {
        println!("[INFO] Entering GTK main loop");
    }

    gtk::main();
    Ok(())
}

fn create_wallpaper_window(
    gdk_display: &gdk::Display,
    config: &WallpaperConfig,
) -> WallpaperResult<(Window, WebView)> {
    let monitor = monitor_for_display(gdk_display, &config.display).ok_or_else(|| {
        WallpaperError::PlatformError(format!(
            "Could not resolve monitor for display {}",
            config.display.index
        ))
    })?;

    let window = Window::new(WindowType::Toplevel);
    window.set_title("WebWallpaper");
    window.set_decorated(false);
    window.set_resizable(false);
    window.set_accept_focus(false);
    window.set_focus_on_map(false);
    window.set_skip_taskbar_hint(true);
    window.set_skip_pager_hint(true);
    window.stick();
    window.set_default_size(config.display.width as i32, config.display.height as i32);

    window.init_layer_shell();
    window.set_namespace("webwallpaper");
    window.set_layer(Layer::Background);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Bottom, true);
    window.set_monitor(&monitor);
    // Background surfaces should cover the full output even when panels like Waybar
    // claim exclusive zones; -1 asks the compositor to keep it fullscreen.
    window.set_exclusive_zone(-1);

    let webview = WebViewBuilder::new_gtk(&window)
        .with_initialization_script(DISABLE_POINTER_REACTIONS_SCRIPT)
        .with_url(&config.url)
        .with_devtools(false)
        .with_background_color((0, 0, 0, 255))
        .build()
        .map_err(|e| WallpaperError::WindowCreationFailed(format!("WebView error: {}", e)))?;

    window.show_all();

    Ok((window, webview))
}

fn monitor_for_display(
    gdk_display: &gdk::Display,
    target: &crate::display::Display,
) -> Option<gdk::Monitor> {
    if let Some(monitor) = gdk_display.monitor(target.index as i32) {
        return Some(monitor);
    }

    for index in 0..gdk_display.n_monitors() {
        if let Some(monitor) = gdk_display.monitor(index) {
            let geometry = monitor.geometry();
            if geometry.x() == target.x
                && geometry.y() == target.y
                && geometry.width() as u32 == target.width
                && geometry.height() as u32 == target.height
            {
                return Some(monitor);
            }
        }
    }

    gdk_display.primary_monitor()
}

fn process_pending_commands(
    receiver: &Receiver<IpcCommand>,
    windows: &Rc<RefCell<Vec<ManagedWindow>>>,
    verbose: bool,
) {
    while let Ok(command) = receiver.try_recv() {
        match command {
            IpcCommand::Ping => {}
            IpcCommand::StopAll => {
                if verbose {
                    println!("[INFO] Received STOP:ALL");
                }
                destroy_all(windows);
                break;
            }
            IpcCommand::Stop(display_index) => {
                if verbose {
                    println!("[INFO] Received STOP:{}.", display_index);
                }
                destroy_display_window(windows, display_index);
            }
        }
    }
}

fn destroy_display_window(windows: &Rc<RefCell<Vec<ManagedWindow>>>, display_index: u32) {
    let mut windows_mut = windows.borrow_mut();
    if let Some(position) = windows_mut
        .iter()
        .position(|managed| managed.display_index == display_index)
    {
        let managed = windows_mut.remove(position);
        managed.window.close();
    }
}

fn destroy_all(windows: &Rc<RefCell<Vec<ManagedWindow>>>) {
    let mut windows_mut = windows.borrow_mut();
    let existing = std::mem::take(&mut *windows_mut);
    drop(windows_mut);

    for managed in existing {
        managed.window.close();
    }
}
