//! Shader file support for ShaderToy-style fragment shaders.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const VERTEX_SHADER_SOURCE: &str = r#"#version 300 es
in vec2 a_position;

void main() {
    gl_Position = vec4(a_position, 0.0, 1.0);
}
"#;

pub struct ShaderBundle {
    pub root_dir: PathBuf,
    pub entry_file: String,
}

pub fn is_shader_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("shader"))
        .unwrap_or(false)
}

pub fn validate_scale(scale: f32) -> Result<f32, String> {
    if !scale.is_finite() {
        return Err("Scale must be a finite number".to_string());
    }

    if !(0.1..=2.0).contains(&scale) {
        return Err("Scale must be between 0.1 and 2.0".to_string());
    }

    Ok(scale)
}

pub fn validate_time_scale(time_scale: f32) -> Result<f32, String> {
    if !time_scale.is_finite() {
        return Err("Time scale must be a finite number".to_string());
    }

    if !(0.0..=100.0).contains(&time_scale) {
        return Err("Time scale must be between 0.0 and 100.0".to_string());
    }

    Ok(time_scale)
}

/// Describes a single iChannel input texture.
#[derive(Clone, Debug)]
pub struct ChannelInput {
    /// Index 0–3
    pub channel: usize,
    /// Filename inside the bundle directory
    pub filename: String,
    /// Whether this is a 3D volume texture (.bin)
    pub is_volume: bool,
}

pub fn create_shader_bundle(
    shader_path: &Path,
    scale: f32,
    time_scale: f32,
    channel_paths: &[Option<String>; 4],
) -> Result<ShaderBundle, String> {
    let shader_source = fs::read_to_string(shader_path).map_err(|e| {
        format!(
            "Failed to read shader file {}: {}",
            shader_path.display(),
            e
        )
    })?;

    if !shader_source.contains("mainImage") {
        return Err("Shader file must define a ShaderToy-style mainImage() function".to_string());
    }

    let bundle_dir = unique_shader_dir()?;
    fs::create_dir_all(&bundle_dir)
        .map_err(|e| format!("Failed to create temporary shader directory: {}", e))?;

    // Resolve and copy channel textures into the bundle directory
    let mut channels: Vec<ChannelInput> = Vec::new();
    for (i, opt) in channel_paths.iter().enumerate() {
        if let Some(ref path_str) = opt {
            let src = resolve_channel_path(shader_path, path_str)?;
            let is_volume = src
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("bin"))
                .unwrap_or(false);
            let ext = src
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png");
            let filename = format!("ichannel{}.{}", i, ext);
            let dest = bundle_dir.join(&filename);
            fs::copy(&src, &dest).map_err(|e| {
                format!(
                    "Failed to copy channel{} texture {} -> {}: {}",
                    i,
                    src.display(),
                    dest.display(),
                    e
                )
            })?;
            channels.push(ChannelInput {
                channel: i,
                filename,
                is_volume,
            });
        }
    }

    let html = build_shader_html(&shader_source, scale, time_scale, &channels)
        .map_err(|e| format!("Failed to build shader HTML: {}", e))?;

    let index_path = bundle_dir.join("index.html");
    fs::write(&index_path, html)
        .map_err(|e| format!("Failed to write shader HTML bundle: {}", e))?;

    Ok(ShaderBundle {
        root_dir: bundle_dir,
        entry_file: "index.html".to_string(),
    })
}

/// Generate JavaScript code to load and bind iChannel textures.
fn build_channel_load_js(channels: &[ChannelInput]) -> String {
    let mut js = String::new();
    for ch in channels {
        if ch.is_volume {
            // 3D volume texture: fetch .bin, parse header, upload as texImage3D
            js.push_str(&format!(
                r#"
      (function() {{
        const ch = {ch};
        fetch("{filename}")
          .then(r => r.arrayBuffer())
          .then(data => {{
            const view = new DataView(data);
            const xres = view.getUint32(4, true);
            const yres = view.getUint32(8, true);
            const zres = view.getUint32(12, true);
            const numCh = view.getUint8(16);
            const buffer = new Uint8Array(data, 20);
            const tex = gl.createTexture();
            gl.activeTexture(gl.TEXTURE0 + ch);
            gl.bindTexture(gl.TEXTURE_3D, tex);
            const fmt = numCh === 1 ? gl.RED : numCh === 2 ? gl.RG : numCh === 3 ? gl.RGB : gl.RGBA;
            const ifmt = numCh === 1 ? gl.R8 : numCh === 2 ? gl.RG8 : numCh === 3 ? gl.RGB8 : gl.RGBA8;
            gl.texImage3D(gl.TEXTURE_3D, 0, ifmt, xres, yres, zres, 0, fmt, gl.UNSIGNED_BYTE, buffer);
            gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
            gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
            gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_WRAP_S, gl.REPEAT);
            gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_WRAP_T, gl.REPEAT);
            gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_WRAP_R, gl.REPEAT);
            channelTextures[ch] = {{ texture: tex, target: gl.TEXTURE_3D }};
            channelResolutions[ch*3] = xres;
            channelResolutions[ch*3+1] = yres;
            channelResolutions[ch*3+2] = zres;
          }})
          .catch(e => console.error("Failed to load volume iChannel" + ch, e));
      }})();
"#,
                ch = ch.channel,
                filename = ch.filename,
            ));
        } else {
            // 2D texture: load as image, upload as texImage2D with mipmaps
            js.push_str(&format!(
                r#"
      (function() {{
        const ch = {ch};
        const img = new Image();
        img.onload = function() {{
          const tex = gl.createTexture();
          gl.activeTexture(gl.TEXTURE0 + ch);
          gl.bindTexture(gl.TEXTURE_2D, tex);
          gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true);
          gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);
          gl.generateMipmap(gl.TEXTURE_2D);
          gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
          gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
          gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT);
          gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.REPEAT);
          channelTextures[ch] = {{ texture: tex, target: gl.TEXTURE_2D }};
          channelResolutions[ch*3] = img.width;
          channelResolutions[ch*3+1] = img.height;
          channelResolutions[ch*3+2] = 1.0;
        }};
        img.src = "{filename}";
      }})();
"#,
                ch = ch.channel,
                filename = ch.filename,
            ));
        }
    }
    js
}

/// Resolve a channel path: if absolute use as-is, otherwise resolve relative
/// to the shader file's parent directory.
fn resolve_channel_path(shader_path: &Path, channel_path: &str) -> Result<PathBuf, String> {
    let p = Path::new(channel_path);
    let resolved = if p.is_absolute() {
        p.to_path_buf()
    } else {
        shader_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(p)
    };
    if !resolved.exists() {
        return Err(format!(
            "Channel texture file not found: {}",
            resolved.display()
        ));
    }
    Ok(resolved)
}

pub fn cleanup_shader_bundle(bundle: &ShaderBundle) {
    let _ = fs::remove_dir_all(&bundle.root_dir);
}

fn unique_shader_dir() -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("System clock error: {}", e))?
        .as_millis();

    Ok(std::env::temp_dir().join(format!(
        "webwallpaper_shader_{}_{}",
        std::process::id(),
        timestamp
    )))
}

fn build_shader_html(
    shader_source: &str,
    scale: f32,
    time_scale: f32,
    channels: &[ChannelInput],
) -> Result<String, serde_json::Error> {
    let shader_json = serde_json::to_string(shader_source)?;
    let vertex_shader_json = serde_json::to_string(VERTEX_SHADER_SOURCE)?;

    // Build channel config JSON for JS (provided channels with known types)
    let mut channel_config_entries = Vec::new();
    for ch in channels {
        channel_config_entries.push(format!(
            "{{ch:{},type:\"{}\"}}",
            ch.channel,
            if ch.is_volume { "3d" } else { "2d" }
        ));
    }
    let channel_config_js = format!("[{}]", channel_config_entries.join(","));

    // Build JS code to load textures and bind them
    let channel_load_js = build_channel_load_js(channels);

    Ok(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
  <title>WebWallpaper Shader</title>
  <style>
    :root {{
      color-scheme: dark;
      --bg: #000;
      --fg: #f4f4f4;
      --muted: #9ca3af;
      --panel: rgba(0, 0, 0, 0.8);
    }}

    html, body {{
      margin: 0;
      width: 100%;
      height: 100%;
      overflow: hidden;
      background: radial-gradient(circle at top, #10131a 0%, var(--bg) 55%);
      font-family: "Segoe UI", "PingFang SC", sans-serif;
    }}

    body {{
      display: grid;
      place-items: stretch;
    }}

    canvas {{
      width: 100vw;
      height: 100vh;
      display: block;
      background: #000;
    }}

    #error {{
      position: fixed;
      inset: auto 16px 16px 16px;
      padding: 12px 14px;
      border: 1px solid rgba(255, 255, 255, 0.12);
      border-radius: 10px;
      background: var(--panel);
      color: var(--fg);
      white-space: pre-wrap;
      font-size: 13px;
      line-height: 1.5;
      display: none;
      max-height: 45vh;
      overflow: auto;
    }}

    #hint {{
      position: fixed;
      top: 16px;
      left: 16px;
      padding: 8px 10px;
      border-radius: 999px;
      background: rgba(0, 0, 0, 0.45);
      color: var(--muted);
      font-size: 12px;
      letter-spacing: 0.04em;
      backdrop-filter: blur(12px);
    }}
  </style>
</head>
<body>
  <canvas id="shader"></canvas>
  <div id="hint">Shader scale {scale:.2} | time scale {time_scale:.2}</div>
  <pre id="error"></pre>
  <script>
    (() => {{
      const renderScale = {scale};
      const timeScale = {time_scale};
      const userShaderSource = {shader_json};
      const vertexShaderSource = {vertex_shader_json};
      const providedChannels = {channel_config_js};
      const canvas = document.getElementById("shader");
      const errorEl = document.getElementById("error");
      const gl = canvas.getContext("webgl2", {{
        alpha: false,
        antialias: false,
        depth: false,
        stencil: false,
        preserveDrawingBuffer: false,
        powerPreference: "high-performance"
      }});

      if (!gl) {{
        showError("WebGL 2 is not available in this WebView.");
        return;
      }}

      // Build fragment shader source with given channel type config
      function buildFragSource(unusedAs3D) {{
        let decls = "";
        for (let i = 0; i < 4; i++) {{
          const prov = providedChannels.find(c => c.ch === i);
          if (prov) {{
            decls += prov.type === "3d"
              ? "uniform highp sampler3D iChannel" + i + ";\n"
              : "uniform sampler2D iChannel" + i + ";\n";
          }} else {{
            decls += unusedAs3D.has(i)
              ? "uniform highp sampler3D iChannel" + i + ";\n"
              : "uniform sampler2D iChannel" + i + ";\n";
          }}
        }}
        decls += "uniform vec3 iChannelResolution[4];\n";
        return `#version 300 es
precision highp float;
precision highp sampler3D;
#define HW_PERFORMANCE 1
uniform vec3 iResolution;
uniform float iTime;
uniform float iTimeDelta;
uniform float iFrameRate;
uniform int iFrame;
uniform vec4 iMouse;
uniform vec4 iDate;
${{decls}}
${{userShaderSource}}

out vec4 fragColor;
void main() {{
  vec4 color = vec4(0.0);
  mainImage(color, gl_FragCoord.xy);
  fragColor = color;
}}`;
      }}

      // Find unused channel indices
      const unusedIndices = [];
      for (let i = 0; i < 4; i++) {{
        if (!providedChannels.find(c => c.ch === i)) unusedIndices.push(i);
      }}

      // Try compiling: first all unused as sampler2D, then try sampler3D combos
      let program = null;
      let lastError = "";
      const totalCombos = 1 << unusedIndices.length;
      for (let mask = 0; mask < totalCombos; mask++) {{
        const as3D = new Set();
        for (let j = 0; j < unusedIndices.length; j++) {{
          if (mask & (1 << j)) as3D.add(unusedIndices[j]);
        }}
        const src = buildFragSource(as3D);
        // All attempts are silent; we show error only if all fail
        program = createProgram(gl, vertexShaderSource, src, true);
        if (program) break;
        if (mask === 0) lastError = gl._lastError || "";
      }}
      if (!program) {{
        showError(lastError || "Shader compilation failed");
        return;
      }}

      const positionLocation = gl.getAttribLocation(program, "a_position");
      const resolutionLocation = gl.getUniformLocation(program, "iResolution");
      const timeLocation = gl.getUniformLocation(program, "iTime");
      const timeDeltaLocation = gl.getUniformLocation(program, "iTimeDelta");
      const frameRateLocation = gl.getUniformLocation(program, "iFrameRate");
      const frameLocation = gl.getUniformLocation(program, "iFrame");
      const mouseLocation = gl.getUniformLocation(program, "iMouse");
      const dateLocation = gl.getUniformLocation(program, "iDate");
      const channelResLocation = gl.getUniformLocation(program, "iChannelResolution");

      // iChannel texture slots
      const channelLocations = [
        gl.getUniformLocation(program, "iChannel0"),
        gl.getUniformLocation(program, "iChannel1"),
        gl.getUniformLocation(program, "iChannel2"),
        gl.getUniformLocation(program, "iChannel3"),
      ];
      const channelTextures = [null, null, null, null];
      const channelResolutions = new Float32Array(12); // 4 x vec3

      // Load channel textures
      {channel_load_js}

      const positionBuffer = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
      gl.bufferData(
        gl.ARRAY_BUFFER,
        new Float32Array([
          -1.0, -1.0,
           1.0, -1.0,
          -1.0,  1.0,
          -1.0,  1.0,
           1.0, -1.0,
           1.0,  1.0
        ]),
        gl.STATIC_DRAW
      );

      let startTime = performance.now();
      let lastFrameTime = startTime;
      let frame = 0;
      let mouse = [0, 0, 0, 0];

      canvas.addEventListener("mousemove", event => {{
        const rect = canvas.getBoundingClientRect();
        const x = (event.clientX - rect.left) * (canvas.width / rect.width);
        const y = canvas.height - (event.clientY - rect.top) * (canvas.height / rect.height);
        mouse[0] = x;
        mouse[1] = y;
      }});

      canvas.addEventListener("mousedown", () => {{
        mouse[2] = mouse[0];
        mouse[3] = mouse[1];
      }});

      canvas.addEventListener("mouseup", () => {{
        mouse[2] = 0;
        mouse[3] = 0;
      }});

      function resize() {{
        const dpr = window.devicePixelRatio || 1;
        const width = Math.max(1, Math.floor(window.innerWidth * dpr * renderScale));
        const height = Math.max(1, Math.floor(window.innerHeight * dpr * renderScale));

        if (canvas.width !== width || canvas.height !== height) {{
          canvas.width = width;
          canvas.height = height;
        }}

        gl.viewport(0, 0, canvas.width, canvas.height);
      }}

      window.addEventListener("resize", resize);
      resize();

      gl.useProgram(program);
      gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
      gl.enableVertexAttribArray(positionLocation);
      gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 0, 0);

      function render(now) {{
        resize();

        const elapsed = ((now - startTime) / 1000) * timeScale;
        const delta = Math.max(0, (now - lastFrameTime) / 1000) * timeScale;
        const fps = delta > 0 ? 1 / delta : 0;
        const date = new Date();
        const seconds = date.getHours() * 3600 + date.getMinutes() * 60 + date.getSeconds() + date.getMilliseconds() / 1000;

        gl.uniform3f(resolutionLocation, canvas.width, canvas.height, 1.0);
        gl.uniform1f(timeLocation, elapsed);
        gl.uniform1f(timeDeltaLocation, delta);
        gl.uniform1f(frameRateLocation, fps);
        gl.uniform1i(frameLocation, frame);
        gl.uniform4f(mouseLocation, mouse[0], mouse[1], mouse[2], mouse[3]);
        gl.uniform4f(dateLocation, date.getFullYear(), date.getMonth() + 1, date.getDate(), seconds);

        // Bind iChannel textures
        for (let i = 0; i < 4; i++) {{
          if (channelTextures[i]) {{
            gl.activeTexture(gl.TEXTURE0 + i);
            gl.bindTexture(channelTextures[i].target, channelTextures[i].texture);
            if (channelLocations[i]) gl.uniform1i(channelLocations[i], i);
          }}
        }}
        if (channelResLocation) gl.uniform3fv(channelResLocation, channelResolutions);

        gl.drawArrays(gl.TRIANGLES, 0, 6);

        frame += 1;
        lastFrameTime = now;
        requestAnimationFrame(render);
      }}

      requestAnimationFrame(render);

      function createProgram(gl, vertexSource, fragmentSource, silent) {{
        const vertexShader = compileShader(gl, gl.VERTEX_SHADER, vertexSource, silent);
        const fragmentShader = compileShader(gl, gl.FRAGMENT_SHADER, fragmentSource, silent);
        if (!vertexShader || !fragmentShader) {{
          return null;
        }}

        const program = gl.createProgram();
        gl.attachShader(program, vertexShader);
        gl.attachShader(program, fragmentShader);
        gl.linkProgram(program);

        if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {{
          if (!silent) showError("Shader program link error:\n" + gl.getProgramInfoLog(program));
          gl.deleteProgram(program);
          return null;
        }}

        return program;
      }}

      function compileShader(gl, type, source, silent) {{
        const shader = gl.createShader(type);
        gl.shaderSource(shader, source);
        gl.compileShader(shader);

        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {{
          const err = (type === gl.VERTEX_SHADER ? "Vertex" : "Fragment") + " shader compile error:\n" + gl.getShaderInfoLog(shader);
          gl._lastError = err;
          if (!silent) showError(err);
          gl.deleteShader(shader);
          return null;
        }}

        return shader;
      }}

      function showError(message) {{
        errorEl.textContent = message;
        errorEl.style.display = "block";
      }}
    }})();
  </script>
</body>
</html>
"#,
        channel_config_js = channel_config_js,
        channel_load_js = channel_load_js,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_shader_file() {
        assert!(is_shader_file(Path::new("demo.shader")));
        assert!(is_shader_file(Path::new("DEMO.SHADER")));
        assert!(!is_shader_file(Path::new("index.html")));
    }

    #[test]
    fn test_validate_scale() {
        assert_eq!(validate_scale(1.0).unwrap(), 1.0);
        assert!(validate_scale(0.05).is_err());
        assert!(validate_scale(2.5).is_err());
        assert!(validate_scale(f32::NAN).is_err());
    }

    #[test]
    fn test_validate_time_scale() {
        assert_eq!(validate_time_scale(1.0).unwrap(), 1.0);
        assert_eq!(validate_time_scale(0.0).unwrap(), 0.0);
        assert!(validate_time_scale(-0.1).is_err());
        assert!(validate_time_scale(101.0).is_err());
        assert!(validate_time_scale(f32::NAN).is_err());
    }

    #[test]
    fn test_shader_html_contains_runtime() {
        let html = build_shader_html(
            "void mainImage(out vec4 c, in vec2 f) { c = vec4(1.0); }",
            0.5,
            1.25,
            &[],
        )
        .unwrap();
        assert!(html.contains("Shader scale 0.50"));
        assert!(html.contains("time scale 1.25"));
        assert!(html.contains("mainImage"));
        assert!(html.contains("renderScale = 0.5"));
        assert!(html.contains("timeScale = 1.25"));
    }

    #[test]
    fn test_create_shader_bundle() {
        let temp_path = std::env::temp_dir().join(format!(
            "webwallpaper_test_shader_{}_{}.shader",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        fs::write(
            &temp_path,
            "void mainImage(out vec4 fragColor, in vec2 fragCoord) { fragColor = vec4(1.0); }",
        )
        .unwrap();

        let no_channels = [None, None, None, None];
        let bundle = create_shader_bundle(&temp_path, 1.0, 1.0, &no_channels).unwrap();
        assert!(bundle.root_dir.join(&bundle.entry_file).exists());

        cleanup_shader_bundle(&bundle);
        let _ = fs::remove_file(temp_path);
    }
}
