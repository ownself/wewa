/**

License: Creative Commons Attribution-NonCommercial-ShareAlike 3.0 Unported License

Tile Warp Experiment PT3
Tweaked a few things, new colors and movement - fun!

03/07/2026  @byt3_m3chanic

 */

#define R     iResolution
#define T     iTime
#define M     iMouse

#define PI    3.141592653
#define PI2   6.283185307

#define N     5.

mat2 rot(float a) { return mat2(cos(a),sin(a),-sin(a),cos(a)); }

float hash21(vec2 a) { 
    a.y=mod(a.y,N*4.);
    a.x=mod(a.x,N*4.);
    return fract(sin(dot(a, vec2(27.609, 57.583)))*43758.5453); 
}

float box( in vec2 p, in vec2 b) {
    vec2 q = abs(p)-b;
    return min(max(q.x,q.y),0.0) + length(max(q,0.0));
}

const float size = 4.;
const float hlf = size/2.;
const float dbl = size*2.;

vec3 hue(float t){ 
    return .4+.4*cos( PI2*t +vec3(2,1,0)*vec3(1,.75,.8)); 
}

void mainImage( out vec4 fragColor, in vec2 F )
{    

    vec2 uv = (2.*F.xy-R.xy)/max(R.x,R.y);
    vec2 suv = uv;

    uv *= rot(-T*.09);
    uv = -vec2(log(length(uv)),atan(uv.y,uv.x));
    uv /= PI;
    uv *= N;

    float px = fwidth(uv.x*PI)/PI;
    uv.x += T*.25;

    uv.y += .04*sin(uv.x*10.+T*2.5);
    vec2 p = uv*size, q;
    vec3 C = vec3(0);

    float sp =.45, sl =hlf*.975;
    float t = 1e5,id,fd;

    for(int i = 0; i<2; i++) {
        if(i==1) p.y+=.5;
        float cnt = i<1 ? size : dbl;
        q = vec2(p.x-cnt,p.y);
        id = floor(q.x/dbl) + .5;
        q.x -= (id)*dbl;

        fd = floor(q.y)+float(i);
        q.y = fract(q.y)-.5;
        t = box(q,vec2(sl,sp));
        float tc = length(q-vec2(sl,0))-sp;
        float bc = length(vec2(q.x,abs(q.y)-sp)+vec2(sl,0))-.5;

        t = min(t,tc);
        t = max(t,-bc);

        float hs = hash21(vec2(id,fd));
        float fs = fract(hs*4785.312);

        vec3 h = mod(fd+float(i),2.)==0.? vec3(.9):hue(hs+T*.1);
        vec3 CC = mix(h,h*.3,.75-q.x*.4);
        C = mix(C,CC,smoothstep(px,-px,t));
    }

    float m = length(suv*4.)-.25;

    C = mix(C,C*.1,smoothstep(.6,.0,m));

    C = pow(C,vec3(.4545));
    fragColor = vec4(C,1.0);
}

