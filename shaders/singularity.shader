/*
   "Singularity" by @XorDev

   A whirling blackhole.
   Feel free to code golf!

FabriceNeyret2: -19
dean_the_coder: -12
iq: -4

   Modified for naga/wgpu compatibility:
   - Expanded mat2(cos(vec4(...))) to explicit scalar construction
   - Extracted assignment-in-expression (a=dot(c,c))
   - Expanded mat2(1,1,vec2) to 4 scalars
 */
void mainImage(out vec4 O, vec2 F)
{
    //Iterator and attenuation (distance-squared)
    float i = .2, a;
    //Resolution for scaling and centering
    vec2 r = iResolution.xy,
         //Centered ratio-corrected coordinates
         p = ( F+F - r ) / r.y / .7,
         //Diagonal vector for skewing
         d = vec2(-1,1),
         //Blackhole center
         b = p - i*d;

    //Rotate and apply perspective
    vec2 dv = d/(.1 + i/dot(b,b));
    vec2 c = p * mat2(1.0, 1.0, dv.x, dv.y);

    // Compute attenuation
    a = dot(c,c);

    //Rotate into spiraling coordinates
    vec4 angles = .5*log(a) + iTime*i + vec4(0,33,11,0);
    vec4 cv = cos(angles);
    vec2 v = c * mat2(cv.x, cv.y, cv.z, cv.w) / i;

    //Waves cumulative total for coloring
    vec2 w = vec2(0.0);

    //Loop through waves
    for(; i++<9.; w += 1.+sin(v) )
        //Distort coordinates
        v += .7* sin(v.yx*i+iTime) / i + .5;
    //Acretion disk radius
    i = length( sin(v/.3)*.4 + c*(3.+d) );
    //Red/blue gradient
    O = 1. - exp( -exp( c.x * vec4(.6,-.4,-1,0) )
            //Wave coloring
            /  w.xyyx
            //Acretion disk brightness
            / ( 2. + i*i/4. - i )
            //Center darkness
            / ( .5 + 1. / a )
            //Rim highlight
            / ( .03 + abs( length(p)-.7 ) )
            );
}
