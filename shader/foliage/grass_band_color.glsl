#ifndef GRASS_BAND_COLOR_GLSL
#define GRASS_BAND_COLOR_GLSL

#include "../include/core/color.glsl"
#include "../include/core/gradient_noise.glsl"

float sample_grass_band_noise(vec2 world_xz) {
    // Perlin gradient noise FBM (no large const tables — Metal-safe)
    float noise = fbm_cnoise_2d(world_xz.x, world_xz.y, 9041u, 0.008, 3, 2.0, 0.5);
    return noise * 0.5f + 0.5f;
}

float sample_grass_interpolation_t(float noise_01) {
    return clamp(noise_01, 0.0f, 1.0f);
}

void sample_grass_band_gradient(vec2 world_xz, out vec3 bottom_color_linear,
                                out vec3 tip_color_linear) {
    float noise_01 = sample_grass_band_noise(world_xz);
    float interp_t = sample_grass_interpolation_t(noise_01);

    vec3 bottom_srgb = mix(gui_input.grass_bottom_dark, gui_input.grass_bottom_light, interp_t);
    vec3 tip_srgb    = mix(gui_input.grass_tip_dark, gui_input.grass_tip_light, interp_t);

    bottom_color_linear = srgb_to_linear(bottom_srgb);
    tip_color_linear    = srgb_to_linear(tip_srgb);
}

#endif // GRASS_BAND_COLOR_GLSL
