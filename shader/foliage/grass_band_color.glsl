#ifndef GRASS_BAND_COLOR_GLSL
#define GRASS_BAND_COLOR_GLSL

#include "../include/core/color.glsl"

const int GRASS_BAND_NOISE_SEED         = 9041;
const float GRASS_BAND_NOISE_FREQUENCY  = 0.0018f;
const int GRASS_BAND_NOISE_OCTAVES      = 3;
const float GRASS_BAND_NOISE_LACUNARITY = 2.0f;
const float GRASS_BAND_NOISE_GAIN       = 0.5f;
const uint GRASS_BAND_COUNT             = 3u;

const vec3 GRASS_BAND_LUT[GRASS_BAND_COUNT] =
    vec3[](vec3(0.184, 0.345, 0.102), vec3(0.286, 0.490, 0.141), vec3(0.455, 0.655, 0.224));

float sample_grass_band_noise(vec2 world_xz) {
    fnl_state state    = fnlCreateState(GRASS_BAND_NOISE_SEED);
    state.noise_type   = FNL_NOISE_PERLIN;
    state.fractal_type = FNL_FRACTAL_FBM;
    state.octaves      = GRASS_BAND_NOISE_OCTAVES;
    state.frequency    = GRASS_BAND_NOISE_FREQUENCY;
    state.lacunarity   = GRASS_BAND_NOISE_LACUNARITY;
    state.gain         = GRASS_BAND_NOISE_GAIN;

    float noise = fnlGetNoise2D(state, world_xz.x, world_xz.y);
    return noise * 0.5f + 0.5f;
}

uint sample_grass_band_index(float noise_01) {
    float scaled = floor(noise_01 * float(GRASS_BAND_COUNT));
    return uint(min(scaled, float(GRASS_BAND_COUNT - 1u)));
}

vec3 sample_grass_band_color(vec2 world_xz) {
    float noise_01 = sample_grass_band_noise(world_xz);
    uint band_idx  = sample_grass_band_index(noise_01);
    return srgb_to_linear(GRASS_BAND_LUT[band_idx]);
}

#endif // GRASS_BAND_COLOR_GLSL
