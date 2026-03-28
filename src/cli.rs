//! CLI argument parsing using clap derive
//!
//! Defines the command-line interface for webwallpaper.

use clap::Parser;

/// Display web content as desktop wallpaper
#[derive(Parser, Debug)]
#[command(name = "webwallpaper")]
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
    #[arg(long, conflicts_with_all = ["url_or_path", "stop"])]
    pub stopall: bool,

    /// HTTP server port for serving local files (default: 8080)
    #[arg(short = 'p', long, default_value = "8080")]
    pub port: u16,

    /// Shader render scale for .shader inputs (default: 1.0)
    #[arg(short = 's', long, default_value = "1.0")]
    pub scale: f32,

    /// Shader time scale for .shader inputs (default: 1.0)
    #[arg(long, visible_alias = "ts", default_value = "1.0")]
    pub time_scale: f32,

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
        } else if let Some(ref url_or_path) = self.url_or_path {
            CommandMode::Start {
                url_or_path: url_or_path.clone(),
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
        let args = CliArgs::parse_from(["webwallpaper", "https://example.com"]);
        assert_eq!(args.url_or_path, Some("https://example.com".to_string()));
        assert!(!args.stopall);
        assert!(args.stop.is_none());
    }

    #[test]
    fn test_parse_stop() {
        let args = CliArgs::parse_from(["webwallpaper", "--stop", "0"]);
        assert_eq!(args.stop, Some(0));
        assert!(args.url_or_path.is_none());
    }

    #[test]
    fn test_parse_stopall() {
        let args = CliArgs::parse_from(["webwallpaper", "--stopall"]);
        assert!(args.stopall);
    }

    #[test]
    fn test_parse_display_flag() {
        let args = CliArgs::parse_from(["webwallpaper", "https://example.com", "-d", "1"]);
        assert_eq!(args.display, Some(1));
    }

    #[test]
    fn test_parse_scale_flag() {
        let args = CliArgs::parse_from(["webwallpaper", "demo.shader", "--scale", "0.5"]);
        assert_eq!(args.scale, 0.5);
    }

    #[test]
    fn test_parse_time_scale_flag() {
        let args = CliArgs::parse_from(["webwallpaper", "demo.shader", "--time-scale", "0.5"]);
        assert_eq!(args.time_scale, 0.5);
    }
}
