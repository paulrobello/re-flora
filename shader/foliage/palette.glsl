#ifndef FLORA_PALETTE_GLSL
#define FLORA_PALETTE_GLSL

#include "../include/core/color.glsl"
#include "../include/core/hash.glsl"
#include "../include/flora_registry.glsl"

const uint FLORA_PALETTE_LEAVES = 3u;

// Palette colors are pre-linearized (sRGB -> linear applied at compile time)
// to avoid a per-vertex pow(x, 2.4) call, which is expensive on Metal/MoltenVK.
// Values are mathematically identical to srgb_to_linear() of the original sRGB
// inputs (see color.glsl for the transform). Original sRGB values are kept in
// the comments for reference.

const uint LAVENDER_PALETTE_LEN = 6u;
const vec3 LAVENDER_TIP_PALETTE[LAVENDER_PALETTE_LEN] =
    vec3[](vec3(0.963988, 0.520965, 0.017604), // Golden Bloom  [sRGB 0.984, 0.749, 0.141]
           vec3(0.963988, 0.165023, 0.234972), // Rose Petal    [sRGB 0.984, 0.443, 0.522]
           vec3(0.963988, 0.624367, 0.807346), // Soft Peony    [sRGB 0.984, 0.812, 0.910]
           vec3(0.752262, 0.012335, 0.064641), // Deep Rose     [sRGB 0.882, 0.114, 0.282]
           vec3(0.204902, 0.650607, 0.972918), // Sky Azure     [sRGB 0.490, 0.827, 0.988]
           vec3(0.258082, 0.180539, 0.939675)  // Deep Violet   [sRGB 0.545, 0.462, 0.973]
    );

const uint LEAF_PALETTE_LEN = 4u;
const vec3 LEAF_TIP_PALETTE[LEAF_PALETTE_LEN] =
    vec3[](vec3(1.000000, 0.955105, 0.870997), // Pearly White  [sRGB 1.000, 0.980, 0.941]
           vec3(1.000000, 0.831651, 0.854301), // Sakura Blush  [sRGB 1.000, 0.922, 0.933]
           vec3(0.913564, 0.913564, 0.716171), // Magnolia Cream[sRGB 0.961, 0.961, 0.863]
           vec3(0.862625, 0.610629, 0.701283)  // Budding Rose  [sRGB 0.937, 0.804, 0.855]
    );

uint combine_color_seed(uint seed) {
    // lightweight scramble to decorrelate neighboring instances
    return wellons_hash(seed);
}

// Uniformly map a seed to a palette bucket in [0, palette_len)
uint sample_palette_bucket(uint seed, uint palette_len) {
    float r = construct_float_01(seed);
    return uint(r * float(palette_len));
}

vec3 sample_tip_palette(uint palette_id, uint seed, vec3 fallback_tip_color) {
    if (palette_id == FLORA_SPECIES_LAVENDER) {
        uint idx = sample_palette_bucket(seed, LAVENDER_PALETTE_LEN);
        // LAVENDER_TIP_PALETTE is pre-linearized; no srgb_to_linear needed.
        return LAVENDER_TIP_PALETTE[idx];
    }

    if (palette_id == FLORA_PALETTE_LEAVES) {
        uint idx = sample_palette_bucket(seed, LEAF_PALETTE_LEN);
        // LEAF_TIP_PALETTE is pre-linearized; no srgb_to_linear needed.
        return LEAF_TIP_PALETTE[idx];
    }

    return srgb_to_linear(fallback_tip_color);
}

#endif // FLORA_PALETTE_GLSL
