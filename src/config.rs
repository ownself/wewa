//! Configuration and instance tracking
//!
//! Manages application configuration and tracks running wallpaper instances.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Directory for storing instance tracking files
    pub instance_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            instance_dir: Self::default_instance_dir(),
        }
    }
}

impl Config {
    /// Get the default instance directory path
    pub fn default_instance_dir() -> PathBuf {
        std::env::temp_dir().join("wewa")
    }

    /// Ensure the instance directory exists
    pub fn ensure_instance_dir(&self) -> io::Result<()> {
        if !self.instance_dir.exists() {
            fs::create_dir_all(&self.instance_dir)?;
        }
        Ok(())
    }

    /// Get the path to an instance file for a specific display
    pub fn instance_file_path(&self, display_index: u32) -> PathBuf {
        self.instance_dir
            .join(format!("display_{}.json", display_index))
    }

    /// List all instance files in the instance directory
    pub fn list_instance_files(&self) -> io::Result<Vec<PathBuf>> {
        if !self.instance_dir.exists() {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();
        for entry in fs::read_dir(&self.instance_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with("display_")
                        && name.to_string_lossy().ends_with(".json")
                    {
                        files.push(path);
                    }
                }
            }
        }
        Ok(files)
    }
}

/// Represents a running wallpaper instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperInstance {
    /// 0-based monitor index
    pub display_index: u32,
    /// Operating system process ID
    pub pid: u32,
    /// URL or file:// path being displayed
    pub url: String,
    /// Local HTTP server port (if serving local files)
    pub server_port: Option<u16>,
    /// Instance start timestamp
    pub started_at: DateTime<Utc>,
}

impl WallpaperInstance {
    /// Create a new wallpaper instance record
    pub fn new(display_index: u32, url: String, server_port: Option<u16>) -> Self {
        Self {
            display_index,
            pid: std::process::id(),
            url,
            server_port,
            started_at: Utc::now(),
        }
    }

    /// Save the instance to a JSON file
    pub fn save(&self, config: &Config) -> io::Result<()> {
        config.ensure_instance_dir()?;
        let path = config.instance_file_path(self.display_index);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(path, json)
    }

    /// Load an instance from a JSON file
    #[allow(dead_code)]
    pub fn load(config: &Config, display_index: u32) -> io::Result<Self> {
        let path = config.instance_file_path(display_index);
        let json = fs::read_to_string(path)?;
        serde_json::from_str(&json).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Delete the instance file
    pub fn delete(config: &Config, display_index: u32) -> io::Result<()> {
        let path = config.instance_file_path(display_index);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Load all instances from the instance directory
    #[allow(dead_code)]
    pub fn load_all(config: &Config) -> io::Result<Vec<Self>> {
        let files = config.list_instance_files()?;
        let mut instances = Vec::new();

        for path in files {
            if let Ok(json) = fs::read_to_string(&path) {
                if let Ok(instance) = serde_json::from_str::<WallpaperInstance>(&json) {
                    instances.push(instance);
                }
            }
        }

        Ok(instances)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config
            .instance_dir
            .to_string_lossy()
            .contains("wewa"));
    }

    #[test]
    fn test_instance_file_path() {
        let config = Config::default();
        let path = config.instance_file_path(0);
        assert!(path.to_string_lossy().contains("display_0.json"));
    }

    #[test]
    fn test_wallpaper_instance_serialization() {
        let instance = WallpaperInstance::new(0, "https://example.com".to_string(), Some(8080));

        let json = serde_json::to_string(&instance).unwrap();
        let deserialized: WallpaperInstance = serde_json::from_str(&json).unwrap();

        assert_eq!(instance.display_index, deserialized.display_index);
        assert_eq!(instance.url, deserialized.url);
        assert_eq!(instance.server_port, deserialized.server_port);
    }
}
