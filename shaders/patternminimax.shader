// Plasma Orb - Clean & Simple with 2x AA
vec4 render(vec2 u, float t)
{
    vec4 o = vec4(0.0);

    for(float i = 1.0; i < 12.0; i++)
    {
        o += 0.015 / length(sin(u*4.0 + i*1.5 + t*2.0)) 
            * (cos(i*0.4 + vec4(4,5,6,0) + t) + 1.0);

        u = (u + 0.3*sin(u.yx*4.0 + i + t*1.5)).yx;
    }

    o *= 1.0 - length(u)*0.3;
    o = tanh(o * 1.5);

    return o;
}

void mainImage(out vec4 O, vec2 C)
{
    float t = iTime;
    vec2 r = iResolution.xy;
    vec2 u = (C*2.0 - r) / r.y / 0.4;

    float s = 0.5; // AA sample offset
    O = (render(u + vec2(s,-s)/r.y, t) + 
            render(u + vec2(s,s)/r.y, t) + 
            render(u + vec2(-s,-s)/r.y, t) + 
            render(u + vec2(-s,s)/r.y, t)) * 0.25;
}
