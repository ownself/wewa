// inspo was https://www.shadertoy.com/view/NcS3Wz
// i just wanted to tinker with some raymarch ideas
// reducing rayDir.z by a small percentage of the iterator
// comment out the "D.z -= ..." to see
//
// Modified for naga/wgpu compatibility:
// - Separated assignment-in-expression (u = ... inside vec3 constructor)
// - Expanded mat2(cos(...+vec4(0,33,11,0))) to explicit scalar construction
// - Split chained assignment d += s *= .5 into separate statements
// - Converted comma operators to proper statements

void mainImage(out vec4 o, vec2 u) {
    float i,a,d,s,t=iTime;
    vec3  res = iResolution;

    u = (u+u-res.xy)/res.y;
    vec3  D = normalize(vec3(u, 1.0));
    vec3  p;

    vec2 v = .2*sin(t) + u + u.yx*.8 + vec2(1.4,.3);
    o = vec4(0.0);
    for(i = 0.0; i++<1e2; ) {
        p = D *d;
        p.z += t * 7e1;

        D.z -= i*.000025;

        vec4 angles = t/6.+p.z/2e2+vec4(0,33,11,0);
        vec4 cv = cos(angles);
        p.xy *= mat2(cv.x, cv.y, cv.z, cv.w);

        s = mix(1e2 - abs(p.x),
                abs(2e1*dot(sin(p/4e1), sin(p.yzx/5e1)))
                -abs(1e1*dot(sin(p/2e1), sin(p.yzx/2e1)))
                -abs(2e1*dot(sin(p/3e1), sin(p.zxy/4e1))), .6);

        for (a = .01; a < 4.; a += a+a)
            s -= abs(dot(sin(.2*p / a), vec3(a*3.)));

        s *= .5;
        d += s;

        s = max(s, .005);

        o += s * i * (1.+cos(i*.5+vec4(2,1,0,0))) * 4.
            +  s * i * vec4(4,2,1,0)
            + vec4(4,5,6,0)/s
            + 3e1*vec4(1,2,6,0)/length(v);
    }
    o = tanh(o*o / 5e9 );
}
