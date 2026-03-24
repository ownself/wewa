void mainImage( out vec4 fragColor, in vec2 fragCoord )
{
    float t = iTime * .2;

    vec2 uv = ( fragCoord * 0.25 ) / iResolution.y;
    vec2 st = vec2(
            length( uv ) * 1.5,
            atan( uv.y, uv.x )
            );

    st.y += st.x * 1.1;

    float x = fbm3d(
            vec3(
                sin( st.y ),
                cos( st.y ),
                pow( st.x, .3 ) + t * .1
                ),
            3
            );
    float y = fbm3d(
            vec3(
                cos( 1. - st.y ),
                sin( 1. - st.y ),
                pow( st.x, .5 ) + t * .1
                ),
            4
            );

    float r = fbm3d(
            vec3(
                x,
                y,
                st.x + t * .3
                ),
            5
            );
    r = fbm3d(
            vec3(
                r - x,
                r - y,
                r + t * .3
                ),
            6
            );

    float c = ( r + st.x * 5. ) / 6.;

    fragColor = vec4(
            smoothstep( .3, .4, c ),
            smoothstep( .4, .55, c ),
            smoothstep( .2, .55, c ),
            1.0
            );
}
