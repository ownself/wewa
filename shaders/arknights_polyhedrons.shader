// Thanks to @FabriceNeyret2!
/*
   float L( vec3 p, vec3 a, vec3 b ) {
   p -= a, b += (a-b) * max(cos(iTime),0.) - a;
   return length( p - b*clamp( dot(p,b)/dot(b,b), 0., 1. ) );
   }
 */

#define L(p,b) length( p - (b)* clamp( dot(p,b)/dot(b,b), 0., T ) )

#define N normalize
#define P(b,a) n = N(cross(b,a)),     \
                   q -= 2.*n* max(0., dot(q,n) )

void mainImage( out vec4 O, vec2 u ) {
    vec2  R = iResolution.xy, p;
    float w = 1./min(R.x,R.y),
          m = iMouse.z > 0. 
              ? iMouse.x / R.x / .1
              :.5*iTime,
          T = 1. - max(cos(iTime),0.),
          t, h, v, i;

    p = w * ( u+u - R );

    vec3 P = 25.*vec3(sin(m), .2, cos(m)), q,n,
         Y = N(-P),
         X = N(vec3(-Y.z,0,Y)), // Y -> Y.x: GLSL compiler truncation
         D = N( p.x*X + p.y*cross(X,Y) + 1.8*Y ),
         a = vec3( 0, 3.75, 6 ), b = a.zxy, c = a.yzx,
         d = a+b, e = c+b, f = c+a;

    for(; i++ < 80. && t < 1e2; v += w/h/h)       
        q = abs(P += h*D), // h = sdfIcosahedron(P)
          P(b,a), P(c,b), P(a,c), 
          P(e,d), P(f,e), P(d,f),
          t += h = min( L(q-f, d-f ),
                  min( L(q-d, e-d ),
                      L(q-e, f-e ))) -.02;      

    O = tanh(sqrt( v * vec4(2,2,1.+cos(4.+sin(iTime*.1)),1)/2.7 ));
}

// 20260317
/*
   void mainImage( out vec4 fragColor, in vec2 fragCoord )
   {
   vec2 p = (2.0*fragCoord-iResolution.xy)/min(iResolution.x,iResolution.y);

   float an = 10.0 * iMouse.x / iResolution.x;
   if (iMouse.z <= 0.0) an = 0.5*iTime;

   vec3 ta = vec3(0.0, 0.0, 0.0); 
   vec3 ro = ta + vec3(25.0 * sin(an), 5.0, 25.0 * cos(an)); 

   vec3 ww = normalize(ta-ro);
   vec3 uu = normalize(cross(ww,vec3(0.0,1.0,0.0)));
   vec3 vv = normalize(cross(uu,ww));
   vec3 rd = normalize(p.x*uu + p.y*vv + 1.8*ww);


   float t = 0.0;
   float glow = 0.0;

   for(int i=0; i<80; i++)
   {
   vec3 pos = ro + t*rd;
   float h = sdfIcosahedron(vec3(0),pos,15.0,iTime); 

   glow += PIXEL / (0.001+h*h); 
   t += h;
   if(t > 100.0) break;
   }
   vec2 uv = fragCoord/iResolution.xy;
   vec3 col = 0.2*(0.5 + 0.5*cos(vec3(0,0,4.0+sin(iTime*0.1))))*glow;

   col = pow(col, vec3(0.4545));

   fragColor = vec4(tanh(col),1.0);
   }
 */
