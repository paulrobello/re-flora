#ifndef WIND_HASH_GLSL
#define WIND_HASH_GLSL

// Gradient-noise wind simulation — Metal/MoltenVK-safe replacement for wind.glsl.
// Preserves the original spatial variation, traveling gusts, and direction/strength
// behavior using Perlin gradient noise (no large permutation tables).

#include "./core/definitions.glsl"
#include "./core/gradient_noise.glsl"

const float WIND_DIRECTION_FREQUENCY       = 0.0025f;
const float WIND_STRENGTH_FREQUENCY        = 0.00125f;
const float WIND_MIN_STRENGTH              = 0.5f;
const float WIND_MAX_STRENGTH              = 5.0f;
const float WIND_SAMPLE_SCALE              = 256.0f;
const vec2 WIND_SECOND_SAMPLE_OFFSET       = vec2(57.23f, -113.87f);
const vec2 WIND_STRENGTH_OFFSET            = vec2(-211.0f, 83.0f);
const vec2 WIND_DIRECTION_TIME_SCROLL      = vec2(0.2f, -0.3f);
const vec2 WIND_STRENGTH_TIME_SCROLL       = vec2(-0.3f, 0.5f);
const float WIND_TIME_SCALE                = 200.0f;
const float WIND_DIRECTION_DETAIL_STRENGTH = 0.35f;

vec3 get_wind(vec3 world_pos, float time) {
    vec2 sample_pos     = world_pos.xz * WIND_SAMPLE_SCALE;
    float scroll_time   = time * WIND_TIME_SCALE;
    vec2 direction_time = WIND_DIRECTION_TIME_SCROLL * scroll_time;
    vec2 strength_time  = WIND_STRENGTH_TIME_SCROLL * scroll_time;

    // Primary direction noise (replaces FNL OpenSimplex2 FBM, seed 1729)
    float primary_direction_noise =
        fbm_cnoise_2d(sample_pos.x + direction_time.x, sample_pos.y + direction_time.y, 1729u,
                      WIND_DIRECTION_FREQUENCY, 4, 2.0, 0.5);

    // Detail direction noise (second sample offset)
    float detail_direction_noise =
        fbm_cnoise_2d(sample_pos.x + WIND_SECOND_SAMPLE_OFFSET.x + direction_time.x,
                      sample_pos.y + WIND_SECOND_SAMPLE_OFFSET.y + direction_time.y, 1729u,
                      WIND_DIRECTION_FREQUENCY, 4, 2.0, 0.5);

    float base_angle   = (primary_direction_noise * 0.5f + 0.5f) * TWO_PI;
    float detail_angle = detail_direction_noise * WIND_DIRECTION_DETAIL_STRENGTH;
    vec2 direction     = vec2(cos(base_angle + detail_angle), sin(base_angle + detail_angle));

    // Strength noise (replaces FNL OpenSimplex2 FBM, seed 2843)
    float strength_noise = fbm_cnoise_2d(sample_pos.x + WIND_STRENGTH_OFFSET.x + strength_time.x,
                                         sample_pos.y + WIND_STRENGTH_OFFSET.y + strength_time.y,
                                         2843u, WIND_STRENGTH_FREQUENCY, 4, 2.0, 0.5);

    float strength = mix(WIND_MIN_STRENGTH, WIND_MAX_STRENGTH, strength_noise * 0.5f + 0.5f);

    vec2 wind_planar = direction * strength;
    return vec3(wind_planar.x, 0.0f, wind_planar.y);
}

#endif // WIND_HASH_GLSL
