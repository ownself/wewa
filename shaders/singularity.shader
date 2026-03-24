/*
   "Singularity" by @XorDev

   A whirling blackhole.
   Feel free to code golf!

FabriceNeyret2: -19
dean_the_coder: -12
iq: -4
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
         b = p - i*d,
         //Rotate and apply perspective
         c = p * mat2(1, 1, d/(.1 + i/dot(b,b))),
         //Rotate into spiraling coordinates
         v = c * mat2(cos(.5*log(a=dot(c,c)) + iTime*i + vec4(0,33,11,0)))/i,
         //Waves cumulative total for coloring
         w;

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



//Original [432]
/*
   void mainImage(out vec4 O,in vec2 F)
   {
   vec2 p=(F*2.-iResolution.xy)/(iResolution.y*.7),
   d=vec2(-1,1),
   c=p*mat2(1,1,d/(.1+5./dot(5.*p-d,5.*p-d))),
   v=c;
   v*=mat2(cos(log(length(v))+iTime*.2+vec4(0,33,11,0)))*5.;
   vec4 o=vec4(0);
   for(float i;i++<9.;o+=sin(v.xyyx)+1.)
   v+=.7*sin(v.yx*i+iTime)/i+.5;
   O=1.-exp(-exp(c.x*vec4(.6,-.4,-1,0))/o
   /(.1+.1*pow(length(sin(v/.3)*.2+c*vec2(1,2))-1.,2.))
   /(1.+7.*exp(.3*c.y-dot(c,c)))
   /(.03+abs(length(p)-.7))*.2);
   }*/
