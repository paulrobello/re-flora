#ifndef OCEAN_DEPTH_GLSL
#define OCEAN_DEPTH_GLSL

#include "./ray.glsl"

const float OCEAN_DEPTH_DRAG_MULT   = 0.1;
const float OCEAN_DEPTH_WATER_DEPTH = 0.5;
const int OCEAN_DEPTH_ITER_RAYMARCH = 12;
const float OCEAN_DEPTH_SPEED       = 0.4;

float ocean_depth_time() {
    float t = float(env_info.frame_serial_idx) * 0.016 * gui_input.ocean_time_multiplier;
    return 1.0 + t * OCEAN_DEPTH_SPEED;
}

float ocean_depth_sea_level() { return gui_input.ocean_sea_level_shift; }

vec2 ocean_depth_wave_dx(vec2 position, vec2 direction, float frequency, float timeshift) {
    float x    = dot(direction, position) * frequency + timeshift;
    float wave = exp(sin(x) - 1.0);
    float dx   = wave * cos(x);
    return vec2(wave, -dx);
}

float ocean_depth_get_waves(vec2 position, int iterations) {
    float wave_phase_shift = length(position) * 0.1;
    float iter             = 0.0;
    float frequency        = 1.0;
    float time_multiplier  = 2.0;
    float weight           = 1.0;

    float sum_of_values  = 0.0;
    float sum_of_weights = 0.0;

    float t = ocean_depth_time();

    for (int i = 0; i < iterations; i++) {
        vec2 dir = vec2(sin(iter), cos(iter));

        vec2 res =
            ocean_depth_wave_dx(position, dir, frequency, t * time_multiplier + wave_phase_shift);

        position += dir * res.y * weight * OCEAN_DEPTH_DRAG_MULT;
        sum_of_values += res.x * weight;
        sum_of_weights += weight;

        weight = mix(weight, 0.0, 0.2);
        frequency *= 1.18;
        time_multiplier *= 1.07;
        iter += 1232.399963;
    }

    return sum_of_values / max(sum_of_weights, 1e-3);
}

float ocean_depth_intersect_plane(vec3 origin, vec3 dir, vec3 point, vec3 normal) {
    float denom = dot(dir, normal);
    if (abs(denom) < 1e-6) {
        return -1.0;
    }
    return dot(point - origin, normal) / denom;
}

float ocean_depth_raymarch_water(vec3 camera, vec3 start, vec3 end, float depth) {
    vec3 pos        = start;
    vec3 dir        = normalize(end - start);
    float sea_level = ocean_depth_sea_level();

    for (int i = 0; i < 64; i++) {
        float height =
            ocean_depth_get_waves(pos.xz, OCEAN_DEPTH_ITER_RAYMARCH) * depth - depth + sea_level;

        if (height + 0.01 > pos.y) {
            return distance(pos, camera);
        }

        pos += dir * (pos.y - height);
    }

    return distance(start, camera);
}

float ocean_depth_01_from_view_dir(vec3 view_dir, out bool hit_ocean) {
    vec3 ori        = camera_info.pos.xyz;
    vec3 dir        = normalize(view_dir);
    float sea_level = ocean_depth_sea_level();

    if (dir.y >= 0.0) {
        hit_ocean = false;
        return 1.0;
    }

    vec3 water_plane_high = vec3(0.0, sea_level, 0.0);
    vec3 water_plane_low  = vec3(0.0, sea_level - OCEAN_DEPTH_WATER_DEPTH, 0.0);
    vec3 up               = vec3(0.0, 1.0, 0.0);

    float high_t = ocean_depth_intersect_plane(ori, dir, water_plane_high, up);
    float low_t  = ocean_depth_intersect_plane(ori, dir, water_plane_low, up);

    if (high_t <= 0.0 || low_t <= 0.0 || low_t <= high_t) {
        hit_ocean = false;
        return 1.0;
    }

    vec3 high_pos = ori + dir * high_t;
    vec3 low_pos  = ori + dir * low_t;

    float dist = ocean_depth_raymarch_water(ori, high_pos, low_pos, OCEAN_DEPTH_WATER_DEPTH);
    vec3 p     = ori + dir * dist;

    vec4 point_ndc = camera_info.view_proj_mat * vec4(p, 1.0);
    hit_ocean      = true;
    return point_ndc.z / point_ndc.w;
}

float ocean_depth_01_from_screen_uv(vec2 screen_uv, out bool hit_ocean) {
    Ray ray = ray_gen(screen_uv, camera_info.view_proj_mat_inv);
    return ocean_depth_01_from_view_dir(ray.direction, hit_ocean);
}

#endif // OCEAN_DEPTH_GLSL
