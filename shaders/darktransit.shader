// MIT License

// low tech tunnel
// 28 steps

/*
   @FabriceNeyret2 -40 chars
   → 611 (from 651)!

   Further golfing below shader code

 */

// Modified for naga/wgpu compatibility:
// - Converted comma operators in for-loop body to proper statements
// - Split chained assignment d += s = min(e,...)
// - Fixed vec3(Z.z,0,-Z) truncation to vec3(Z.z,0.0,-Z.x)
// - Replaced o *= i zeroing with o = vec4(0.0)

#define T        iTime*4. + 5. + 5.*sin(iTime*.3)         //
#define P(z)     vec3( 12.* cos( (z)*vec2(.1,.12) ) , z)  //
#define A(F,H,K) abs(dot( sin(F*p*K), H +p-p )) / K

void mainImage(out vec4 o, in vec2 u) {

    float t,s,i,d,e;
    vec3  c,r = iResolution;

    u = ( u - r.xy/2. ) / r.y;            // scaled coords
    if (abs(u.y) > .375) { o = vec4(0.0); return;}// cinema bars


    vec3  p = P(T),                       // setup ray origin, direction, and look-at
          Z = normalize( P(T+4.) - p),
          X = normalize(vec3(Z.z, 0.0, -Z.x)),
          D = vec3(u, 1) * mat3(-X, cross(X, Z), Z);

    c = vec3(0.0);
    s = 0.0;
    d = 0.0;

    for(i = 0.0; i++ < 28. && d < 3e1 ; ) {
        p += D * s;                      // march
        X = P(p.z);                      // get path
        t = sin(iTime);                  // store sine of iTime (not T)
        e = length(p - vec3(             // orb (sphere with xyz offset by t)
                    X.x + t,
                    X.y + t*2.,
                    6.+T + t*2.))-.01;
        s = cos(p.z*.6)*2.+ 4.           // tunnel with modulating radius
            - min( length(p.xy - X.x - 6.)
                    , length((p-X).xy) )
            + A(  4., .25, .1)             // noise, large scoops
            + A(T+8., .22, 2.);            // noise, detail texture
                                           // (remove "T+" if you don't like the texture moving)
        s = min(e,.01+.3*abs(s));        // clamp step
        d += s;                          // accumulate distance

        c += 1./s + 1e1*vec3(1,2,5)/max(e, .6);
    }

    o.rgb = c*c/1e6;                     // adjust brightness and saturation
}
