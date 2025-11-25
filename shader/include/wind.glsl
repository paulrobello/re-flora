#ifndef WIND_GLSL
#define WIND_GLSL

#include "./core/definitions.glsl"
#include "./core/fast_noise_lite.glsl"

const int WIND_DIRECTION_SEED         = 1729;
const int WIND_STRENGTH_SEED          = 2843;
const float WIND_DIRECTION_FREQUENCY  = 0.0025f;
const float WIND_STRENGTH_FREQUENCY   = 0.00125f;
const float WIND_MIN_STRENGTH         = 0.5f;
const float WIND_MAX_STRENGTH         = 5.0f;
const float WIND_SAMPLE_SCALE         = 256.0f;
const vec2 WIND_SECOND_SAMPLE_OFFSET  = vec2(57.23f, -113.87f);
const vec2 WIND_STRENGTH_OFFSET       = vec2(-211.0f, 83.0f);
const vec2 WIND_DIRECTION_TIME_SCROLL = vec2(0.2f, -0.3f);
const vec2 WIND_STRENGTH_TIME_SCROLL  = vec2(-0.3f, 0.5f);
const float WIND_TIME_SCALE           = 200.0f;
const float WIND_DIRECTION_DETAIL_STRENGTH = 0.35f;

fnl_state wind_noise_state(int seed, float frequency) {
    fnl_state state    = fnlCreateState(seed);
    state.noise_type   = FNL_NOISE_OPENSIMPLEX2;
    state.fractal_type = FNL_FRACTAL_FBM;
    state.octaves      = 4;
    state.frequency    = frequency;
    state.lacunarity   = 2.0f;
    state.gain         = 0.5f;
    return state;
}

vec3 get_wind(vec3 world_pos, float time) {
    vec2 sample_pos     = world_pos.xz * WIND_SAMPLE_SCALE;
    float scroll_time   = time * WIND_TIME_SCALE;
    vec2 direction_time = WIND_DIRECTION_TIME_SCROLL * scroll_time;
    vec2 strength_time  = WIND_STRENGTH_TIME_SCROLL * scroll_time;

    fnl_state direction_state = wind_noise_state(WIND_DIRECTION_SEED, WIND_DIRECTION_FREQUENCY);
    fnl_state strength_state  = wind_noise_state(WIND_STRENGTH_SEED, WIND_STRENGTH_FREQUENCY);

    float primary_direction_noise =
        fnlGetNoise2D(direction_state, sample_pos.x + direction_time.x,
                      sample_pos.y + direction_time.y);
    float detail_direction_noise =
        fnlGetNoise2D(direction_state,
                      sample_pos.x + WIND_SECOND_SAMPLE_OFFSET.x + direction_time.x,
                      sample_pos.y + WIND_SECOND_SAMPLE_OFFSET.y + direction_time.y);

    float base_angle   = (primary_direction_noise * 0.5f + 0.5f) * TWO_PI;
    float detail_angle = detail_direction_noise * WIND_DIRECTION_DETAIL_STRENGTH;
    vec2 direction     = vec2(cos(base_angle + detail_angle), sin(base_angle + detail_angle));

    float strength_noise =
        fnlGetNoise2D(strength_state, sample_pos.x + WIND_STRENGTH_OFFSET.x + strength_time.x,
                      sample_pos.y + WIND_STRENGTH_OFFSET.y + strength_time.y);
    float strength = mix(WIND_MIN_STRENGTH, WIND_MAX_STRENGTH, strength_noise * 0.5f + 0.5f);

    vec2 wind_planar = direction * strength;
    return vec3(wind_planar.x, 0.0f, wind_planar.y);
}

#endif // WIND_GLSL
