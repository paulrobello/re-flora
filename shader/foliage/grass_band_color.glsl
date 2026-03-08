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
    vec3[](vec3(0.000, 0.350, 0.200), vec3(0.020, 0.400, 0.220), vec3(0.040, 0.450, 0.240),
           vec3(0.060, 0.500, 0.260), vec3(0.080, 0.550, 0.280), vec3(0.100, 0.600, 0.300),
           vec3(0.120, 0.650, 0.320), vec3(0.150, 0.700, 0.350), vec3(0.180, 0.750, 0.380));

const vec3 GRASS_TIP_LUT[GRASS_BAND_VARIANT_LUT_LEN] =
    vec3[](vec3(0.300, 0.750, 0.100), vec3(0.380, 0.800, 0.120), vec3(0.460, 0.840, 0.140),
           vec3(0.540, 0.880, 0.160), vec3(0.620, 0.920, 0.180), vec3(0.700, 0.950, 0.200),
           vec3(0.780, 0.970, 0.220), vec3(0.860, 0.990, 0.240), vec3(0.950, 1.000, 0.260));

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
