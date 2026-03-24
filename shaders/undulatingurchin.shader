#define A 9. // amplitude
#define T (iTime/3e2)
#define H(a) (cos(radians(vec3(180, 90, 0))+(a)*6.2832)*.5+.5)  // hue

float map(vec3 u, float v)  // sdf
{
    float t = T,     // speed
          l = 5.,    // loop to reduce clipping
          f = 1e10, i = 0., y, z;

    u.xy = vec2(atan(u.x, u.y), length(u.xy));  // polar transform
    u.x += t*v*3.1416*.7;  // counter rotation

    for (; i++<l;)
    {
        vec3 p = u;
        y = round((p.y-i)/l)*l+i;
        p.x *= y;
        p.x -= y*y*t*3.1416;
        p.x -= round(p.x/6.2832)*6.2832;
        p.y -= y;
        z = cos(y*t*6.2832)*.5 +.5;  // z wave
        f = min(f, max(length(p.xy), -p.z -z*A) -.1 -z*.2 -p.z/1e2);  // tubes
    }
    return f;
}

void mainImage( out vec4 C, vec2 U )
{
    vec2 R = iResolution.xy, j,
         M = iMouse.xy,
         m = (M -R/2.)/R.y;

    if (iMouse.z < 1. && M.x+M.y < 10.) m = vec2(0, .5);

    vec3 o = vec3(0, 0, -130.),  // camera
         u = normalize(vec3(U -R/2., R.y)),  // 3d coords
         c = vec3(0),
         p, k;

    float t = T,
          v = -o.z/3.,  // pattern scale
          i = 0., d = i,
          s, f, z, r;

    bool b;

    for (; i++<70.;)  // raymarch
    {
        p = u*d +o;
        p.xy /= v;           // scale down
        r = length(p.xy);    // radius
        z = abs(1. -r*r);    // z warp
        b = r < 1.;          // inside?
        if (b) z = sqrt(z);
        p.xy /= z+1.;        // spherize
        p.xy -= m;           // move with mouse
        p.xy *= v;           // scale back up
        p.xy -= cos(p.z/8. +t*3e2 +vec2(0, 1.5708) +z/2.)*.2;  // wave along z

        s = map(p, v);  // sdf

        r = length(p.xy);                  // new r
        f = cos(round(r)*t*6.2832)*.5+.5;  // multiples
        k = H(.2 -f/3. +t +p.z/2e2);       // color
        if (b) k = 1.-k;                   // flip color

        // this stuff can go outside the raymarch,
        // but accumulating it here produces softer edges
        c += min(exp(s/-.05), s)        // shapes
            * (f+.01)                    // shade pattern
            * min(z, 1.)                 // darken edges
            * sqrt(cos(r*6.2832)*.5 +.5) // shade between rows
            * k*k;                       // color

        d += s*clamp(z, .3, .9);  // smaller steps towards sphere edge
        if (s < 1e-3 || d > 1e3) break;
    }

    c += texture(iChannel0, u*d +o).rrr * vec3(0, .4, s)*s*z*.03;  // wavy aqua
    c += min(exp(-p.z -f*A)*z*k*.01/s, 1.);  // light tips

    j = p.xy/v +m;  // 2d coords
    c /= clamp(dot(j, j)*4., .04, 4.);  // brightness

    C = vec4(exp(log(c)/2.2), 1);
}
