#define _Layers 3.0
#define _Speed 0.03
#define _ColorA vec3(1.1, 0.2, 0.0) 
#define _ColorB vec3(1.0, 1.2, 0.5) 
#define _ColorC vec3(0.0, 0.8, 1.2) 
#define _Glow 1.5

// RU: Тройная линейная интерполяция для плавных переходов цвета
// EN: Three-way linear interpolation for smooth color transitions
// JP: スムーズな色の変化のための3方向線形補間
vec3 lerp3(vec3 a, vec3 b, vec3 c, float t) {
    if (t < 0.5) {
        return mix(a, b, t * 2.0);
    } else {
        return mix(b, c, (t - 0.5) * 2.0);
    }
}

// RU: Функция отрисовки звезды с лучами
// EN: Function to draw a star with procedural rays
// JP: 光線を持つ星を描画する関数
float Star(vec2 uv, float size) {
    float d = length(uv);
    float rays = max(0.0, 1.0 - abs(uv.x * uv.y * 1000.0));
    return (0.005 * size / d + rays * 0.05 * size) * smoothstep(0.15, 0.0, d);
}

// RU: Матрицы вращения вокруг осей X и Y
// EN: Rotation matrices for X and Y axes
// JP: X軸とY軸の回転行列
mat3 rotX(float a) {
    float s = sin(a), c = cos(a);
    return mat3(1, 0, 0, 0, c, -s, 0, s, c);
}

mat3 rotY(float a) {
    float s = sin(a), c = cos(a);
    return mat3(c, 0, s, 0, 1, 0, -s, 0, c);
}

// RU: Простая хеш-функция для генерации псевдослучайных чисел
// EN: Simple hash function for pseudo-random numbers
// JP: 疑似乱数生成のための単純なハッシュ関数
float hash(float n) { return fract(sin(n) * 43758.5453); }

// RU: Параметры фрактала "Magic Formula"
// EN: Fractal "Magic Formula" parameters
// JP: フラクタル「魔法の公式」のパラメータ
#define iterations 17
#define formuparam 0.53
#define volsteps 20
#define stepsize 0.1
#define zoom   0.800
#define tile   0.850
#define speed  0.010 
#define brightness 0.0015
#define darkmatter 0.300
#define distfading 0.730
#define saturation 0.850

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    // RU: Подготовка координат UV
    // EN: UV coordinates preparation
    // JP: UV座標の準備
    vec2 uv = (fragCoord - 0.5 * iResolution.xy) / iResolution.y;
    uv *= 0.5;
    vec2 uv3 = fragCoord.xy / iResolution.xy - .5;
    uv3.y *= iResolution.y / iResolution.x;

    vec3 dir = vec3(uv3 * zoom, 1.);
    float time = iTime * speed + .25;

    // RU: Секция объемного рендеринга (звездная пыль/туманность)
    // EN: Volumetric rendering section (stardust/nebula)
    // JP: ボリュームレンダリングセクション（星屑・星雲）
    vec3 from = vec3(1., .5, 0.5);
    float s = 0.1, fade = 1.;
    vec3 v3 = vec3(0.);
    for (int r = 0; r < volsteps; r++) {
        vec3 p = from + s * dir * .5;
        p = abs(vec3(tile) - mod(p, vec3(tile * 2.))); 
        float pa, a = pa = 0.;
        for (int i = 0; i < iterations; i++) { 
            p = abs(p) / dot(p, p) - formuparam;
            p.xy *= mat2(cos(iTime * 0.01), sin(iTime * 0.01), -sin(iTime * 0.01), cos(iTime * 0.01));
            a += abs(length(p) - pa);
            pa = length(p);
        }
        float dm = max(0., darkmatter - a * a * .001); 
        a *= a * a; 
        if (r > 6) fade *= 1.2 - dm;
        v3 += fade;
        v3 += vec3(s, s * s, s * s * s * s) * a * brightness * fade; 
        fade *= distfading; 
        s += stepsize;
    }
    v3 = mix(vec3(length(v3)), v3, saturation);
    vec4 fc = vec4(v3 * .03, 1.);	

    vec3 col = vec3(0.0);

    // RU: Создание сетки для "иероглифов" или созвездий
    // EN: Creating a grid for "hieroglyphs" or constellations
    // JP: 「文字」や「星座」のためのグリッド作成
    vec2 gv = uv * 12.0;
    gv.y += iTime * _Speed * 10.0; 

    vec2 id = floor(gv); // RU: ID ячейки | EN: Cell ID | JP: セルID
    vec2 fv = fract(gv) - 0.5; // RU: Локальные координаты | EN: Local coordinates | JP: ローカル座標

    // RU: Вершины куба для построения фигур
    // EN: Cube vertices for shape construction
    // JP: 形状構築のための立方体の頂点
    vec3 v[8]; 
    v[0]=vec3(-1,-1,-1); v[1]=vec3(1,-1,-1); v[2]=vec3(1,1,-1); v[3]=vec3(-1,1,-1);
    v[4]=vec3(-1,-1,1); v[5]=vec3(1,-1,1); v[6]=vec3(1,1,1); v[7]=vec3(-1,1,1);

    // RU: Индексы ребер
    // EN: Edge indices
    // JP: エッジのインデックス
    int ed[24] = int[](0,1, 1,2, 2,3, 3,0, 4,5, 5,6, 6,7, 7,4, 0,4, 1,5, 2,6, 3,7);

    float rnd = hash(id.x * 12.5 + id.y * 33.4);
    float twinkle = pow(sin(iTime * 3.0 + rnd * 6.28) * 0.5 + 0.5, 4.0);

    // RU: Вращение фигур внутри ячеек
    // EN: Rotation of shapes inside cells
    // JP: セル内の形状の回転
    mat3 transform = rotX(rnd * 6.3 + iTime * 0.2) * rotY(iTime * 0.3);
    float scale = 0.25;

    // RU: Отрисовка случайных соединений (линий)
    // EN: Drawing random connections (lines)
    // JP: ランダムな接続（線）の描画
    for (int n = 0; n < 12; n++) {
        if (hash(rnd + float(n) * 0.5) > 0.5) continue;

        vec3 p1 = transform * (v[ed[n]] * scale);
        vec3 p2 = transform * (v[n % 8] * scale); 

        // RU: Проекция 3D точек в 2D пространство ячейки
        // EN: Projection of 3D points into 2D cell space
        // JP: 3Dポイントの2Dセル空間への投影
        float z1 = 1.0 / (2.0 - p1.z);
        vec2 a = p1.xy * z1, b = p2.xy * (1.0 / (2.0 - p2.z));

        // RU: Вычисление расстояния до линии (сегмента)
        // EN: Line segment distance calculation
        // JP: 線分への距離計算
        vec2 pa = fv - a, ba = b - a;
        float h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
        float d = length(pa - ba * h);

        vec3 layerCol = lerp3(_ColorA, _ColorB, _ColorC, fract(rnd + iTime * 0.1));
        col += layerCol * smoothstep(0.01, 0.0, d) * (0.3 + twinkle * 0.7);

        // RU: Добавление ярких звезд в узлы
        // EN: Adding bright stars to nodes
        // JP: ノードに輝く星を追加
        if(n < 4 && twinkle > 0.5) {
            col += layerCol * Star(fv - a, 0.2) * twinkle * fc.xyz * 40.;
        }
    }

    // RU: Финальное свечение и пост-обработка
    // EN: Final glow and post-processing
    // JP: 最終的な発光とポストプロセス
    col += _ColorC * 0.1 / (length(fv) + 0.5);
    fragColor = vec4(tanh(col * 2.), 1.0);
}
