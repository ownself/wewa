//! WebWallpaper - Display web content as desktop wallpaper
//!
//! A cross-platform CLI tool that renders web content (URLs or local HTML files)
//! as desktop wallpaper, supporting multiple monitors and instance management.

mod cli;
mod config;
mod display;
mod ipc;
mod platform;
mod server;
mod shader;
mod wallpaper;

use cli::{CliArgs, CommandMode};
use config::{Config, WallpaperInstance};
use server::LocalServer;
use std::path::Path;
use wallpaper::WallpaperConfig;

/// Exit codes as defined in contracts/cli.md
mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const GENERAL_ERROR: i32 = 1;
    pub const DISPLAY_NOT_FOUND: i32 = 2;
    pub const NO_RUNNING_INSTANCE: i32 = 3;
    pub const WEBVIEW_NOT_AVAILABLE: i32 = 4;
    pub const SERVER_STARTUP_FAILED: i32 = 5;
}

fn main() {
    // Initialize platform (DPI awareness, etc.)
    if let Err(e) = platform::init_platform() {
        eprintln!("[WARN] Platform initialization warning: {}", e);
    }

    // Parse CLI arguments
    let args = CliArgs::parse_args();
    let config = Config::default();

    // Handle verbose mode
    if args.verbose {
        println!("[INFO] WebWallpaper v{}", env!("CARGO_PKG_VERSION"));
        println!("[INFO] Instance directory: {:?}", config.instance_dir);
    }

    // Dispatch based on command mode
    let exit_code = match args.mode() {
        CommandMode::Start {
            url_or_path,
            display,
            port,
            scale,
        } => handle_start(&config, &url_or_path, display, port, scale, args.verbose),

        CommandMode::Stop(display_index) => handle_stop(&config, display_index, args.verbose),

        CommandMode::StopAll => handle_stop_all(&config, args.verbose),

        CommandMode::ShowHelp => {
            // clap will show help automatically, but if we get here, show usage
            eprintln!("Usage: webwallpaper [OPTIONS] [URL_OR_PATH]");
            eprintln!("       webwallpaper --stop <DISPLAY>");
            eprintln!("       webwallpaper --stopall");
            eprintln!();
            eprintln!("Run 'webwallpaper --help' for more information.");
            exit_codes::GENERAL_ERROR
        }
    };

    std::process::exit(exit_code);
}

/// Determine if the input is a URL or a local file path
fn is_url(input: &str) -> bool {
    input.starts_with("http://") || input.starts_with("https://") || input.starts_with("file://")
}

/// Transform special URLs for better wallpaper experience
/// Currently supports:
/// - ShaderToy: converts view URLs to fullscreen embed URLs
fn transform_url(url: &str, verbose: bool) -> String {
    // ShaderToy: https://www.shadertoy.com/view/XXXXX -> embed format
    if url.contains("shadertoy.com/view/") {
        // Extract shader ID from URL
        if let Some(id_start) = url.find("/view/") {
            let id_part = &url[id_start + 6..];
            // Extract just the ID (stop at ? or end)
            let shader_id = id_part
                .split(&['?', '#', '/'][..])
                .next()
                .unwrap_or(id_part);

            if !shader_id.is_empty() {
                let embed_url = format!(
                    "https://www.shadertoy.com/embed/{}?gui=false&t=0&paused=false&muted=true",
                    shader_id
                );
                if verbose {
                    println!("[INFO] Transformed ShaderToy URL to embed format:");
                    println!("[INFO]   Original: {}", url);
                    println!("[INFO]   Embed: {}", embed_url);
                }
                return embed_url;
            }
        }
    }

    // No transformation needed
    url.to_string()
}

/// Strip Windows extended path prefix (\\?\) from a path
/// This prefix is added by canonicalize() on Windows but can cause issues
#[cfg(target_os = "windows")]
fn strip_windows_prefix(path: &Path) -> std::path::PathBuf {
    let path_str = path.to_string_lossy();
    if path_str.starts_with(r"\\?\") {
        std::path::PathBuf::from(&path_str[4..])
    } else {
        path.to_path_buf()
    }
}

#[cfg(not(target_os = "windows"))]
fn strip_windows_prefix(path: &Path) -> std::path::PathBuf {
    path.to_path_buf()
}

/// Resolve a local path to an absolute path and get the directory and filename
fn resolve_local_path(path_str: &str) -> Result<(std::path::PathBuf, String), String> {
    let path = Path::new(path_str);

    // Check if file exists
    if !path.exists() {
        return Err(format!("File not found: {}", path_str));
    }

    // Get absolute path
    let abs_path = path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve path: {}", e))?;

    // Strip Windows extended path prefix (\\?\) if present
    // This prefix can cause issues with path operations in some contexts
    let abs_path = strip_windows_prefix(&abs_path);

    if abs_path.is_file() {
        let parent = abs_path
            .parent()
            .ok_or_else(|| "Cannot get parent directory".to_string())?;
        let filename = abs_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| "Invalid filename".to_string())?
            .to_string();
        Ok((parent.to_path_buf(), filename))
    } else if abs_path.is_dir() {
        // If it's a directory, look for index.html
        let index_path = abs_path.join("index.html");
        if index_path.exists() {
            Ok((abs_path, "index.html".to_string()))
        } else {
            Err(format!(
                "Directory does not contain index.html: {}",
                path_str
            ))
        }
    } else {
        Err(format!("Path is neither file nor directory: {}", path_str))
    }
}

/// Handle the start command
fn handle_start(
    config: &Config,
    url_or_path: &str,
    display: Option<u32>,
    port: u16,
    scale: f32,
    verbose: bool,
) -> i32 {
    if verbose {
        println!("[INFO] Starting wallpaper...");
        println!("[INFO] URL/Path: {}", url_or_path);
        println!("[INFO] Display: {:?}", display);
        println!("[INFO] Port: {}", port);
        println!("[INFO] Scale: {:.2}", scale);
    }

    let scale = match shader::validate_scale(scale) {
        Ok(scale) => scale,
        Err(e) => {
            eprintln!("error: {}", e);
            return exit_codes::GENERAL_ERROR;
        }
    };

    // Enumerate displays
    let displays: Vec<crate::display::Display> = match platform::enumerate_displays() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: Failed to enumerate displays: {}", e);
            return exit_codes::GENERAL_ERROR;
        }
    };

    if verbose {
        platform::print_display_info(&displays);
    }

    // Check platform runtime availability before proceeding
    if let Err(e) = platform::ensure_runtime_available() {
        eprintln!("error: {}", e);
        return exit_codes::WEBVIEW_NOT_AVAILABLE;
    }

    // Determine target displays
    let target_displays: Vec<crate::display::Display> = if let Some(index) = display {
        // Specific display requested
        match crate::display::find_display_by_index(&displays, index) {
            Some(d) => vec![d.clone()],
            None => {
                let available: Vec<String> = displays.iter().map(|d| d.index.to_string()).collect();
                eprintln!(
                    "error: Display {} does not exist (available: {})",
                    index,
                    available.join(", ")
                );
                return exit_codes::DISPLAY_NOT_FOUND;
            }
        }
    } else {
        // No display specified - apply to ALL displays
        if displays.is_empty() {
            eprintln!("error: No displays found");
            return exit_codes::GENERAL_ERROR;
        }
        if verbose {
            println!(
                "[INFO] No --display specified, applying to all {} display(s)",
                displays.len()
            );
        }
        displays.clone()
    };

    // Check for existing instances and stop them (instance replacement)
    for target_display in &target_displays {
        let instance_path = config.instance_file_path(target_display.index);
        if instance_path.exists() {
            if verbose {
                println!(
                    "[INFO] Existing wallpaper on display {}, stopping it first...",
                    target_display.index
                );
            }
            // Try to stop via IPC
            let _ = ipc::IpcClient::stop_display(target_display.index);
            // Clean up instance file regardless
            let _ = WallpaperInstance::delete(config, target_display.index);
            // Small delay to allow cleanup
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }

    // Ensure instance directory exists
    if let Err(e) = config.ensure_instance_dir() {
        eprintln!("error: Failed to create instance directory: {}", e);
        return exit_codes::GENERAL_ERROR;
    }

    // Determine the URL to load
    let mut shader_bundle: Option<shader::ShaderBundle> = None;
    let (url, server): (String, Option<LocalServer>) = if is_url(url_or_path) {
        // It's already a URL - apply transformations for special sites
        if verbose {
            println!("[INFO] Input is a URL");
        }
        let transformed = transform_url(url_or_path, verbose);
        (transformed, None)
    } else {
        // It's a local file path - need to start HTTP server
        if verbose {
            println!("[INFO] Input is a local path, starting HTTP server...");
        }

        let local_path = Path::new(url_or_path);
        let (root_dir, filename) = if shader::is_shader_file(local_path) {
            if verbose {
                println!("[INFO] Detected .shader input, generating temporary HTML runtime...");
            }

            let bundle = match shader::create_shader_bundle(local_path, scale) {
                Ok(bundle) => bundle,
                Err(e) => {
                    eprintln!("error: {}", e);
                    return exit_codes::GENERAL_ERROR;
                }
            };

            let root_dir = bundle.root_dir.clone();
            let entry_file = bundle.entry_file.clone();
            shader_bundle = Some(bundle);
            (root_dir, entry_file)
        } else {
            match resolve_local_path(url_or_path) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("error: {}", e);
                    return exit_codes::GENERAL_ERROR;
                }
            }
        };

        if verbose {
            println!("[INFO] Serving files from: {:?}", root_dir);
            println!("[INFO] Entry file: {}", filename);
        }

        // Check if port is available
        if !LocalServer::is_port_available(port) {
            eprintln!("error: Port {} is already in use", port);
            eprintln!("hint: Use --port to specify a different port");
            return exit_codes::SERVER_STARTUP_FAILED;
        }

        // Start the server
        let mut local_server = LocalServer::new(root_dir, port);
        if let Err(e) = local_server.start() {
            eprintln!("error: Failed to start HTTP server: {}", e);
            return exit_codes::SERVER_STARTUP_FAILED;
        }

        let url = local_server.file_url(&filename);
        if verbose {
            println!("[INFO] HTTP server started at {}", local_server.url());
            println!("[INFO] Loading: {}", url);
        }

        (url, Some(local_server))
    };

    // Write instance files for all target displays
    let server_port = server.as_ref().map(|_| port);
    let display_indices: Vec<u32> = target_displays.iter().map(|d| d.index).collect();

    for target_display in &target_displays {
        let instance = WallpaperInstance::new(target_display.index, url.clone(), server_port);

        if let Err(e) = instance.save(config) {
            eprintln!(
                "[WARN] Failed to save instance file for display {}: {}",
                target_display.index, e
            );
        } else if verbose {
            println!(
                "[INFO] Instance file saved: {:?}",
                config.instance_file_path(target_display.index)
            );
        }
    }

    // Create wallpaper configurations for all displays
    let wallpaper_configs: Vec<WallpaperConfig> = target_displays
        .iter()
        .map(|d| WallpaperConfig::new(url.clone(), d.clone(), verbose))
        .collect();

    // Print status message
    if target_displays.len() == 1 {
        println!(
            "Started wallpaper on display {}: {}",
            target_displays[0].index, url_or_path
        );
    } else {
        let display_list: Vec<String> = target_displays
            .iter()
            .map(|d| d.index.to_string())
            .collect();
        println!(
            "Started wallpaper on {} display(s) [{}]: {}",
            target_displays.len(),
            display_list.join(", "),
            url_or_path
        );
    }

    // Create and run the wallpapers (this blocks)
    if let Err(e) = platform::create_wallpapers(wallpaper_configs) {
        eprintln!("error: Wallpaper creation failed: {}", e);

        // Clean up instance files
        for index in &display_indices {
            let _ = WallpaperInstance::delete(config, *index);
        }

        if let Some(bundle) = shader_bundle.as_ref() {
            shader::cleanup_shader_bundle(bundle);
        }

        return exit_codes::GENERAL_ERROR;
    }

    // Clean up on exit
    if let Some(s) = server.as_ref() {
        s.shutdown();
    }
    if let Some(bundle) = shader_bundle.as_ref() {
        shader::cleanup_shader_bundle(bundle);
    }
    for index in &display_indices {
        let _ = WallpaperInstance::delete(config, *index);
    }

    exit_codes::SUCCESS
}

/// Handle the stop command for a specific display
fn handle_stop(config: &Config, display_index: u32, verbose: bool) -> i32 {
    if verbose {
        println!("[INFO] Stopping wallpaper on display {}...", display_index);
    }

    // Check if instance file exists
    let instance_path = config.instance_file_path(display_index);
    if !instance_path.exists() {
        eprintln!("error: No wallpaper running on display {}", display_index);
        return exit_codes::NO_RUNNING_INSTANCE;
    }

    // Send stop command via IPC
    match ipc::IpcClient::stop_display(display_index) {
        Ok(response) => {
            if verbose {
                println!("[INFO] IPC response: {:?}", response);
            }
            // Clean up instance file
            let _ = WallpaperInstance::delete(config, display_index);
            println!("Stopped wallpaper on display {}", display_index);
            exit_codes::SUCCESS
        }
        Err(e) => {
            if verbose {
                println!("[WARN] IPC failed: {}", e);
            }
            // If IPC fails, the process might have crashed - clean up the stale instance file
            let _ = WallpaperInstance::delete(config, display_index);
            eprintln!(
                "error: Failed to stop wallpaper on display {} (process may have already exited)",
                display_index
            );
            exit_codes::GENERAL_ERROR
        }
    }
}

/// Handle the stopall command
fn handle_stop_all(config: &Config, verbose: bool) -> i32 {
    if verbose {
        println!("[INFO] Stopping all wallpapers...");
    }

    // List all instance files
    let instances = match config.list_instance_files() {
        Ok(files) => files,
        Err(e) => {
            eprintln!("error: Failed to list instances: {}", e);
            return exit_codes::GENERAL_ERROR;
        }
    };

    if instances.is_empty() {
        println!("No wallpaper instances running");
        return exit_codes::SUCCESS;
    }

    let instance_count = instances.len();

    // Send stop all command via IPC
    match ipc::IpcClient::stop_all() {
        Ok(response) => {
            if verbose {
                println!("[INFO] IPC response: {:?}", response);
            }
        }
        Err(e) => {
            if verbose {
                println!("[WARN] IPC failed: {}", e);
            }
        }
    }

    // Clean up all instance files regardless of IPC result
    // (process might have crashed or IPC might fail for other reasons)
    for instance_path in &instances {
        if let Some(filename) = instance_path.file_stem().and_then(|s| s.to_str()) {
            if let Some(index_str) = filename.strip_prefix("display_") {
                if let Ok(index) = index_str.parse::<u32>() {
                    let _ = WallpaperInstance::delete(config, index);
                }
            }
        }
    }

    println!("Stopped {} wallpaper instance(s)", instance_count);
    exit_codes::SUCCESS
}
