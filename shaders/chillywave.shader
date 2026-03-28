const vec3 BLACK = vec3(0.);
const vec3 PINK = vec3(233, 71, 245) / 255.;
const vec3 BLUE = vec3(47, 75, 162) / 255.;

mat2x2 rotate(float r) {
    return mat2x2(cos(r), sin(r), -sin(r), cos(r));
}

vec2 hash22(vec2 p)
{
    vec3 p3 = fract(vec3(p.xyx) * vec3(.1031, .1030, .0973));
    p3 += dot(p3, p3.yzx+33.33);
    return fract((p3.xx+p3.yz)*p3.zy);

}

float sdfCircle(vec2 p, float r) {
    return length(p) - r;
}

float circleGrid(vec2 uv) {
    float time = iTime;
    uv.y += time*.5;
    vec2 id = floor(uv);
    vec2 h = hash22(id);
    vec2 gv = fract(uv) - .5;

    if (h.x > .8) {
        float size = h.y * .1;

        h *= 10.;
        gv += vec2(cos(h.x+time), sin(h.y+time)) * .2;
        return smoothstep(.1, .05, sdfCircle(gv, size));
    }

    return 0.;
}

vec3 background_color(vec2 uv) {
    vec3 col = vec3(0.);

    float y = sin(uv.x - .2) * .3 - .1;
    float m = uv.y - y;

    col += mix(BLUE, BLACK, smoothstep(0., 1., abs(m)));
    col += mix(PINK, BLACK, smoothstep(0., 1., abs(m - .8)));
    return col * .5;
}

float wave(vec2 uv, float offset) {
    float time = iTime;

    float x_offset = offset;
    float x_movement = time*.1;
    float amp = sin(offset+time*.2)*.3;
    float y = sin(uv.x + x_offset + x_movement)*amp;

    float m = uv.y - y;
    return .0175 / max(abs(m) + .01, 1e-3) + .01;
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    float time = iTime;
    vec2 uv = (2. * gl_FragCoord.xy - iResolution.xy) / iResolution.y;
    vec3 col = vec3(0.);
    uv.y *= -1.;

    vec3 b = background_color(uv);

    col += b * circleGrid(uv * 5. + vec2(0.   + time*.2, 65. )) * (1. - abs(uv.y));
    col += b * circleGrid(uv * 10.+ vec2(10.  - time*.2, 984.)) * (1. - abs(uv.y)) * .5;
    col += b * circleGrid(uv * 20.+ vec2(-89. + time*.2, 7.  )) * (1. - abs(uv.y)) * .25;

    for (int i = 0; i < 6; ++i) {
        float fi = float(i);
        // bottom waves
        vec2 ruv = uv * rotate(.4*log(length(uv) + 1.));
        col += b * wave(ruv + vec2(.1 * fi + 2., -.7), 1.5 + .2 * fi)*.2;

        // middle waves
        ruv = uv * rotate(.2*log(length(uv) + 1.));
        col += b * wave(ruv + vec2(.1 * fi + 5., .0), 2. + .15 * fi);

        // top waves
        ruv = uv * rotate(-.4*log(length(uv) + 1.));
        ruv.x *= -1.;
        col += b * wave(ruv + vec2(.1 * fi + 10., .5), 1. + .2 * fi)*.1;
    }

    fragColor = vec4(col, 1.);
}
