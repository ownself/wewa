#define PI 3.141592653
#define AA 2.

vec3 colMap(float v) {
    v+=PI;
    return vec3(
            sin(sin(v-.6)),
            sin(sin(v)),
            sin(sin(v+.8))
            );
}

void mainImage( out vec4 fragColor, in vec2 fragCoord ) {
    float aaFract = 1./AA;
    fragColor = vec4(0.);
    vec2 mouz = iMouse.xy;
    if(mouz.x == 0. || mouz.y == 0.){
        mouz.x=iResolution.x*.45;
        mouz.y=iResolution.y*.45;
    }
    for(float aa=0.; aa<1.; aa+=aaFract){
        for(float bb=0.; bb<1.; bb+=aaFract){
            vec2 uv =  ( (fragCoord.yx + vec2(aa,bb) ) -.5* iResolution.yx ) / iResolution.x;
            uv*=(-cos(iTime*.1)+1.3)*.28;
            uv+=vec2(.1,.65);

            vec2 c =uv;
            vec2 z=c;
            float l=0.;
            float sum=length(z);
            vec2 newZ;
            for(int i=0;i<40;i++){
                c+=(mouz.yx/iResolution.yx  -.5)*-.01*float(i);
                newZ=vec2(-z.y*z.y+z.x*z.x, 2.*z.x*z.y)+c;
                sum+=length(newZ-z);
                z=newZ;
                l=length(z);
                if(l>2.) break;
            }
            vec2 dir=z-c;
            vec3 col = vec3(dir, .0);

            uv.x=mod((atan(dir.y,dir.x)/PI*.5+.5)*6.+(iTime+sin(iTime)*.9)*4., 1.);
            uv.y=mod(length(dir*.5),1.);

            col=colMap(sum*.2-l*.1+iTime*.1).gbr;

            fragColor += vec4(col.grb, 1.0) * aaFract*aaFract;
        }
    }
}
