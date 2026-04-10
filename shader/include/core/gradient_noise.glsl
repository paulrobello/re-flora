#ifndef GRADIENT_NOISE_GLSL
#define GRADIENT_NOISE_GLSL

// Classic Perlin noise (gradient-based) — no large permutation tables.
// Uses arithmetic hash (mod289 + permute) for gradient selection.
// Adapted from lygia/generative/cnoise.glsl (Stefan Gustavson, Ian McEwan, MIT license).

// ── Math helpers (inlined to avoid extra include files) ──────────────────────

#ifndef FNC_MOD289
#define FNC_MOD289
float mod289(const in float x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
vec2 mod289(const in vec2 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
vec3 mod289(const in vec3 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
vec4 mod289(const in vec4 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
#endif

#ifndef FNC_PERMUTE
#define FNC_PERMUTE
float permute(const in float v) { return mod289(((v * 34.0) + 1.0) * v); }
vec2 permute(const in vec2 v) { return mod289(((v * 34.0) + 1.0) * v); }
vec3 permute(const in vec3 v) { return mod289(((v * 34.0) + 1.0) * v); }
vec4 permute(const in vec4 v) { return mod289(((v * 34.0) + 1.0) * v); }
#endif

#ifndef FNC_TAYLORINVSQRT
#define FNC_TAYLORINVSQRT
float taylorInvSqrt(in float r) { return 1.79284291400159 - 0.85373472095314 * r; }
vec4 taylorInvSqrt(in vec4 r) { return 1.79284291400159 - 0.85373472095314 * r; }
#endif

#ifndef FNC_QUINTIC
#define FNC_QUINTIC
float quintic(const in float v) { return v * v * v * (v * (v * 6.0 - 15.0) + 10.0); }
vec2 quintic(const in vec2 v) { return v * v * v * (v * (v * 6.0 - 15.0) + 10.0); }
#endif

// ── 2D Classic Perlin Noise ─────────────────────────────────────────────────
// Output range: approximately [-1, 1]

float cnoise(in vec2 P) {
    vec4 Pi = floor(P.xyxy) + vec4(0.0, 0.0, 1.0, 1.0);
    vec4 Pf = fract(P.xyxy) - vec4(0.0, 0.0, 1.0, 1.0);
    Pi      = mod289(Pi);
    vec4 ix = Pi.xzxz;
    vec4 iy = Pi.yyww;
    vec4 fx = Pf.xzxz;
    vec4 fy = Pf.yyww;

    vec4 i = permute(permute(ix) + iy);

    vec4 gx = fract(i * (1.0 / 41.0)) * 2.0 - 1.0;
    vec4 gy = abs(gx) - 0.5;
    vec4 tx = floor(gx + 0.5);
    gx      = gx - tx;

    vec2 g00 = vec2(gx.x, gy.x);
    vec2 g10 = vec2(gx.y, gy.y);
    vec2 g01 = vec2(gx.z, gy.z);
    vec2 g11 = vec2(gx.w, gy.w);

    vec4 norm = taylorInvSqrt(vec4(dot(g00, g00), dot(g01, g01), dot(g10, g10), dot(g11, g11)));
    g00 *= norm.x;
    g01 *= norm.y;
    g10 *= norm.z;
    g11 *= norm.w;

    float n00 = dot(g00, vec2(fx.x, fy.x));
    float n10 = dot(g10, vec2(fx.y, fy.y));
    float n01 = dot(g01, vec2(fx.z, fy.z));
    float n11 = dot(g11, vec2(fx.w, fy.w));

    vec2 fade_xy = quintic(Pf.xy);
    vec2 n_x     = mix(vec2(n00, n01), vec2(n10, n11), fade_xy.x);
    float n_xy   = mix(n_x.x, n_x.y, fade_xy.y);
    return 2.3 * n_xy;
}

// ── Seeded 2D Perlin Noise ──────────────────────────────────────────────────
// Offsets the input by a seed-derived displacement to produce different patterns
// per seed, without needing a permutation table.

float cnoise_seeded(vec2 P, uint seed) {
    // Derive a large pseudo-random offset from the seed so each seed
    // produces an independent-looking region of the noise field.
    float sx = float(seed * 73856093u & 0xFFFFu) * 0.153;
    float sy = float(seed * 19349663u & 0xFFFFu) * 0.217;
    return cnoise(P + vec2(sx, sy));
}

// ── FBM over seeded 2D Perlin Noise ─────────────────────────────────────────
// Output range: approximately [-1, 1] (amplitude-normalized).

float fbm_cnoise_2d(float x, float y, uint seed, float frequency, int octaves, float lacunarity,
                    float gain) {
    float sum     = 0.0;
    float amp     = 1.0;
    float amp_sum = 0.0;
    float freq    = frequency;
    for (int i = 0; i < octaves; i++) {
        sum += cnoise_seeded(vec2(x, y) * freq, seed + uint(i) * 1000u) * amp;
        amp_sum += amp;
        freq *= lacunarity;
        amp *= gain;
    }
    return sum / amp_sum;
}

#endif // GRADIENT_NOISE_GLSL
