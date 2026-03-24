float speed = .2;

vec2 rotateUV(vec2 uv, float rotation, vec2 mid)
{
    return vec2(
            cos(rotation) * (uv.x - mid.x) + sin(rotation) * (uv.y - mid.y) + mid.x,
            cos(rotation) * (uv.y - mid.y) - sin(rotation) * (uv.x - mid.x) + mid.y
            );
}


void mainImage( out vec4 fragColor, in vec2 fragCoord )
{
    // Normalized pixel coordinates (from 0 to 1)
    vec2 uv = rotateUV(fragCoord, 0.5, iResolution.xy/2.0)/iResolution.xy;

    // Output to screen
    if (uv.y + sin(iTime) * 0.05 > .85 + 0.1 * (cos((uv.x + iTime * speed) * 3.15 * 5.0))) {
        vec3 col = 0.5 + 0.5*cos(1.01 * iTime+uv.xyx+vec3(0,2,4));
        fragColor = vec4(col,1.0);
    } else if (uv.y + cos(iTime) * 0.05 > .6 + 0.1 * (cos((uv.x + iTime * speed * 0.998) * 3.15 * 5.0))) {
        vec3 col = 0.5 + 0.5*cos(1.02 * iTime+uv.xyx+vec3(0,2,4));
        fragColor = vec4(col * .8,1.0);
    } else if (uv.y + sin(iTime) * 0.05 > .40 + 0.1 * (cos((uv.x + iTime * speed * 0.996) * 3.15 * 5.0))) {
        vec3 col = 0.5 + 0.5*cos(1.03 * iTime+uv.xyx+vec3(0,2,4));
        fragColor = vec4(col * .4,1.0);
    } else if (uv.y + sin(iTime) * 0.05 > .15 + 0.1 * (cos((uv.x + iTime * speed * 0.994) * 3.15 * 5.0))) {
        vec3 col = 0.5 + 0.5*cos(1.03 * iTime+uv.xyx+vec3(0,2,4));
        fragColor = vec4(col * .2,1.0);
    }
    else {
        vec3 col = 0.5 + 0.5*cos(1.04 * iTime+uv.xyx+vec3(0,2,4));
        fragColor = vec4(col * .0,1.0);
    }
}
