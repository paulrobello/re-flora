#ifndef FLORA_PALETTE_GLSL
#define FLORA_PALETTE_GLSL

#include "../include/core/color.glsl"
#include "../include/core/hash.glsl"
#include "../include/flora_registry.glsl"

const uint FLORA_PALETTE_LEAVES = 3u;

const uint LAVENDER_PALETTE_LEN = 6u;
const vec3 LAVENDER_TIP_PALETTE[LAVENDER_PALETTE_LEN] =
    vec3[](vec3(0.984, 0.749, 0.141), // Golden Bloom
           vec3(0.984, 0.443, 0.522), // Rose Petal
           vec3(0.984, 0.812, 0.910), // Soft Peony
           vec3(0.882, 0.114, 0.282), // Deep Rose
           vec3(0.490, 0.827, 0.988), // Sky Azure
           vec3(0.545, 0.462, 0.973)  // Deep Violet
    );

const uint LEAF_PALETTE_LEN                   = 1u;
const vec3 LEAF_TIP_PALETTE[LEAF_PALETTE_LEN] = vec3[](vec3(217, 242, 0) / 255.0);

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
        return srgb_to_linear(LAVENDER_TIP_PALETTE[idx]);
    }

    if (palette_id == FLORA_PALETTE_LEAVES) {
        uint idx = sample_palette_bucket(seed, LEAF_PALETTE_LEN);
        return srgb_to_linear(LEAF_TIP_PALETTE[idx]);
    }

    return srgb_to_linear(fallback_tip_color);
}

#endif // FLORA_PALETTE_GLSL
