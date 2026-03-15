// ocean surface based on "Seascape (fast version)"
// https://www.shadertoy.com/view/MdXyzX
// by Alexander Alekseev (TDM) / afl_ext, MIT License
// adapted for the tracer composition pass
#ifndef OCEAN_GLSL
#define OCEAN_GLSL

// configuration derived from "Seascape (fast version)"
const float OCEAN_DRAG_MULT     = 0.1;
const float OCEAN_WATER_DEPTH   = 0.5;
const int   OCEAN_ITER_RAYMARCH = 12;
const int   OCEAN_ITER_NORMAL   = 36;
const float OCEAN_SPEED         = 0.4;

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

// lighting helpers
float ocean_diffuse(vec3 n, vec3 l, float p) {
    return pow(dot(n, l) * 0.4 + 0.6, p);
}

float ocean_specular(vec3 n, vec3 l, vec3 e, float s) {
    float nrm = (s + 8.0) / (PI * 8.0);
    return pow(max(dot(reflect(e, n), l), 0.0), s) * nrm;
}

float ocean_light_factor() {
    // direct sun contribution based on altitude
    float sun_alt    = sun_info.sun_dir.y;
    float sun_factor = smoothstep(-0.1, 0.2, sun_alt);

    // ambient contribution based on ambient light luminance, clamped and down-weighted
    float ambient_luma = dot(shading_info.ambient_light,
                             vec3(0.2126, 0.7152, 0.0722));
    float ambient_factor = clamp(ambient_luma, 0.0, 1.0);
    float ambient_weight = 0.3;

    return clamp(sun_factor + ambient_factor * ambient_weight, 0.0, 1.0);
}

// seascape-fast style wave field
vec2 ocean_wavedx(vec2 position, vec2 direction, float frequency, float timeshift) {
    float x    = dot(direction, position) * frequency + timeshift;
    float wave = exp(sin(x) - 1.0);
    float dx   = wave * cos(x);
    return vec2(wave, -dx);
}

float ocean_getwaves(vec2 position, int iterations) {
    float wave_phase_shift = length(position) * 0.1;
    float iter             = 0.0;
    float frequency        = 1.0;
    float time_multiplier  = 2.0;
    float weight           = 1.0;

    float sum_of_values  = 0.0;
    float sum_of_weights = 0.0;

    float t = ocean_time();

    for (int i = 0; i < iterations; i++) {
        vec2 dir = vec2(sin(iter), cos(iter));

        vec2 res = ocean_wavedx(position, dir, frequency,
                                t * time_multiplier + wave_phase_shift);

        position      += dir * res.y * weight * OCEAN_DRAG_MULT;
        sum_of_values += res.x * weight;
        sum_of_weights += weight;

        weight         = mix(weight, 0.0, 0.2);
        frequency     *= 1.18;
        time_multiplier *= 1.07;
        iter          += 1232.399963;
    }

    return sum_of_values / max(sum_of_weights, 1e-3);
}

float ocean_normal_epsilon_base() {
    // mirror the original seascape EPSILON_NRM definition using current render width
    return 0.1 / float(imageSize(composited_tex).x);
}

vec3 ocean_surface_normal(vec2 pos, float eps, float depth) {
    vec2 ex = vec2(eps, 0.0);

    float H = ocean_getwaves(pos, OCEAN_ITER_NORMAL) * depth;
    vec3 a  = vec3(pos.x, H, pos.y);

    vec3 b = vec3(pos.x - eps,
                  ocean_getwaves(pos - ex, OCEAN_ITER_NORMAL) * depth,
                  pos.y);
    vec3 c = vec3(pos.x,
                  ocean_getwaves(pos + ex.yx, OCEAN_ITER_NORMAL) * depth,
                  pos.y + eps);

    return normalize(cross(a - b, a - c));
}

float ocean_intersect_plane(vec3 origin, vec3 dir, vec3 point, vec3 normal) {
    float denom = dot(dir, normal);
    if (abs(denom) < 1e-6) {
        return -1.0;
    }
    return dot(point - origin, normal) / denom;
}

float ocean_raymarch_water(vec3 camera, vec3 start, vec3 end, float depth) {
    vec3 pos = start;
    vec3 dir = normalize(end - start);

    for (int i = 0; i < 64; i++) {
        float height = ocean_getwaves(pos.xz, OCEAN_ITER_RAYMARCH) * depth - depth;

        if (height + 0.01 > pos.y) {
            return distance(pos, camera);
        }

        pos += dir * (pos.y - height);
    }

    return distance(start, camera);
}

vec3 ocean_get_sea_color_fast(vec3 p, vec3 n, vec3 light_dir,
                              vec3 view_dir, float dist) {
    float dist2 = dist * dist;

    float light_factor = ocean_light_factor();

    float fresnel = 0.04 + (1.0 - 0.04) *
                    pow(1.0 - max(0.0, dot(-n, view_dir)), 5.0);

    vec3 R = normalize(reflect(view_dir, n));
    R.y    = abs(R.y);

    vec3 reflection = compute_sky_with_sun_and_stars(R);

    vec3 deep = ocean_base_color();

    float depth_norm = clamp((p.y + OCEAN_WATER_DEPTH) / OCEAN_WATER_DEPTH,
                             0.0, 1.0);

    vec3 scattering_base = deep;
    vec3 scattering      = scattering_base * light_factor * 0.1 * (0.2 + depth_norm);

    vec3 color = fresnel * reflection + (1.0 - fresnel) * scattering;

    float atten = max(1.0 - dist2 * 0.001, 0.0);
    color += deep * depth_norm * 0.18 * atten * light_factor;

    color += ocean_specular(n, light_dir, view_dir,
                            600.0 * inversesqrt(max(dist2, 1e-4))) * light_factor;

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

    vec3 waterPlaneHigh = vec3(0.0, 0.0, 0.0);
    vec3 waterPlaneLow  = vec3(0.0, -OCEAN_WATER_DEPTH, 0.0);
    vec3 up             = vec3(0.0, 1.0, 0.0);

    float highT = ocean_intersect_plane(ori, dir, waterPlaneHigh, up);
    float lowT  = ocean_intersect_plane(ori, dir, waterPlaneLow, up);

    if (highT <= 0.0 || lowT <= 0.0 || lowT <= highT) {
        hit_ocean = false;
        return vec3(0.0);
    }

    vec3 highPos = ori + dir * highT;
    vec3 lowPos  = ori + dir * lowT;

    float dist = ocean_raymarch_water(ori, highPos, lowPos, OCEAN_WATER_DEPTH);
    vec3 p     = ori + dir * dist;
    vec3 dvec  = p - ori;

    float eps_base = ocean_normal_epsilon_base();
    float eps      = max(dot(dvec, dvec) * eps_base, 1e-5);

    vec3 n = ocean_surface_normal(p.xz, eps, OCEAN_WATER_DEPTH);

    n = mix(n, vec3(0.0, 1.0, 0.0),
            0.8 * min(1.0, sqrt(dist * 0.01) * 1.1));

    vec3 light_dir = normalize(sun_info.sun_dir);

    vec3 color = ocean_get_sea_color_fast(p, n, light_dir, dir, dist);

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
