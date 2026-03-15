// seascape-style ocean rendering adapted for the tracer composition pass
#ifndef OCEAN_GLSL
#define OCEAN_GLSL

// configuration adapted from "Seascape" (Alexander Alekseev / TDM)
const int   OCEAN_NUM_STEPS       = 4;
const float OCEAN_EPSILON         = 1e-4;
const int   OCEAN_ITER_GEOMETRY   = 3;
const int   OCEAN_ITER_FRAGMENT   = 5;
const float OCEAN_HEIGHT          = 0.3;
const float OCEAN_CHOPPY          = 1.0;
const float OCEAN_SPEED           = 0.4;
const float OCEAN_FREQ            = 0.16;
const mat2  OCEAN_OCTAVE_M        = mat2(1.6, 1.2, -1.2, 1.6);

float ocean_time() {
    // keep the existing ocean time multiplier but map to seascape's notion of time
    float t = float(env_info.frame_serial_idx) * 0.016 * gui_input.ocean_time_multiplier;
    return 1.0 + t * OCEAN_SPEED;
}

// use GUI colors instead of fixed palette
vec3 ocean_base_color() {
    return srgb_to_linear(gui_input.ocean_deep_color);
}

vec3 ocean_water_color() {
    return srgb_to_linear(gui_input.ocean_shallow_color);
}

// basic 2D hash/noise, locally scoped to avoid clashes
float ocean_hash(vec2 p) {
    float h = dot(p, vec2(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

float ocean_noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);

    float n00 = ocean_hash(i + vec2(0.0, 0.0));
    float n10 = ocean_hash(i + vec2(1.0, 0.0));
    float n01 = ocean_hash(i + vec2(0.0, 1.0));
    float n11 = ocean_hash(i + vec2(1.0, 1.0));

    float nx0 = mix(n00, n10, u.x);
    float nx1 = mix(n01, n11, u.x);

    return -1.0 + 2.0 * mix(nx0, nx1, u.y);
}

// lighting helpers
float ocean_diffuse(vec3 n, vec3 l, float p) {
    return pow(dot(n, l) * 0.4 + 0.6, p);
}

float ocean_specular(vec3 n, vec3 l, vec3 e, float s) {
    float nrm = (s + 8.0) / (PI * 8.0);
    return pow(max(dot(reflect(e, n), l), 0.0), s) * nrm;
}

float ocean_sea_octave(vec2 uv, float choppy) {
    uv += ocean_noise(uv);
    vec2 wv  = 1.0 - abs(sin(uv));
    vec2 swv = abs(cos(uv));
    wv       = mix(wv, swv, wv);
    return pow(1.0 - pow(wv.x * wv.y, 0.65), choppy);
}

float ocean_map(vec3 p) {
    float freq   = OCEAN_FREQ;
    float amp    = OCEAN_HEIGHT;
    float choppy = OCEAN_CHOPPY;
    vec2 uv      = p.xz;
    uv.x *= 0.75;

    float d;
    float h   = 0.0;
    float t   = ocean_time();
    for (int i = 0; i < OCEAN_ITER_GEOMETRY; i++) {
        d = ocean_sea_octave((uv + t) * freq, choppy);
        d += ocean_sea_octave((uv - t) * freq, choppy);
        h += d * amp;
        uv *= OCEAN_OCTAVE_M;
        freq *= 1.9;
        amp *= 0.22;
        choppy = mix(choppy, 1.0, 0.2);
    }
    return p.y - h;
}

float ocean_map_detailed(vec3 p) {
    float freq   = OCEAN_FREQ;
    float amp    = OCEAN_HEIGHT;
    float choppy = OCEAN_CHOPPY;
    vec2 uv      = p.xz;
    uv.x *= 0.75;

    float d;
    float h   = 0.0;
    float t   = ocean_time();
    for (int i = 0; i < OCEAN_ITER_FRAGMENT; i++) {
        d = ocean_sea_octave((uv + t) * freq, choppy);
        d += ocean_sea_octave((uv - t) * freq, choppy);
        h += d * amp;
        uv *= OCEAN_OCTAVE_M;
        freq *= 1.9;
        amp *= 0.22;
        choppy = mix(choppy, 1.0, 0.2);
    }
    return p.y - h;
}

float ocean_normal_epsilon_base() {
    // mirror the original seascape EPSILON_NRM definition using current render width
    return 0.1 / float(imageSize(composited_tex).x);
}

vec3 ocean_get_normal(vec3 p, float eps) {
    vec3 n;
    n.y = ocean_map_detailed(p);
    n.x = ocean_map_detailed(vec3(p.x + eps, p.y, p.z)) - n.y;
    n.z = ocean_map_detailed(vec3(p.x, p.y, p.z + eps)) - n.y;
    n.y = eps;
    return normalize(n);
}

float ocean_height_map_tracing(vec3 ori, vec3 dir, out vec3 p) {
    float tm = 0.0;
    float tx = 1000.0;
    float hx = ocean_map(ori + dir * tx);
    if (hx > 0.0) {
        p = ori + dir * tx;
        return tx;
    }
    float hm = ocean_map(ori);
    for (int i = 0; i < OCEAN_NUM_STEPS; i++) {
        float tmid = mix(tm, tx, hm / (hm - hx));
        p          = ori + dir * tmid;
        float hmid = ocean_map(p);
        if (hmid < 0.0) {
            tx = tmid;
            hx = hmid;
        } else {
            tm = tmid;
            hm = hmid;
        }
        if (abs(hmid) < OCEAN_EPSILON) {
            break;
        }
    }
    return mix(tm, tx, hm / (hm - hx));
}

vec3 ocean_get_sea_color(vec3 p, vec3 n, vec3 l, vec3 eye, vec3 dist) {
    float fresnel = clamp(1.0 - dot(n, -eye), 0.0, 1.0);
    fresnel       = min(fresnel * fresnel * fresnel, 0.5);

    vec3 reflected = compute_sky_with_sun_and_stars(reflect(eye, n));
    vec3 base      = ocean_base_color();
    vec3 water     = ocean_water_color();

    vec3 refracted = base + ocean_diffuse(n, l, 80.0) * water * 0.12;
    vec3 color     = mix(refracted, reflected, fresnel);

    float dist2 = dot(dist, dist);
    float atten = max(1.0 - dist2 * 0.001, 0.0);
    color += water * (p.y - OCEAN_HEIGHT) * 0.18 * atten;

    color += ocean_specular(n, l, eye, 600.0 * inversesqrt(max(dist2, 1e-4)));

    return color;
}

vec3 compute_ocean_color(vec3 view_dir, out bool hit_ocean) {
    vec3 ori = camera_info.pos.xyz;
    vec3 dir = normalize(view_dir);

    // assume ocean below the camera; avoid useless tracing when we look upwards
    if (dir.y >= 0.0) {
        hit_ocean = false;
        return vec3(0.0);
    }

    vec3 p;
    float t = ocean_height_map_tracing(ori, dir, p);
    if (t <= 0.0) {
        hit_ocean = false;
        return vec3(0.0);
    }

    vec3 dist      = p - ori;
    float eps_base = ocean_normal_epsilon_base();
    vec3 n         = ocean_get_normal(p, dot(dist, dist) * eps_base);
    vec3 light_dir = normalize(sun_info.sun_dir);

    vec3 color = ocean_get_sea_color(p, n, light_dir, dir, dist);

    hit_ocean = true;
    return color;
}

vec3 compute_environment_color(vec3 view_dir) {
    bool hit_ocean;
    vec3 ocean_color = compute_ocean_color(view_dir, hit_ocean);
    if (hit_ocean) {
        return ocean_color;
    }
    return compute_sky_with_sun_and_stars(view_dir);
}

#endif // OCEAN_GLSL
