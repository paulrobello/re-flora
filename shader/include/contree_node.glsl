#ifndef CONTREE_NODE_GLSL
#define CONTREE_NODE_GLSL

// NOTE: Original child_mask was uint64_t, but Metal/MoltenVK does not support
// 64-bit integers natively — software emulation causes ~170x slowdown in the
// ray traversal loop. Split into two uint32 fields instead.

struct ContreeNode {
    uint packed_0;      // [0]=is_leaf, [1..31]=child_ptr
    uint child_mask_lo; // bits [0..31] of the 64-child bitmask
    uint child_mask_hi; // bits [32..63] of the 64-child bitmask
};

// Helpers for child_mask access
bool child_mask_test(ContreeNode node, uint idx) {
    if (idx < 32u) {
        return (node.child_mask_lo & (1u << idx)) != 0u;
    } else {
        return (node.child_mask_hi & (1u << (idx - 32u))) != 0u;
    }
}

// Count set bits in child_mask below the given index
uint child_mask_bitcount_below(ContreeNode node, uint idx) {
    if (idx < 32u) {
        return bitCount(node.child_mask_lo & ((1u << idx) - 1u));
    } else {
        return bitCount(node.child_mask_lo) +
               bitCount(node.child_mask_hi & ((1u << (idx - 32u)) - 1u));
    }
}

// Test bits in a range: checks (mask >> (idx & pattern)) & wide_mask
// Used for the neighbor-cell optimization in contree_marching
bool child_mask_test_pattern(ContreeNode node, uint idx, uint wide_mask) {
    // idx & 0x2A maps bits: 0->0,1->0, 2->2,3->2, 4->4,5->4, ...
    uint shifted_idx = idx & 0x2Au;
    if (shifted_idx < 32u) {
        return ((node.child_mask_lo >> shifted_idx) & wide_mask) != 0u;
    } else {
        return ((node.child_mask_hi >> (shifted_idx - 32u)) & wide_mask) != 0u;
    }
}

#endif // CONTREE_NODE_GLSL
