#ifndef GRASS_BAND_COLOR_GLSL
#define GRASS_BAND_COLOR_GLSL

#include "../include/core/color.glsl"
const int GRASS_BAND_NOISE_SEED         = 9041;
const float GRASS_BAND_NOISE_FREQUENCY  = 0.008f;
const int GRASS_BAND_NOISE_OCTAVES      = 3;
const float GRASS_BAND_NOISE_LACUNARITY = 2.0f;
const float GRASS_BAND_NOISE_GAIN       = 0.5f;
const uint GRASS_BAND_COUNT             = 3u;
const uint GRASS_VARIANT_COUNT          = 3u;
const uint GRASS_BAND_VARIANT_LUT_LEN   = GRASS_BAND_COUNT * GRASS_VARIANT_COUNT;

const vec3 GRASS_BOTTOM_LUT[GRASS_BAND_VARIANT_LUT_LEN] =
    vec3[](vec3(0.231, 0.380, 0.122), vec3(0.255, 0.404, 0.129), vec3(0.275, 0.424, 0.141),
           vec3(0.325, 0.506, 0.169), vec3(0.353, 0.533, 0.184), vec3(0.373, 0.557, 0.196),
           vec3(0.443, 0.616, 0.243), vec3(0.471, 0.651, 0.263), vec3(0.498, 0.675, 0.282));

const vec3 GRASS_TIP_LUT[GRASS_BAND_VARIANT_LUT_LEN] =
    vec3[](vec3(0.424, 0.627, 0.220), vec3(0.451, 0.655, 0.235), vec3(0.478, 0.682, 0.251),
           vec3(0.565, 0.753, 0.333), vec3(0.596, 0.776, 0.357), vec3(0.624, 0.800, 0.380),
           vec3(0.710, 0.851, 0.482), vec3(0.737, 0.867, 0.514), vec3(0.761, 0.886, 0.541));

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

uint sample_grass_variant_index(float noise_01, uint band_idx) {
    float band_pos = noise_01 * float(GRASS_BAND_COUNT);
    float local_band_pos = clamp(band_pos - float(band_idx), 0.0f, 0.9999f);
    float scaled = floor(local_band_pos * float(GRASS_VARIANT_COUNT));
    return uint(min(scaled, float(GRASS_VARIANT_COUNT - 1u)));
}

void sample_grass_band_gradient(vec2 world_xz, out vec3 bottom_color_linear,
                                out vec3 tip_color_linear) {
    float noise_01    = sample_grass_band_noise(world_xz);
    uint band_idx     = sample_grass_band_index(noise_01);
    uint variant_idx  = sample_grass_variant_index(noise_01, band_idx);
    uint palette_idx  = band_idx * GRASS_VARIANT_COUNT + variant_idx;
    bottom_color_linear = srgb_to_linear(GRASS_BOTTOM_LUT[palette_idx]);
    tip_color_linear    = srgb_to_linear(GRASS_TIP_LUT[palette_idx]);
}

#endif // GRASS_BAND_COLOR_GLSL
