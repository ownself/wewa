
/*
   I'm too lazy to copy the MIT license so like just do whatever you want with this shader 
   but like if you can leave a comment or something or hit me up, that'd be dope. thxxx

   -- int_45h

https://www.shadertoy.com/view/XdGfRR
 */

#define THRESHOLD .99
#define DUST
#define MIN_DIST .04
#define MAX_DIST 40.
#define MAX_DRAWS 40
#define AA 2
#define M_PI 3.1415926535897932384626433832795

float hash12(vec2 p)
{
    uvec2 q = uvec2(ivec2(p)) * uvec2(1597334673U, 3812015801U);
    uint n = (q.x ^ q.y) * 1597334673U;
    return float(n) * 2.328306437080797e-10;
}

float value2d(vec2 p);

// Based on xaot88's starfield: https://www.shadertoy.com/view/Md2SR3
float get_stars_rough(vec2 p)
{
    float s = smoothstep(THRESHOLD,1.,hash12(p));
    if (s >= THRESHOLD) 
        s = pow((s-THRESHOLD) / (1.-THRESHOLD), 10.); // Get s in 0-1 range
    return s;
}

// Instead of linear interpolation like in the original, I used cubic hermite interpolation
float get_stars(vec2 p, float a, float t)
{
    vec2 pg=floor(p), pc=p-pg, k=vec2(0,1);
    pc *= pc*pc*(3.-2.*pc);

    float s = mix(
            mix(get_stars_rough(pg+k.xx), get_stars_rough(pg+k.yx), pc.x),
            mix(get_stars_rough(pg+k.xy), get_stars_rough(pg+k.yy), pc.x),
            pc.y
            );
    return smoothstep(a,a+t, s)*pow(value2d(p*.1 + iTime)*.5+.5,8.3);
}

// This is stupid but I needed another value noise function (for part of the wave effect)
float value2d(vec2 p)
{
    vec2 pg=floor(p),pc=p-pg,k=vec2(0,1);
    pc*=pc*pc*(3.-2.*pc);
    return mix(
            mix(hash12(pg+k.xx),hash12(pg+k.yx),pc.x),
            mix(hash12(pg+k.xy),hash12(pg+k.yy),pc.x),
            pc.y
            );
}

// Shorthand for .5+.5*sin/cos(x)
float s5(float x) {return .5+.5*sin(x);}
float c5(float x) {return .5+.5*cos(x);}

// Sample the starfield at different sizes (to fake depth). What are stars other than really big balls of condensed dust...
float get_dust(vec2 p, vec2 size, float f)
{
    // Aspect ratio (so the stars look correct)
    vec2 ar = vec2(iResolution.x/iResolution.y,1);

    // Play with the power exponents to mess with the 
    // intensity of the sin/cos waves (for the stars, NOT for)
    // the translucent wave
    vec2 pp = p * size * ar;
    return 
        pow(   .64+.46*cos(p.x*6.28), 1.7)*    // keep stars at edges of the screen
                                               //pow(1.-c5(p.y*6.28+.2), 3.3)* // keep stars in middle row
        f*
        (
         get_stars(.1*pp+iTime*vec2(20.,-10.1),.11,.71)*4. + 
         get_stars(.2*pp+iTime*vec2(30.,-10.1),.1,.31)*5. + 
         get_stars(.32*pp+iTime*vec2(40.,-10.1),.1,.91)*2.
        );
}

float sdf(vec3 p)
{
    p*=2.;

    float o = 8.2 * sin( .05 * p.x + iTime * .25) + // Make the wave move up and down
        (.04*p.z) *            // Make waves more intense as they get further away
        sin(p.x*.11+iTime) *   // Add a sine wave in the x direction to make it more wavy
        2.*sin(p.z*.2+iTime) * // Add some waves in the z direction too, why not?
        value2d(              // Value noise (to make the waves more erratic and because i didn't use the sine waves at first)
                vec2(.03,.4)*p.xz+vec2(iTime*.5,0) // Stretch it out, make it longer in the y direction, then add movement
               );
    return abs(dot(p,normalize(vec3(0,1,0.05)))+2.5+o*.5);
}

vec3 norm(vec3 p)
{
    const vec2 k=vec2(1,-1);
    const float t=.001;
    return normalize(
            k.xyy*sdf(p+t*k.xyy) + 
            k.yyx*sdf(p+t*k.yyx) + 
            k.yxy*sdf(p+t*k.yxy) + 
            k.xxx*sdf(p+t*k.xxx)
            );
}

// Since we only need the alpha, return a float.
vec2 raymarch(vec3 o, vec3 d, float jitter)
{ 
    // Apply jitter directly to the starting distance to desynchronize rays
    float t = jitter * 2.0;
    float a = 0.0;
    float g = MAX_DIST;
    int dr = 0;

    for (int i = 0; i < 100; i++)
    {
        vec3 p = o + d * t;

        // Evaluate the SDF
        float ndt = sdf(p);

        // Track closest approach for your background dust effect
        g = (t > 10.0) ? min(g, abs(ndt)) : MAX_DIST;

        if (t >= MAX_DIST) break;

        // If we are close enough to the surface to be in the "glow" shell...
        if (abs(ndt) < MIN_DIST)
        {
            if (dr > MAX_DRAWS) break;
            dr++;

            // Smoothly fade the volume over depth
            float f = smoothstep(0.0, 0.3, (p.z * 0.9) / 100.0);

            // Accumulate a small, constant amount of glow
            a += 0.015 * f;
            t += 0.05;
        }
        else
        {
            // Outside the shell, take a normal SDF leap forward.
            // We multiply by 0.8 to ensure we don't accidentally leap entirely over the shell.
            t += abs(ndt) * 0.8;
        }
    }

    g /= 3.0;
    return vec2(a, max(1.0 - g, 0.0));
}


float dither(vec2 pos)
{
    // Generates a tiny pseudorandom value based on the exact pixel coordinate
    return fract(52.9829189 * fract(dot(pos, vec2(0.06711056, 0.00583715))));
}

vec3 render(vec2 U)
{
    vec2 ires = iResolution.xy;
    vec2 uv = U / ires;

    vec3 o = vec3(0.0);
    vec3 d = vec3((U - 0.5 * ires) / ires.y, 1.0);

    // Pass the random dither noise into the raymarcher
    vec2 mg = raymarch(o, d, dither(U));
    float m = mg.x;

    float q = iDate.w / 86400.0;
    // Your Day/Night color mixing
    float p = sin((2.*M_PI * q) + (.5*M_PI));
    vec3 l1 = mix(vec3(0.149, 0.471, 0.569), vec3(0.231, 0.231, 0.231), p);
    vec3 l2 = mix(vec3(0.075, 0.333, 0.412), vec3(0.129, 0.129, 0.129), p);
    vec3 l3 = mix(vec3(0.063, 0.329, 0.412), vec3(0.149, 0.149, 0.149), p);
    vec3 l4 = mix(vec3(0.169, 0.482, 0.580), vec3(0.251, 0.251, 0.251), p);

    vec3 c = mix(
            mix(l1, l2, uv.x),
            mix(l3, l4, uv.x),
            uv.y
            );

    c = mix(c, vec3(1.0), clamp(m, 0.0, 1.0));

#ifdef DUST
    c += get_dust(uv, vec2(2000.0), mg.y) * 0.3;
#endif

    return c;
}

void mainImage(out vec4 O, in vec2 U)
{
    vec3 totalColor = vec3(0.0);

#if AA > 1
    // Run the sub-pixel sampling loop
    for (int m = 0; m < AA; m++) {
        for (int n = 0; n < AA; n++) {
            // Offset the ray slightly for each sample
            vec2 offset = vec2(float(m), float(n)) / float(AA) - 0.5;
            totalColor += render(U + offset);
        }
    }
    totalColor /= float(AA * AA);
#else
    // Skip the loop if AA is set to 1
    totalColor = render(U);
#endif

    // Apply the dither to the final averaged color (must happen outside the AA loop)
    totalColor += (dither(U) - 0.5) / 255.0;

    O = vec4(totalColor, 1.0);
}
