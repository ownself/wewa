// Copyright Inigo Quilez, 2020 - https://iquilezles.org/
// I am the sole copyright owner of this Work. You cannot
// host, display, distribute or share this Work neither as
// is or altered, in any form including physical and
// digital. You cannot use this Work in any commercial or
// non-commercial product, website or project. You cannot
// sell this Work and you cannot mint an NFTs of it. You
// cannot use this Work to train AI models. I share this
// Work for educational purposes, you can link to it as
// an URL, proper attribution and unmodified screenshot,
// as part of your educational material. If these
// conditions are too restrictive please contact me.

// Everybody has to implement an hexagonal grid. This it mine.
// It does raycasting on it, efficiently (just a few muls per step) and robustly
// (works in integers). Each cell is visited only once and in the right order.
// Based on https://www.shadertoy.com/view/WtSBWK Check castRay() in line 92.

// That, plus the fact the ambient occlusion is analytical means this shader should
// run smoothly even on a crappy phone. It does on mine!


#if HW_PERFORMANCE==0
#define AA 1
#else
// make this bigger if you have a storng PC
#define AA 2
#endif


// -----------------------------------------
// mod3 - not as trivial as you first though
// -----------------------------------------
int mod3( int n )
{
    return (n<0) ? 2-((2-n)%3) : n%3;

    // Some methods of computing mod3:            // PC-WebGL  Native-OpenGL  Android WebGL
    //                                            // --------  -------------  -------
    // 1.  return (n<0) ? 2-((2-n)%3) : n%3;      //    Ok        Ok            Ok 
    // 2.  return int((uint(n)+0x80000001U)%3u);  //    Ok        Ok            Broken
    // 3.  n %= 3; return (n<0)?n+3:n;            //    Ok        Broken        Ok
    // 4.  n %= 3; n+=((n>>31)&3); return n;      //    Ok        Broken        Ok
    // 5.  return ((n%3)+3)%3;                    //    Ok        Broken        Ok
    // 6.  return int[](1,2,0,1,2)[n%3+2];        //    Ok        Broken        Ok
}

// --------------------------------------
// hash by Hugo Elias)
// --------------------------------------
int hash( int n ) { n=(n<<13)^n; return n*(n*n*15731+789221)+1376312589; }


// --------------------------------------
// basic hexagon grid functions
// --------------------------------------
ivec2 hexagonID( vec2 p ) 
{
    const float k3 = 1.732050807;
    vec2 q = vec2( p.x, p.y*k3*0.5 + p.x*0.5 );

    ivec2 pi = ivec2(floor(q));
    vec2  pf =       fract(q);

    int v = mod3(pi.x+pi.y);

    int   ca = (v<1)?0:1;
    int   cb = (v<2)?0:1;
    ivec2 ma = (pf.x>pf.y)?ivec2(0,1):ivec2(1,0);

    ivec2 id = pi + ca - cb*ma;

    return ivec2( id.x, id.y - (id.x+id.y)/3 );
}

vec2 hexagonCenFromID( in ivec2 id )
{
    const float k3 = 1.732050807;
    return vec2(float(id.x),float(id.y)*k3);
}

// ---------------------------------------------------------------------
// the height function. yes, i know reading from a video texture is cool
// ---------------------------------------------------------------------
const float kMaxH = 6.0;

float map( vec2 p, in float time ) 
{
    p *= 0.5;
    float f = 0.5+0.5*sin(0.53*p.x+0.5*time+1.0*sin(p.y*0.24))*
        sin(0.13*p.y+time);
    f*= 0.75+0.25*sin(1.7*p.x+1.32*time)*sin(1.3*p.y+time*2.1);	
    return kMaxH*(0.005+0.995*f);
}

// --------------------------------------------------
// raycast. this function is the point of this shader
// --------------------------------------------------
vec4 castRay( in vec3 ro, in vec3 rd, in float time, 
        out ivec2 outPrismID, out int outFaceID )
{
    ivec2 hid = hexagonID(ro.xz);

    vec4 res = vec4( -1.0, 0.0, 0.0, 0.0 );

    const float k3 = 0.866025;
    const vec2 n1 = vec2( 1.0,0.0);
    const vec2 n2 = vec2( 0.5,k3);
    const vec2 n3 = vec2(-0.5,k3);

    float d1 = 1.0/dot(rd.xz,n1);
    float d2 = 1.0/dot(rd.xz,n2);
    float d3 = 1.0/dot(rd.xz,n3);
    float d4 = 1.0/rd.y;

    float s1 = (d1<0.0)?-1.0:1.0;
    float s2 = (d2<0.0)?-1.0:1.0;
    float s3 = (d3<0.0)?-1.0:1.0;
    float s4 = (d4<0.0)?-1.0:1.0;

    ivec2 i1 = ivec2( 2,0); if(d1<0.0) i1=-i1;
    ivec2 i2 = ivec2( 1,1); if(d2<0.0) i2=-i2;
    ivec2 i3 = ivec2(-1,1); if(d3<0.0) i3=-i3;

    // traverse hexagon grid (in 2D)
    bool found = false;
    vec2 t1, t2, t3, t4;
    for( int i=0; i<100; i++ ) 
    {
        // fetch height for this hexagon
        vec2  ce = hexagonCenFromID( hid );
        float he = 0.5*map(ce, time);

        // compute ray-hexaprism intersection
        vec3 oc = ro - vec3(ce.x,he,ce.y);
        t1 = (vec2(-s1,s1)-dot(oc.xz,n1))*d1;
        t2 = (vec2(-s2,s2)-dot(oc.xz,n2))*d2;
        t3 = (vec2(-s3,s3)-dot(oc.xz,n3))*d3;
        t4 = (vec2(-s4,s4)*he-oc.y)*d4;
        float tN = max(max(t1.x,t2.x),max(t3.x,t4.x));
        float tF = min(min(t1.y,t2.y),min(t3.y,t4.y));
        if( tN<tF && tF>0.0 )
        {
            found = true;
            break;
        }

        // move to next hexagon
        if( t1.y<t2.y && t1.y<t3.y ) hid += i1;
        else if( t2.y<t3.y )              hid += i2;
        else                              hid += i3;
    }

    if( found )
    {
        {res=vec4(t1.x,s1*vec3(n1.x,0,n1.y)); outFaceID=(d1<0.0)?-1: 1;}
        if( t2.x>res.x ) {res=vec4(t2.x,s2*vec3(n2.x,0,n2.y)); outFaceID=(d2<0.0)?-2: 2;}
        if( t3.x>res.x ) {res=vec4(t3.x,s3*vec3(n3.x,0,n3.y)); outFaceID=(d3<0.0)?-3: 3;}
        if( t4.x>res.x ) {res=vec4(t4.x,s4*vec3( 0.0,1,0));    outFaceID=(d4<0.0)? 4:-4;}

        outPrismID = hid;
    }

    return res;
}

// -------------------------------------------------------------------------
// same as above, but simpler sinec we don't need the normal and primtive id
// --------------------------------------------------------------------------
float castShadowRay( in vec3 ro, in vec3 rd, in float time )
{
    float res = 1.0;

    ivec2 hid = hexagonID(ro.xz);

    const float k3 = 0.866025;
    const vec2 n1 = vec2( 1.0,0.0);
    const vec2 n2 = vec2( 0.5,k3);
    const vec2 n3 = vec2(-0.5,k3);

    float d1 = 1.0/dot(rd.xz,n1);
    float d2 = 1.0/dot(rd.xz,n2);
    float d3 = 1.0/dot(rd.xz,n3);
    float d4 = 1.0/rd.y;

    float s1 = (d1<0.0)?-1.0:1.0;
    float s2 = (d2<0.0)?-1.0:1.0;
    float s3 = (d3<0.0)?-1.0:1.0;
    float s4 = (d4<0.0)?-1.0:1.0;

    ivec2 i1 = ivec2( 2,0); if(d1<0.0) i1=-i1;
    ivec2 i2 = ivec2( 1,1); if(d2<0.0) i2=-i2;
    ivec2 i3 = ivec2(-1,1); if(d3<0.0) i3=-i3;

    vec2 c1 = (vec2(-s1,s1)-dot(ro.xz,n1))*d1;
    vec2 c2 = (vec2(-s2,s2)-dot(ro.xz,n2))*d2;
    vec2 c3 = (vec2(-s3,s3)-dot(ro.xz,n3))*d3;

    // traverse regular grid (2D)	
    for( int i=0; i<8; i++ ) 
    {
        vec2  ce = hexagonCenFromID( hid );
        float he = 0.5*map(ce, time);

        vec2 t1 = c1 + dot(ce,n1)*d1;
        vec2 t2 = c2 + dot(ce,n2)*d2;
        vec2 t3 = c3 + dot(ce,n3)*d3;
        vec2 t4 = (vec2(1.0-s4,1.0+s4)*he-ro.y)*d4;

        float tN = max(max(t1.x,t2.x),max(t3.x,t4.x));
        float tF = min(min(t1.y,t2.y),min(t3.y,t4.y));
        if( tN < tF && tF > 0.0)
        {
            res = 0.0;
            break;
        }

        if( t1.y<t2.y && t1.y<t3.y ) hid += i1;
        else if( t2.y<t3.y )              hid += i2;
        else                              hid += i3;
    }

    return res;
}

// -------------------------------------------------------------------------
// analytic occlusion of a quad and an hexagon
// -------------------------------------------------------------------------

float macos(float x ) { return acos(clamp(x,-1.0,1.0));}

float occlusionQuad( in vec3 pos, in vec3 nor, 
        in vec3 v0, in vec3 v1,
        in vec3 v2, in vec3 v3 ) 
{
    v0 = normalize(v0-pos);
    v1 = normalize(v1-pos);
    v2 = normalize(v2-pos);
    v3 = normalize(v3-pos);
    float k01 = dot( nor, normalize( cross(v0,v1)) ) * macos( dot(v0,v1) );
    float k12 = dot( nor, normalize( cross(v1,v2)) ) * macos( dot(v1,v2) );
    float k23 = dot( nor, normalize( cross(v2,v3)) ) * macos( dot(v2,v3) );
    float k30 = dot( nor, normalize( cross(v3,v0)) ) * macos( dot(v3,v0) );

    return abs(k01+k12+k23+k30)/6.283185;
}

float occlusionHexagon( in vec3 pos, in vec3 nor, 
        in vec3 v0, in vec3 v1,
        in vec3 v2, in vec3 v3,
        in vec3 v4, in vec3 v5) 
{
    v0 = normalize(v0-pos);
    v1 = normalize(v1-pos);
    v2 = normalize(v2-pos);
    v3 = normalize(v3-pos);
    v4 = normalize(v4-pos);
    v5 = normalize(v5-pos);
    float k01 = dot( nor, normalize( cross(v0,v1)) ) * macos( dot(v0,v1) );
    float k12 = dot( nor, normalize( cross(v1,v2)) ) * macos( dot(v1,v2) );
    float k23 = dot( nor, normalize( cross(v2,v3)) ) * macos( dot(v2,v3) );
    float k34 = dot( nor, normalize( cross(v3,v4)) ) * macos( dot(v3,v4) );
    float k45 = dot( nor, normalize( cross(v4,v5)) ) * macos( dot(v4,v5) );
    float k50 = dot( nor, normalize( cross(v5,v0)) ) * macos( dot(v5,v0) );

    return abs(k01+k12+k23+k34+k45+k50)/6.283185;
}

// -------------------------------------------------------------------------
// get the walls and top face vertex positions
// -------------------------------------------------------------------------

bool getPrismWall( ivec2 prismID, int sid, in float time,
        out vec3 v0, out vec3 v1, out vec3 v2, out vec3 v3 )
{
    const ivec2 i1 = ivec2( 2,0);
    const ivec2 i2 = ivec2( 1,1);
    const ivec2 i3 = ivec2(-1,1);

    vec2  ce = hexagonCenFromID( prismID );
    vec3  ce3 = vec3(ce.x,0.0,ce.y);
    float he = map( ce, time);

    const float kRa = 2.0/sqrt(3.0);
    const float kC1 = kRa*0.5;
    const float kC2 = kRa*1.0;

    if( sid==0 )
    {
        float he1p = map(hexagonCenFromID( prismID+i1 ), time);
        if( he1p<he ) return false;
        v0 = vec3(1.0,he,   kC1);
        v1 = vec3(1.0,he1p, kC1);
        v2 = vec3(1.0,he1p,-kC1);
        v3 = vec3(1.0,he,  -kC1);
    }
    else if( sid==1 )
    {
        float he3m = map(hexagonCenFromID( prismID-i3 ), time);
        if( he3m<he ) return false;
        v0 = vec3( 1.0,he,  -kC1);
        v1 = vec3( 1.0,he3m,-kC1);
        v2 = vec3( 0.0,he3m,-kC2);
        v3 = vec3( 0.0,he,  -kC2);
    }
    else if( sid==2 )
    {
        float he2m = map(hexagonCenFromID( prismID-i2 ), time);
        if( he2m<he ) return false;
        v0 = vec3( 0.0,he,  -kC2);
        v1 = vec3( 0.0,he2m,-kC2);
        v2 = vec3(-1.0,he2m,-kC1);
        v3 = vec3(-1.0,he,  -kC1);
    }
    else if( sid==3 )
    {
        float he1m = map(hexagonCenFromID( prismID-i1 ), time);
        if( he1m<he ) return false;
        v0 = vec3(-1.0,he,  -kC1);
        v1 = vec3(-1.0,he1m,-kC1);
        v2 = vec3(-1.0,he1m, kC1);
        v3 = vec3(-1.0,he,   kC1);
    }
    else if( sid==4 )
    {
        float he3p = map(hexagonCenFromID( prismID+i3 ), time);
        if( he3p<he ) return false;
        v0 = vec3(-1.0,he,   kC1);
        v1 = vec3(-1.0,he3p, kC1);
        v2 = vec3( 0.0,he3p, kC2);
        v3 = vec3( 0.0,he,   kC2);
    }
    else //if( sid==5 )
    {
        float he2p = map(hexagonCenFromID( prismID+i2 ), time);
        if( he2p<he ) return false;
        v0 = vec3( 0.0,he,   kC2);
        v1 = vec3( 0.0,he2p, kC2);
        v2 = vec3( 1.0,he2p, kC1);
        v3 = vec3( 1.0,he,   kC1);
    }      

    v0 += ce3;
    v1 += ce3;
    v2 += ce3;
    v3 += ce3;

    return true;
}

void getPrismTop( ivec2 prismID, in float time,
        out vec3 v0, out vec3 v1, out vec3 v2, 
        out vec3 v3, out vec3 v4, out vec3 v5 )
{
    vec2  ce = hexagonCenFromID( prismID );
    vec3  ce3 = vec3(ce.x,0.0,ce.y);
    float he = map( ce, time);

    const float kRa = 2.0/sqrt(3.0);
    const float kC1 = kRa*0.5;
    const float kC2 = kRa*1.0;

    v0 = ce3+vec3(  0.0,he, -kC2);
    v1 = ce3+vec3( -1.0,he, -kC1);
    v2 = ce3+vec3( -1.0,he,  kC1);
    v3 = ce3+vec3(  0.0,he,  kC2);
    v4 = ce3+vec3(  1.0,he,  kC1);
    v5 = ce3+vec3(  1.0,he, -kC1);
}

// -------------------------------------------------------------------------
// compute analytical ambient occlusion, by using the solid angle of the
// faces surrounding the current point. if one face is missing (it's below
// the current prism's height) we ignore the portal and assume light comes
// through it. Ideally portals should be recursivelly traversed and clipped
// -------------------------------------------------------------------------

float calcOcclusion( in vec3 pos, in vec3 nor, in float time,
        in ivec2 prismID, in int faceID )
{
    const ivec2 i1 = ivec2( 2,0);
    const ivec2 i2 = ivec2( 1,1);
    const ivec2 i3 = ivec2(-1,1);

    vec3 v0, v1, v2, v3, v4, v5;

    if( faceID==-1 ) prismID += i1;
    else if( faceID== 1 ) prismID -= i1;
    else if( faceID==-2 ) prismID += i2;
    else if( faceID== 2 ) prismID -= i2;
    else if( faceID==-3 ) prismID += i3;
    else if( faceID== 3 ) prismID -= i3;

    float occ = 0.0;
    if( faceID!=1 && getPrismWall( prismID, 0, time, v0, v1, v2, v3 ) )
        occ += occlusionQuad(pos,nor,v0,v1,v2,v3);
    if( faceID!=-3 && getPrismWall( prismID, 1, time, v0, v1, v2, v3 ) )
        occ += occlusionQuad(pos,nor,v0,v1,v2,v3);
    if( faceID!=-2 && getPrismWall( prismID, 2, time, v0, v1, v2, v3 ) )
        occ += occlusionQuad(pos,nor,v0,v1,v2,v3);
    if( faceID!=-1 && getPrismWall( prismID, 3, time, v0, v1, v2, v3 ) )
        occ += occlusionQuad(pos,nor,v0,v1,v2,v3);
    if( faceID!=3 && getPrismWall( prismID, 4, time, v0, v1, v2, v3 ) )
        occ += occlusionQuad(pos,nor,v0,v1,v2,v3);
    if( faceID!=2 && getPrismWall( prismID, 5, time, v0, v1, v2, v3 ) )
        occ += occlusionQuad(pos,nor,v0,v1,v2,v3);

    if( faceID!=4 )
    {
        getPrismTop( prismID, time, v0, v1, v2, v3, v4, v5 );
        occ += occlusionHexagon(pos,nor,v0,v1,v2,v3,v4,v5);

        occ = 1.0-min(0.5,0.2+0.8*(1.0-occ)*pos.y/kMaxH);
    }

    return 1.0-occ;
}

// -------------------------------------------------------------------------
// render = raycast + shade + light
// -------------------------------------------------------------------------

vec3 render( in vec3 ro, in vec3 rd, in float time )
{
    // raycast
    vec3  col = vec3(1.0);
    ivec2 prismID; int faceID;
    vec4  tnor = castRay( ro, rd, time, prismID, faceID );
    float t = tnor.x;
    // if intersection found
    if( t>0.0 )
    {
        // data at intersection point
        vec3  pos = ro + rd*t;
        vec3  nor = -tnor.yzw;
        vec2  ce = hexagonCenFromID(prismID);
        float he = map(ce,time);
        int   id = prismID.x*131 + prismID.y*57;

        // uvs
        vec2 uv = (faceID==4) ? (pos.xz-ce)*0.15 : 
            vec2(atan(pos.x-ce.x,pos.z-ce.y)/3.14156, 
                    (pos.y-he)/4.0 );
        uv += ce;

        // material color			
        vec3 mate = vec3(1.0);
        id = hash(id); mate *= 0.1+0.9*float((id>>13)&3)/3.0;
        id = hash(id); mate  = ( ((id>>8)&15)==0 ) ? vec3(0.7,0.0,0.0) : mate;
        vec3 tex = vec3(0.15,0.09,0.07)+0.75*pow(texture(iChannel0,uv.yx).xyz,vec3(1.0,0.95,0.9));
        mate *= tex;

        // lighting
        float occ = calcOcclusion( pos, nor, time, prismID, faceID );

        // diffuse
        col = mate*pow(vec3(occ),vec3(0.95,1.05,1.1));

        // specular
        float ks = tex.x*2.0;
        vec3 ref = reflect(rd,nor);
        col *= 0.85;
        float fre = clamp(1.0+dot(nor,rd),0.0,1.0);
        col += vec3(1.1)*ks*
            smoothstep(0.0,0.15,ref.y)*
            (0.04 + 0.96*pow(fre,5.0))*
            castShadowRay( pos+nor*0.001, ref, time );

        // fog
        col = mix(col,vec3(1.0), 1.0-exp2(-0.00005*t*t) );
    }

    return col;
}

//-----------------------------------------------
// main = animate + render + color grade
//-----------------------------------------------
void mainImage( out vec4 fragColor, in vec2 fragCoord )
{
    // init random seed
    ivec2 q = ivec2(fragCoord);

    // sample pixel	and time
    vec3 tot = vec3(0.0);
    for( int m=0; m<AA; m++ )
        for( int n=0; n<AA; n++ )
        {
            vec2  of = vec2(m,n)/float(AA) - 0.5;
            vec2  p = (2.0*(fragCoord+of)-iResolution.xy)/min(iResolution.x,iResolution.y);
#if AA>1
            float d = 0.5+0.5*sin(fragCoord.x*147.0)*sin(fragCoord.y*131.0);
            float time = iTime - 0.5*(1.0/24.0)*(float(m*AA+n)+d)/float(AA*AA);
#else
            float time = iTime;
#endif

            // camera
            float cr = -0.1;
            float an = 3.0*time + 20.0*iMouse.x/iResolution.x;
            vec3 ro = vec3(0.1,13.0,1.0-an);
            vec3 ta = vec3(0.0,12.0,0.0-an);

            // build camera matrix
            vec3 ww = normalize( ta - ro);
            vec3 uu = normalize(cross( ww,vec3(sin(cr),cos(cr),0.0) ));
            vec3 vv = normalize(cross(uu,ww));
            // distort
            p *= 0.9+0.1*(p.x*p.x*0.4 + p.y*p.y);
            // buid ray
            vec3 rd = normalize( p.x*uu + p.y*vv + 2.0*ww );

            // dof
#if AA>1
            vec3 fp = ro + rd*17.0;
            vec2 ra = texelFetch(iChannel1,(q+ivec2(13*m,31*n))&1023,0).xy;
            ro.xy += 0.3*sqrt(ra.x)*vec2(cos(6.2831*ra.y),sin(6.2831*ra.y));
            rd = normalize( fp - ro );
#endif

            // render
            vec3 col = render( ro, rd, time );

            // accumulate for AA
            tot += col;
        }
    tot /= float(AA*AA);


    // hdr->ldr tonemap
    tot = tot*1.6/(1.0+tot);
    tot = tot*tot*(3.0-2.0*tot);

    // gamma
    tot = pow( clamp(tot,0.0,1.0), vec3(0.45) );

    // color grade
    tot = mix( tot, vec3(dot(tot,vec3(0.3333))), -0.2 );
    tot = pow(tot,vec3(0.95,1.0,1.0));

    vec2 p = fragCoord/iResolution.xy;
    tot.xyz += (p.xyy-0.5)*0.1;

    // vignetting	
    tot *= 0.5 + 0.5*pow( 16.0*p.x*p.y*(1.0-p.x)*(1.0-p.y), 0.1 );

    // output
    fragColor = vec4( tot, 1.0 );
}
