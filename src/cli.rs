//! CLI argument parsing using clap derive
//!
//! Defines the command-line interface for wewa.

use crate::video::VideoFit;
use clap::Parser;

/// Display web content as desktop wallpaper
#[derive(Parser, Debug)]
#[command(name = "wewa")]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// URL or local file path to display as wallpaper
    #[arg(value_name = "URL_OR_PATH")]
    pub url_or_path: Option<String>,

    /// Target display index (0-based). If not specified, applies to all displays.
    #[arg(short = 'd', long)]
    pub display: Option<u32>,

    /// Stop wallpaper on the specified display
    #[arg(long, value_name = "DISPLAY", conflicts_with_all = ["url_or_path", "stopall"])]
    pub stop: Option<u32>,

    /// Stop all running wallpaper instances
    #[arg(long, visible_alias = "sa", conflicts_with_all = ["url_or_path", "stop"])]
    pub stopall: bool,

    /// HTTP server port for serving local files (default: 8080)
    #[arg(short = 'p', long, default_value = "8080")]
    pub port: u16,

    /// Use a built-in shader by name (use "list" to see available shaders)
    #[arg(short = 'b', long, value_name = "NAME", conflicts_with = "url_or_path")]
    pub builtin: Option<String>,

    /// Use a random built-in shader
    #[arg(short = 'r', long, conflicts_with_all = ["url_or_path", "builtin"])]
    pub random: bool,

    /// Shader render scale relative to CSS/logical dimensions, without a DPI multiplier (0.1-2.0; default: 1.0 for files, preset for built-ins)
    #[arg(short = 's', long)]
    pub scale: Option<f32>,

    /// Shader time scale for .shader inputs (default: 1.0)
    #[arg(long, visible_alias = "ts")]
    pub time_scale: Option<f32>,

    /// Texture file for iChannel0 (2D image or 3D volume with .bin extension)
    #[arg(long, visible_alias = "c0")]
    pub channel0: Option<String>,

    /// Texture file for iChannel1
    #[arg(long, visible_alias = "c1")]
    pub channel1: Option<String>,

    /// Texture file for iChannel2
    #[arg(long, visible_alias = "c2")]
    pub channel2: Option<String>,

    /// Texture file for iChannel3
    #[arg(long, visible_alias = "c3")]
    pub channel3: Option<String>,

    /// Local video sizing: contain (default) keeps black bars, cover crops to fill the display
    #[arg(long, value_enum, value_name = "MODE", requires = "url_or_path", conflicts_with_all = ["builtin", "random", "stop", "stopall"])]
    pub video_fit: Option<VideoFit>,

    /// Enable verbose output
    #[arg(short = 'v', long)]
    pub verbose: bool,
}

impl CliArgs {
    /// Parse command-line arguments
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Determine the command mode based on parsed arguments
    pub fn mode(&self) -> CommandMode {
        if self.stopall {
            CommandMode::StopAll
        } else if let Some(display) = self.stop {
            CommandMode::Stop(display)
        } else if self.random {
            let names = crate::builtin::list_builtins();
            let idx = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as usize
                % names.len();
            CommandMode::BuiltIn {
                name: names[idx].clone(),
                display: self.display,
                port: self.port,
                scale: self.scale,
                time_scale: self.time_scale,
                channels: [
                    self.channel0.clone(),
                    self.channel1.clone(),
                    self.channel2.clone(),
                    self.channel3.clone(),
                ],
            }
        } else if let Some(ref name) = self.builtin {
            CommandMode::BuiltIn {
                name: name.clone(),
                display: self.display,
                port: self.port,
                scale: self.scale,
                time_scale: self.time_scale,
                channels: [
                    self.channel0.clone(),
                    self.channel1.clone(),
                    self.channel2.clone(),
                    self.channel3.clone(),
                ],
            }
        } else if let Some(ref url_or_path) = self.url_or_path {
            CommandMode::Start {
                url_or_path: url_or_path.clone(),
                display: self.display,
                port: self.port,
                scale: self.scale.unwrap_or(1.0),
                time_scale: self.time_scale.unwrap_or(1.0),
                video_fit: self.video_fit,
                channels: [
                    self.channel0.clone(),
                    self.channel1.clone(),
                    self.channel2.clone(),
                    self.channel3.clone(),
                ],
            }
        } else {
            CommandMode::ShowHelp
        }
    }
}

/// The operational mode determined from CLI arguments
#[derive(Debug, Clone)]
pub enum CommandMode {
    /// Start wallpaper with given URL/path
    Start {
        url_or_path: String,
        display: Option<u32>,
        port: u16,
        scale: f32,
        time_scale: f32,
        video_fit: Option<VideoFit>,
        channels: [Option<String>; 4],
    },
    /// Start a built-in shader by name
    BuiltIn {
        name: String,
        display: Option<u32>,
        port: u16,
        scale: Option<f32>,
        time_scale: Option<f32>,
        channels: [Option<String>; 4],
    },
    /// Stop wallpaper on specific display
    Stop(u32),
    /// Stop all wallpaper instances
    StopAll,
    /// No valid command - show help
    ShowHelp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url() {
        let args = CliArgs::parse_from(["wewa", "https://example.com"]);
        assert_eq!(args.url_or_path, Some("https://example.com".to_string()));
        assert!(!args.stopall);
        assert!(args.stop.is_none());
    }

    #[test]
    fn test_parse_stop() {
        let args = CliArgs::parse_from(["wewa", "--stop", "0"]);
        assert_eq!(args.stop, Some(0));
        assert!(args.url_or_path.is_none());
    }

    #[test]
    fn test_parse_stopall() {
        let args = CliArgs::parse_from(["wewa", "--stopall"]);
        assert!(args.stopall);
    }

    #[test]
    fn test_parse_display_flag() {
        let args = CliArgs::parse_from(["wewa", "https://example.com", "-d", "1"]);
        assert_eq!(args.display, Some(1));
    }

    #[test]
    fn test_parse_scale_flag() {
        let args = CliArgs::parse_from(["wewa", "demo.shader", "--scale", "0.5"]);
        assert_eq!(args.scale, Some(0.5));
    }

    #[test]
    fn test_parse_time_scale_flag() {
        let args = CliArgs::parse_from(["wewa", "demo.shader", "--time-scale", "0.5"]);
        assert_eq!(args.time_scale, Some(0.5));
    }

    #[test]
    fn test_parse_random_flag() {
        let args = CliArgs::parse_from(["wewa", "-r"]);
        assert!(args.random);
        assert!(args.url_or_path.is_none());
        assert!(args.builtin.is_none());
        match args.mode() {
            CommandMode::BuiltIn { name, .. } => {
                let all = crate::builtin::list_builtins();
                assert!(all.contains(&name));
            }
            _ => panic!("Expected BuiltIn mode"),
        }
    }

    #[test]
    fn test_video_fit_defaults_and_dispatch() {
        let args = CliArgs::parse_from(["wewa", "movie.mp4"]);
        assert_eq!(args.video_fit, None);
        for (value, expected) in [("contain", VideoFit::Contain), ("cover", VideoFit::Cover)] {
            let args = CliArgs::parse_from(["wewa", "movie.mp4", "--video-fit", value]);
            match args.mode() {
                CommandMode::Start { video_fit, .. } => assert_eq!(video_fit, Some(expected)),
                _ => panic!("Expected Start mode"),
            }
        }
        for args in [
            vec!["wewa", "movie.mp4", "--video-fit", "stretch"],
            vec!["wewa", "--video-fit", "cover"],
            vec!["wewa", "-b", "clouds", "--video-fit", "cover"],
            vec!["wewa", "--stopall", "--video-fit", "cover"],
        ] {
            assert!(CliArgs::try_parse_from(args).is_err());
        }
    }

    #[test]
    fn test_parse_builtin_flag() {
        let args = CliArgs::parse_from(["wewa", "-b", "starnest"]);
        assert_eq!(args.builtin, Some("starnest".to_string()));
        assert!(args.url_or_path.is_none());
    }
}
