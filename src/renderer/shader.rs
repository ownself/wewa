/// Result of wrapping a ShaderToy GLSL source in a complete fragment shader.
pub struct WrappedShader {
    /// Complete GLSL 450 source ready for compilation
    pub source: String,
    /// Number of lines added before the user's shader code (for error mapping)
    pub wrapper_line_offset: usize,
}

/// WGSL vertex shader for a fullscreen quad (6 vertices, two triangles, no vertex buffer).
///
/// Uses `vertex_index` to compute positions for two triangles covering the viewport.
/// Triangle 1: (-1,-1), (1,-1), (1,1)
/// Triangle 2: (-1,-1), (1,1), (-1,1)
pub const FULLSCREEN_QUAD_WGSL: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // 6 vertices forming 2 triangles:
    //   0: (-1,-1)  1: (1,-1)  2: (1,1)
    //   3: (-1,-1)  4: (1, 1)  5: (-1,1)
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );
    var out: VertexOutput;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return out;
}
"#;

/// WGSL blit shader that samples an offscreen texture and draws it fullscreen.
///
/// Used when `--scale` < 1.0 to upscale the lower-resolution render target
/// to the full window size with linear filtering.
pub const BLIT_SHADER_WGSL: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

@group(0) @binding(0) var t_source: texture_2d<f32>;
@group(0) @binding(1) var s_source: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );
    var out: VertexOutput;
    let pos = positions[vertex_index];
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.tex_coord = vec2<f32>((pos.x + 1.0) / 2.0, (1.0 - pos.y) / 2.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_source, s_source, in.tex_coord);
}
"#;

/// Preprocess ShaderToy GLSL source to fix patterns that naga cannot handle
/// and to bridge Vulkan/OpenGL coordinate differences.
///
/// Current fixups:
/// - `const in` parameter qualifier → `in` (naga doesn't support dual qualifier)
/// - `gl_FragCoord` → `_ww_FragCoord` (global variable with y-flipped coordinates,
///    set in main() before user code runs — fixes shaders that access gl_FragCoord
///    directly instead of through the mainImage fragCoord parameter)
fn preprocess_shadertoy_glsl(source: &str) -> String {
    source
        .replace("const in ", "in ")
        .replace("gl_FragCoord", "_ww_FragCoord")
}

/// Wrap a ShaderToy GLSL fragment shader in a complete `#version 450` program.
///
/// The wrapper adds:
/// - `#version 450` header
/// - Uniform block declaration matching `ShaderToyUniforms` layout (std140)
/// - Dummy `iChannel0..3` sampler declarations (sample black, no texture inputs)
/// - The user's shader source (which must define `mainImage`)
/// - A `void main()` that calls `mainImage(outColor, gl_FragCoord.xy)`
pub fn wrap_shadertoy_glsl(user_source: &str) -> WrappedShader {
    let preprocessed = preprocess_shadertoy_glsl(user_source);

    // The wrapper preamble before user code.
    // _ww_FragCoord is a global that replaces gl_FragCoord in user code
    // (done by preprocessing). It is set in main() with y-flipped coordinates
    // to bridge Vulkan (y=0 top) → OpenGL/ShaderToy (y=0 bottom).
    let preamble = r#"#version 450
precision highp float;

layout(set = 0, binding = 0) uniform Uniforms {
    vec3 iResolution;
    float _pad0;
    float iTime;
    float iTimeDelta;
    int iFrame;
    float iFrameRate;
    vec4 iMouse;
    vec4 iDate;
};

vec4 _ww_FragCoord;

"#;

    let wrapper_line_offset = preamble.lines().count();

    // The suffix after user code
    let suffix = r#"

layout(location = 0) out vec4 outColor;

void main() {
    _ww_FragCoord = vec4(gl_FragCoord.x, iResolution.y - gl_FragCoord.y, gl_FragCoord.z, gl_FragCoord.w);
    mainImage(outColor, _ww_FragCoord.xy);
}
"#;

    let source = format!("{}{}{}", preamble, &preprocessed, suffix);

    WrappedShader {
        source,
        wrapper_line_offset,
    }
}

/// Map a naga/wgpu error line number back to the user's original shader source.
pub fn map_error_line(error_line: usize, wrapper_line_offset: usize) -> usize {
    if error_line > wrapper_line_offset {
        error_line - wrapper_line_offset
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_shadertoy_glsl() {
        let user_code = "void mainImage(out vec4 fragColor, in vec2 fragCoord) {\n    fragColor = vec4(1.0);\n}";
        let wrapped = wrap_shadertoy_glsl(user_code);

        assert!(wrapped.source.contains("#version 450"));
        assert!(wrapped.source.contains("layout(set = 0, binding = 0) uniform Uniforms"));
        assert!(wrapped.source.contains("vec3 iResolution"));
        assert!(wrapped.source.contains("float iTime"));
        assert!(wrapped.source.contains("vec4 iMouse"));
        assert!(wrapped.source.contains("vec4 iDate"));
        assert!(wrapped.source.contains("_ww_FragCoord"));
        assert!(wrapped.source.contains("mainImage(outColor, _ww_FragCoord.xy)"));
        assert!(wrapped.wrapper_line_offset > 0);
    }

    #[test]
    fn test_map_error_line() {
        let wrapped = wrap_shadertoy_glsl("// line 1\n// line 2\n");
        let offset = wrapped.wrapper_line_offset;
        // Error on wrapper line offset + 2 should map to user line 2
        assert_eq!(map_error_line(offset + 2, offset), 2);
        // Error in preamble maps to 0
        assert_eq!(map_error_line(3, offset), 0);
    }

    #[test]
    fn test_fullscreen_triangle_wgsl() {
        assert!(FULLSCREEN_QUAD_WGSL.contains("vs_main"));
        assert!(FULLSCREEN_QUAD_WGSL.contains("vertex_index"));
        assert!(FULLSCREEN_QUAD_WGSL.contains("@builtin(position)"));
    }
}
