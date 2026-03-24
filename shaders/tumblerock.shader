// inspo was https://www.shadertoy.com/view/NcS3Wz
// i just wanted to tinker with some raymarch ideas
// reducing rayDir.z by a small percentage of the iterator
// comment out the "D.z -= ..." to see

void mainImage(out vec4 o, vec2 u) {
    float i,a,d,s,t=iTime;
    vec3  p = iResolution,
          D = normalize(vec3(u = (u+u-p.xy)/p.y, 1));    

    vec2 v = .2*sin(t) + u + u.yx*.8 + vec2(1.4,.3);
    for(o*=i; i++<1e2; ) {
        p = D *d;
        p.z += t * 7e1;


        D.z -= i*.000025,

            p.xy *= mat2(cos(t/6.+p.z/2e2+vec4(0,33,11,0))),

            s = mix(1e2 - abs(p.x),
                    abs(2e1*dot(sin(p/4e1), sin(p.yzx/5e1)))
                    -abs(1e1*dot(sin(p/2e1), sin(p.yzx/2e1)))
                    -abs(2e1*dot(sin(p/3e1), sin(p.zxy/4e1))), .6);

        for (a = .01; a < 4.; a += a+a)
            s -= abs(dot(sin(.2*p / a), vec3(a*3.)));

        d += s *= .5;

        s = max(s, .005);

        o += s * i * (1.+cos(i*.5+vec4(2,1,0,0))) * 4.
            +  s * i * vec4(4,2,1,0)
            + vec4(4,5,6,0)/s
            + 3e1*vec4(1,2,6,0)/length(v);
    }
    o = tanh(o*o / 5e9 );
}
