// Ultra Abstract, Blurry, Fast Color Shader
void mainImage( out vec4 fragColor, in vec2 fragCoord )
{
    // Normalize pixel coordinates to [0,1]
    vec2 uv = fragCoord.xy / iResolution.xy;

    // Animate over time (slow movement)
    float t = iTime * 0.25;

    // Generate smooth, animated color channels using sine waves
    // These create large, soft gradients that move over time

    // Red channel: horizontal gradient, animated
    float r = 0.5 + 0.5 * sin(uv.x * 2.0 + t);

    // Green channel: vertical gradient, animated with different speed/direction
    float g = 0.5 + 0.5 * sin(uv.y * 2.0 - t * 1.2);

    // Blue channel: diagonal gradient, animated
    float b = 0.5 + 0.5 * sin((uv.x + uv.y) * 1.5 + t * 0.7);

    // Extra blur: blend with a second set of gradients, offset and animated differently-
    float r2 = 0.5 + 0.5 * sin((uv.x + 0.1 * sin(t)) * 2.0 + t * 1.1);
    float g2 = 0.5 + 0.5 * sin((uv.y + 0.1 * cos(t)) * 2.0 - t * 1.3);
    float b2 = 0.5 + 0.5 * sin((uv.x - uv.y) * 1.2 + t * 0.9);

    // Mix the two color sets for extra smoothness and abstraction
    vec3 color = mix(vec3(r, g, b), vec3(r2, g2, b2), 0.5);

    // Output the final color (fully opaque)
    fragColor = vec4(color, 1.0);
}
