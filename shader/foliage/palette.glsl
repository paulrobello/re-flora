#ifndef FLORA_PALETTE_GLSL
#define FLORA_PALETTE_GLSL

#include "../include/core/hash.glsl"
#include "../include/flora_registry.glsl"

const uint FLORA_PALETTE_LEAVES = 3u;

// Pre-linearized palette colors (sRGB→linear computed at compile time)
// to avoid per-vertex pow(x, 2.4) calls which are extremely expensive on Metal/MoltenVK.

const uint LAVENDER_PALETTE_LEN = 6u;
const vec3 LAVENDER_TIP_PALETTE[LAVENDER_PALETTE_LEN] =
    vec3[](vec3(0.963988, 0.520965, 0.017604), // Golden Bloom
           vec3(0.963988, 0.165023, 0.234972), // Rose Petal
           vec3(0.963988, 0.624367, 0.807346), // Soft Peony
           vec3(0.752262, 0.012335, 0.064641), // Deep Rose
           vec3(0.204902, 0.650607, 0.972918), // Sky Azure
           vec3(0.258082, 0.180539, 0.939675)  // Deep Violet
    );

const uint LEAF_PALETTE_LEN = 4u;
const vec3 LEAF_TIP_PALETTE[LEAF_PALETTE_LEN] =
    vec3[](vec3(1.000000, 0.955105, 0.870997), // Pearly White (Apple/Pear Blossom)
           vec3(1.000000, 0.831651, 0.854301), // Sakura Blush (Cherry Blossom)
           vec3(0.913564, 0.913564, 0.716171), // Magnolia Cream
           vec3(0.862625, 0.610629, 0.701283)  // Budding Rose
    );

uint combine_color_seed(uint seed) {
    return wellons_hash(seed);
}

uint sample_palette_bucket(uint seed, uint palette_len) {
    float r = construct_float_01(seed);
    return uint(r * float(palette_len));
}

// Returns pre-linearized color — no srgb_to_linear needed
vec3 sample_tip_palette(uint palette_id, uint seed, vec3 fallback_tip_color_linear) {
    if (palette_id == FLORA_SPECIES_LAVENDER) {
        uint idx = sample_palette_bucket(seed, LAVENDER_PALETTE_LEN);
        return LAVENDER_TIP_PALETTE[idx];
    }

    if (palette_id == FLORA_PALETTE_LEAVES) {
        // Tint the GUI tip color with subtle per-instance variation from the palette
        uint idx = sample_palette_bucket(seed, LEAF_PALETTE_LEN);
        vec3 palette_tint = LEAF_TIP_PALETTE[idx];
        return fallback_tip_color_linear * (palette_tint * 0.3 + 0.7);
    }

    return fallback_tip_color_linear;
}

#endif // FLORA_PALETTE_GLSL
