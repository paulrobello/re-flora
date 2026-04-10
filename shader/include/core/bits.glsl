#ifndef BITS_GLSL
#define BITS_GLSL

// NOTE: Original used uint64_t / GL_ARB_gpu_shader_int64 but Metal/MoltenVK
// lacks native 64-bit integer support, causing massive performance regression.
// All operations now use uint32 pair (child_mask_lo, child_mask_hi).

uint bit_count_dual(uint lo, uint hi) { return bitCount(lo) + bitCount(hi); }

// Count set bits in dual mask below the given index
uint bit_count_dual_var(uint lo, uint hi, uint width) {
    if (width < 32u) {
        return bitCount(lo & ((1u << width) - 1u));
    } else {
        return bitCount(lo) + bitCount(hi & ((1u << (width - 32u)) - 1u));
    }
}

#endif // BITS_GLSL
