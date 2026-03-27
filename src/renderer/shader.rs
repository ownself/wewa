/// Result of wrapping a ShaderToy GLSL source in a complete fragment shader.
pub struct WrappedShader {
    /// Complete GLSL 450 source ready for compilation
    pub source: String,
    /// Number of lines added before the user's shader code (for error mapping)
    pub wrapper_line_offset: usize,
}

/// WGSL vertex shader for a fullscreen triangle (3 vertices, no vertex buffer).
///
/// Uses `vertex_index` to compute positions that cover the entire viewport.
/// Vertex 0: (-1, -1), Vertex 1: (3, -1), Vertex 2: (-1, 3)
/// The triangle is clipped to the viewport, producing a fullscreen quad effect.
pub const FULLSCREEN_TRIANGLE_WGSL: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(vertex_index & 1u) * 4 - 1);
    let y = f32(i32(vertex_index & 2u) * 2 - 1);
    out.position = vec4<f32>(x, y, 0.0, 1.0);
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
    var out: VertexOutput;
    let x = f32(i32(vertex_index & 1u) * 4 - 1);
    let y = f32(i32(vertex_index & 2u) * 2 - 1);
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.tex_coord = vec2<f32>((x + 1.0) / 2.0, (1.0 - y) / 2.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_source, s_source, in.tex_coord);
}
"#;

/// Preprocess ShaderToy GLSL source to fix patterns that naga cannot handle.
///
/// Current fixups:
/// - `const in` parameter qualifier → `in` (naga doesn't support dual qualifier)
fn preprocess_shadertoy_glsl(source: &str) -> String {
    // Replace `const in` with just `in` in function parameter declarations.
    // This is a common ShaderToy pattern that naga's GLSL parser rejects.
    source.replace("const in ", "in ")
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

    // The wrapper preamble before user code
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

"#;

    let wrapper_line_offset = preamble.lines().count();

    // The suffix after user code
    let suffix = r#"

layout(location = 0) out vec4 outColor;

void main() {
    vec2 fragCoord = vec2(gl_FragCoord.x, iResolution.y - gl_FragCoord.y);
    mainImage(outColor, fragCoord);
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
        assert!(wrapped.source.contains(user_code));
        assert!(wrapped.source.contains("mainImage(outColor, fragCoord)"));
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
        assert!(FULLSCREEN_TRIANGLE_WGSL.contains("vs_main"));
        assert!(FULLSCREEN_TRIANGLE_WGSL.contains("vertex_index"));
        assert!(FULLSCREEN_TRIANGLE_WGSL.contains("@builtin(position)"));
    }
}
