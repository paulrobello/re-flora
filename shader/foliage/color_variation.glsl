#ifndef FLORA_COLOR_VARIATION_GLSL
#define FLORA_COLOR_VARIATION_GLSL

#include "../include/core/color.glsl"
#include "../include/core/hash.glsl"

vec3 signed_unit_noise(float seed) { return hash_31(seed) * 2.0 - 1.0; }

vec3 signed_unit_noise(vec4 seed) { return hash_34(seed) * 2.0 - 1.0; }

vec3 apply_hsv_offset(vec3 linear_rgb, vec3 hsv_offset) {
    vec3 hsv = rgb_to_hsv(linear_rgb);
    hsv.x    = fract(hsv.x + hsv_offset.x);
    hsv.y    = clamp(hsv.y + hsv_offset.y, 0.0, 1.0);
    hsv.z    = clamp(hsv.z + hsv_offset.z, 0.0, 1.0);
    return hsv_to_rgb(hsv);
}

#endif // FLORA_COLOR_VARIATION_GLSL
