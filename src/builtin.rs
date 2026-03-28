//! Built-in shader and texture resources embedded at compile time.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// ── Embedded shader sources ────────────────────────────────────────────────

fn get_shader_source(name: &str) -> Option<&'static str> {
    match name {
        "2dclouds" => Some(include_str!("../shaders/2dclouds.shader")),
        "accretion" => Some(include_str!("../shaders/accretion.shader")),
        "ascend" => Some(include_str!("../shaders/ascend.shader")),
        "auroras" => Some(include_str!("../shaders/auroras.shader")),
        "chillywave" => Some(include_str!("../shaders/chillywave.shader")),
        "chillywave2" => Some(include_str!("../shaders/chillywave2.shader")),
        "classic4colors" => Some(include_str!("../shaders/classic4colors.shader")),
        "clouds" => Some(include_str!("../shaders/clouds.shader")),
        "crumpledwave" => Some(include_str!("../shaders/crumpledwave.shader")),
        "darktransit" => Some(include_str!("../shaders/darktransit.shader")),
        "forknixietubeclock" => Some(include_str!("../shaders/forknixietubeclock.shader")),
        "hexagonalgrid" => Some(include_str!("../shaders/hexagonalgrid.shader")),
        "hexneonlove" => Some(include_str!("../shaders/hexneonlove.shader")),
        "hieroglyphs" => Some(include_str!("../shaders/hieroglyphs.shader")),
        "iceandfire" => Some(include_str!("../shaders/iceandfire.shader")),
        "linuxwallpaper" => Some(include_str!("../shaders/linuxwallpaper.shader")),
        "mandelbrot" => Some(include_str!("../shaders/mandelbrot.shader")),
        "montereywannabe" => Some(include_str!("../shaders/montereywannabe.shader")),
        "patternminimax" => Some(include_str!("../shaders/patternminimax.shader")),
        "pinkvoid" => Some(include_str!("../shaders/pinkvoid.shader")),
        "plasma" => Some(include_str!("../shaders/plasma.shader")),
        "plasmawaves" => Some(include_str!("../shaders/plasmawaves.shader")),
        "polyhedrons" => Some(include_str!("../shaders/polyhedrons.shader")),
        "proteanclouds" => Some(include_str!("../shaders/proteanclouds.shader")),
        "ps3xmb" => Some(include_str!("../shaders/ps3xmb.shader")),
        "seventymelt" => Some(include_str!("../shaders/seventymelt.shader")),
        "singularity" => Some(include_str!("../shaders/singularity.shader")),
        "spiralgalaxy" => Some(include_str!("../shaders/spiralgalaxy.shader")),
        "starnest" => Some(include_str!("../shaders/starnest.shader")),
        "starshipreentry" => Some(include_str!("../shaders/starshipreentry.shader")),
        "synthwavecanyon" => Some(include_str!("../shaders/synthwavecanyon.shader")),
        "tilewarppt3" => Some(include_str!("../shaders/tilewarppt3.shader")),
        "tumblerock" => Some(include_str!("../shaders/tumblerock.shader")),
        "undulatingurchin" => Some(include_str!("../shaders/undulatingurchin.shader")),
        "voronoigradient" => Some(include_str!("../shaders/voronoigradient.shader")),
        "wadongmo759" => Some(include_str!("../shaders/wadongmo759.shader")),
        "waveymc" => Some(include_str!("../shaders/waveymc.shader")),
        _ => None,
    }
}

// ── Embedded texture data ──────────────────────────────────────────────────

fn get_texture_data(name: &str) -> Option<&'static [u8]> {
    match name {
        "noise_rgba.png" => Some(include_bytes!("../textures/noise_rgba.png")),
        "noise_grey.png" => Some(include_bytes!("../textures/noise_grey.png")),
        "noise_volume.bin" => Some(include_bytes!("../textures/noise_volume.bin")),
        "urchin.bin" => Some(include_bytes!("../textures/urchin.bin")),
        "wood.jpg" => Some(include_bytes!("../textures/wood.jpg")),
        _ => None,
    }
}

// ── Embedded config ────────────────────────────────────────────────────────

const BUILTIN_CONFIG: &str = include_str!("../builtins.json");

/// Default parameters for a built-in shader.
pub struct BuiltinConfig {
    pub scale: f32,
    pub time_scale: f32,
    pub channels: [Option<String>; 4],
}

/// Result of extracting a built-in shader to a temporary directory.
pub struct BuiltinResult {
    pub temp_dir: PathBuf,
    pub shader_path: PathBuf,
    pub config: BuiltinConfig,
}

/// Return sorted list of available built-in shader names.
pub fn list_builtins() -> Vec<String> {
    let config: serde_json::Value = serde_json::from_str(BUILTIN_CONFIG).unwrap_or_default();
    let mut names: Vec<String> = config
        .as_object()
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();
    names.sort();
    names
}

/// Extract a built-in shader (and its textures) to a temporary directory.
pub fn prepare_builtin(name: &str) -> Result<BuiltinResult, String> {
    let source = get_shader_source(name).ok_or_else(|| {
        let available = list_builtins().join(", ");
        format!(
            "Unknown built-in shader '{}'. Available: {}",
            name, available
        )
    })?;

    let config = parse_config(name)?;

    let dir = builtin_temp_dir()?;
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    // Write shader source
    let shader_path = dir.join(format!("{}.shader", name));
    fs::write(&shader_path, source)
        .map_err(|e| format!("Failed to write shader: {}", e))?;

    // Write referenced textures
    for ch in &config.channels {
        if let Some(tex_name) = ch {
            let data = get_texture_data(tex_name)
                .ok_or_else(|| format!("Built-in texture not found: {}", tex_name))?;
            fs::write(dir.join(tex_name), data)
                .map_err(|e| format!("Failed to write texture {}: {}", tex_name, e))?;
        }
    }

    Ok(BuiltinResult {
        temp_dir: dir,
        shader_path,
        config,
    })
}

/// Clean up the temporary extraction directory.
pub fn cleanup_builtin(result: &BuiltinResult) {
    let _ = fs::remove_dir_all(&result.temp_dir);
}

fn builtin_temp_dir() -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("System clock error: {}", e))?
        .as_millis();
    Ok(std::env::temp_dir().join(format!(
        "webwallpaper_builtin_{}_{}",
        std::process::id(),
        timestamp
    )))
}

fn parse_config(name: &str) -> Result<BuiltinConfig, String> {
    let all: serde_json::Value = serde_json::from_str(BUILTIN_CONFIG)
        .map_err(|e| format!("Invalid builtins config: {}", e))?;

    let entry = all
        .get(name)
        .ok_or_else(|| format!("No config entry for shader: {}", name))?;

    Ok(BuiltinConfig {
        scale: entry
            .get("scale")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32,
        time_scale: entry
            .get("time_scale")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32,
        channels: [
            entry
                .get("channel0")
                .and_then(|v| v.as_str())
                .map(String::from),
            entry
                .get("channel1")
                .and_then(|v| v.as_str())
                .map(String::from),
            entry
                .get("channel2")
                .and_then(|v| v.as_str())
                .map(String::from),
            entry
                .get("channel3")
                .and_then(|v| v.as_str())
                .map(String::from),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_builtins() {
        let list = list_builtins();
        assert!(list.contains(&"clouds".to_string()));
        assert!(list.contains(&"starnest".to_string()));
        assert!(!list.is_empty());
    }

    #[test]
    fn test_parse_config_with_channels() {
        let config = parse_config("clouds").unwrap();
        assert_eq!(config.channels[0].as_deref(), Some("noise_rgba.png"));
        // channel3 not set
        assert!(config.channels[3].is_none());
    }

    #[test]
    fn test_parse_config_no_channels() {
        let config = parse_config("starnest").unwrap();
        assert!(config.channels[0].is_none());
    }

    #[test]
    fn test_unknown_shader() {
        assert!(get_shader_source("nonexistent").is_none());
    }

    #[test]
    fn test_prepare_and_cleanup() {
        let result = prepare_builtin("starnest").unwrap();
        assert!(result.shader_path.exists());
        cleanup_builtin(&result);
        assert!(!result.temp_dir.exists());
    }
}
