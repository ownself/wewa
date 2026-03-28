//
// Ascend by bµg
// License: CC BY-NC-SA 4.0
//
// Portage of: https://art.pkh.me/2026-02-22-ascend.htm (463 chars)
//
// -13 by FabriceNeyret2
//
// Modified for naga/wgpu compatibility:
// - Expanded macro V inline (chained assignments, comma operators)
// - Expanded macro N inline
// - Converted comma operators in for-loop to proper statements
// - Separated assignments-in-expression

void mainImage(out vec4 O, vec2 P) {
    vec3 o = vec3(0.0);
    vec3 p, q;
    vec3 R = iResolution;

    float i = 0.0, d = 0.0, a = 0.0, l = 0.0, k = 0.0, s = 0.0, x = 0.0;

    for (; i++ < 1e2; ) {
        // Inner loop setup (was for-loop init with comma operators)
        p = normalize(vec3(P+P,R.y)-R)*i*.05;
        p.z -= 3.;
        q = p - vec3(1.5,.7,0);
        s = length(q);
        q.y = p.y - min(p.y,.7);
        l = length(q);
        p.y += iTime;
        d = min(length(p.xz), 1.-p.z);

        // Inner loop body (noise accumulation)
        for (a = .01; a < 3.; a += a) {
            p.zy *= .1*mat2(8,6,-6,8);
            d -= abs(dot(sin(p/a*4.), p-p+a*.2));
            l -= abs(dot(sin(p/a*5.), p-p+a*.01));
        }

        // First V * mix(...) expansion:
        //   V = d = min(d, 0.), k += a = d*k-d, o += a / exp(s*1.3) * (1.+d)
        //   * mix(vec3(0,1.5,3), q=vec3(3,1,.7), x=max(2.-l,0.)*.8)
        d = min(d, 0.);
        a = d*k - d;
        k += a;
        q = vec3(3,1,.7);
        x = max(2.-l, 0.) * .8;
        o += a / exp(s*1.3) * (1.+d) * mix(vec3(0,1.5,3), q, x);

        // d = l
        d = l;

        // Second V * q * 20. expansion
        d = min(d, 0.);
        a = d*k - d;
        k += a;
        o += a / exp(s*1.3) * (1.+d) * q * 20.;

        // Final accumulation
        o += (x - x*k) / s / 4e2;
    }

    O.rgb = tanh(o);
}
