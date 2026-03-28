// v1.2

#define t iTime*2.
#define SIZE 30.
#define COLOR_CYCLE_SPEED 0.3
#define COLOR_CYCLE_PERIOD 10.0

#define col_red vec3(193.,41.,46.)/255.
#define col_yellow vec3(241.,211.,2.)/255.
#define col_blue vec3(41.,128.,193.)/255.
#define col_green vec3(46.,193.,89.)/255.

vec2 ran(vec2 uv) {
    uv *= vec2(dot(uv,vec2(127.1,311.7)),dot(uv,vec2(227.1,521.7)) );
    return 1.0-fract(tan(cos(uv)*123.6)*3533.3)*fract(tan(cos(uv)*123.6)*3533.3);
}
vec2 pt(vec2 id) {
    vec2 phase = t*(ran(id+.5)-0.5)+ran(id-20.1)*8.0;
    phase = mod(phase, 6.28318530718);
    return sin(phase)*0.5;
}


void mainImage( out vec4 fragColor, in vec2 fragCoord )
{
    vec2 uv = (fragCoord-.5*iResolution.xy)/iResolution.x;
    vec2 off = iTime/vec2(50.,30.);
    uv += off;
    uv *= SIZE;

    vec2 gv = fract(uv)-.5;
    vec2 id = floor(uv);

    float mindist = 1e9;
    vec2 vorv;
    for(float i=-1.;i<=1.;i++) {
        for(float j=-1.;j<=1.;j++) { 
            vec2 offv = vec2(i,j);
            float dist = length(gv+pt(id+offv)-offv);
            if(dist<mindist){
                mindist = dist;
                vorv = (id+pt(id+offv)+offv)/SIZE-off;
            }
        }
    }

    // without color cycling
    // vec3 col = mix(col1,col2,clamp(vorv.x*2.2+vorv.y,-1.,1.)*0.5+0.5);

    // with color cycling
    float colorPhase = sin(iTime * COLOR_CYCLE_SPEED * 6.28318530718 / COLOR_CYCLE_PERIOD) * 0.5 + 0.5;
    vec3 leftCol = mix(col_red, col_blue, colorPhase);
    vec3 rightCol = mix(col_yellow, col_green, colorPhase);
    vec3 col = mix(leftCol, rightCol, clamp(vorv.x*2.2+vorv.y, -1., 1.)*0.5+0.5);

    fragColor = vec4(col,1.0);

    /*
       fragColor += vec4(vec3(smoothstep(0.08,0.05,gv.x+pt(id).x)),0.0);
       fragColor -= vec4(vec3(smoothstep(0.05,0.03,gv.x+pt(id).x)),0.0);
     */
}
