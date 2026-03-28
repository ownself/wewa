// Thanks to @FabriceNeyret2!
//
// Modified for naga/wgpu compatibility:
// - Expanded macro P(b,a) inline (side-effect macro modifying q,n)
// - Split abs(pos += h*D) into separate statements
// - Split chained assignment t += h = min(...)
// - Converted comma operators to proper statements
// - Fixed vec3(-Y.z,0,Y) truncation to vec3(-Y.z,0.0,Y.x)

#define L(lp,lb) length( lp - (lb)* clamp( dot(lp,lb)/dot(lb,lb), 0., T ) )

void mainImage( out vec4 O, vec2 u ) {
    vec2  R = iResolution.xy;
    float w = 1./min(R.x,R.y),
          m = iMouse.z > 0.
              ? iMouse.x / R.x / .1
              :.5*iTime,
          T = 1. - max(cos(iTime),0.),
          t = 0.0, h = 0.0, v = 0.0, i = 0.0;

    vec2 p;
    p = w * ( u+u - R );

    vec3 pos = 25.*vec3(sin(m), .2, cos(m)), q, n,
         Y = normalize(-pos),
         X = normalize(vec3(-Y.z, 0.0, Y.x)),
         D = normalize( p.x*X + p.y*cross(X,Y) + 1.8*Y ),
         a = vec3( 0, 3.75, 6 ), b = a.zxy, c = a.yzx,
         d = a+b, e = c+b, f = c+a;

    for(; i++ < 80. && t < 1e2; ) {
        pos += h*D;
        q = abs(pos);

        // P(b,a) expanded: n = normalize(cross(b,a)); q -= 2.*n*max(0., dot(q,n));
        n = normalize(cross(b,a)); q -= 2.*n* max(0., dot(q,n));
        n = normalize(cross(c,b)); q -= 2.*n* max(0., dot(q,n));
        n = normalize(cross(a,c)); q -= 2.*n* max(0., dot(q,n));
        n = normalize(cross(e,d)); q -= 2.*n* max(0., dot(q,n));
        n = normalize(cross(f,e)); q -= 2.*n* max(0., dot(q,n));
        n = normalize(cross(d,f)); q -= 2.*n* max(0., dot(q,n));

        h = min( L(q-f, d-f),
               min( L(q-d, e-d),
                   L(q-e, f-e))) - .02;
        t += h;
        v += w/h/h;
    }

    O = tanh(sqrt( v * vec4(2,2,1.+cos(4.+sin(iTime*.1)),1)/2.7 ));
}
