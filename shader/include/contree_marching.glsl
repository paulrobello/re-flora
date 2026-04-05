#ifndef CONTREE_MARCHING_GLSL
#define CONTREE_MARCHING_GLSL

// shared stack per work-group invocation
// notice: the dispatch size is required to be 64 (e.g  8x8x1) for this to work
shared uint gs_stack[64][11];

#include "../include/contree_node.glsl"
#include "../include/core/aabb.glsl"
#include "../include/core/bits.glsl"

struct ContreeMarchingResult {
    bool is_hit;
    vec3 pos;
    vec3 center_pos;
    uint voxel_addr;
};

// reverses pos from [1.0,2.0) to (2.0,1.0] if dir>0
vec3 get_mirrored_pos(vec3 pos, vec3 dir, bool range_check) {
    uvec3 pu      = floatBitsToUint(pos);
    uvec3 flipped = pu ^ uvec3(0x7FFFFFu);
    vec3 mirrored = uintBitsToFloat(flipped);

    if (range_check) {
        if (any(lessThan(pos, vec3(1.0))) || any(greaterThanEqual(pos, vec3(2.0)))) {
            mirrored = vec3(3.0) - pos;
        }
    }
    return mix(pos, mirrored, greaterThan(dir, vec3(0.0)));
}

// compute child index [0..26) from bits of pos at this scale
int get_node_cell_index(vec3 pos, int scale_exp) {
    uvec3 pu      = floatBitsToUint(pos);
    uvec3 cellpos = (pu >> uint(scale_exp)) & 3u;
    return int(cellpos.x + cellpos.z * 4u + cellpos.y * 16u);
}

// floor(pos / scale) * scale by zeroing low bits of float bitpattern
vec3 floor_scale(vec3 pos, int scale_exp) {
    uint mask = ~0u << uint(scale_exp);
    uvec3 pu  = floatBitsToUint(pos);
    uvec3 r   = pu & uvec3(mask);
    return uintBitsToFloat(r);
}

/// node_offset is the offset when addressing the contree node data
/// leaf_offset is the offset when addressing the contree leaf data
ContreeMarchingResult _contree_marching(vec3 origin, vec3 dir, bool coarse, uint node_offset,
                                        uint leaf_offset) {
    uint group_id    = gl_LocalInvocationIndex;

    for (uint si = 0u; si < 11u; ++si) {
        gs_stack[group_id][si] = 0u;
    }

    int scale_exp    = 21;
    uint node_idx    = 0u;
    ContreeNode node = contree_node_data.data[node_offset + node_idx];

    ContreeMarchingResult res;
    res.is_hit     = false;
    res.voxel_addr = 0;
    res.pos        = vec3(0.0);
    res.center_pos = vec3(0.0);

    vec2 slab = slabs(vec3(1.0), vec3(1.9999999), origin, 1.0 / dir);
    if (slab.x > slab.y || slab.y < 0.0) {
        return res;
    }
    origin += max(slab.x, 0.0) * dir;

    uint mirror_mask = 0u;
    if (dir.x > 0.0) mirror_mask |= 3u << 0;
    if (dir.y > 0.0) mirror_mask |= 3u << 4;
    if (dir.z > 0.0) mirror_mask |= 3u << 2;

    origin         = get_mirrored_pos(origin, dir, true);
    vec3 pos       = clamp(origin, 1.0, 1.9999999);
    vec3 inv_dir   = 1.0 / -abs(dir);
    vec3 side_dist = vec3(0.0);

    for (int i = 0; i < 1024; ++i) {
        if (coarse && i > 20 && (node.packed_0 & 1u) != 0u) {
            break;
        }

        uint child_idx = uint(get_node_cell_index(pos, scale_exp)) ^ mirror_mask;

        // descend as far as possible
        while (child_mask_test(node, child_idx) && (node.packed_0 & 1u) == 0u) {
            uint stk_idx                = uint(scale_exp >> 1);
            gs_stack[group_id][stk_idx] = node_idx;

            uint bits = child_mask_bitcount_below(node, child_idx);
            node_idx  = (node.packed_0 >> 1u) + bits;
            node      = contree_node_data.data[node_offset + node_idx];

            scale_exp -= 2;
            child_idx = uint(get_node_cell_index(pos, scale_exp)) ^ mirror_mask;
        }

        // if leaf has that child, stop
        if (child_mask_test(node, child_idx) && (node.packed_0 & 1u) != 0u) {
            break;
        }

        // figure out how far to step
        int adv_scale_exp = scale_exp;
        // neighbor-cell optimization using dual uint
        // mask 0x00330033 selects the 8 neighbor cells in the same octree row
        uint shifted_idx = child_idx & 0x2Au;
        bool has_neighbor;
        if (shifted_idx < 32u) {
            has_neighbor = ((node.child_mask_lo >> shifted_idx) & 0x00330033u) != 0u;
        } else {
            has_neighbor = ((node.child_mask_hi >> (shifted_idx - 32u)) & 0x00330033u) != 0u;
        }
        if (!has_neighbor) {
            adv_scale_exp++;
        }

        // intersect ray with current cell face
        vec3 cell_min = floor_scale(pos, adv_scale_exp);
        side_dist     = (cell_min - origin) * inv_dir;
        float tmax    = min(min(side_dist.x, side_dist.y), side_dist.z);

        bvec3 side_mask    = bvec3(tmax >= side_dist.x, tmax >= side_dist.y, tmax >= side_dist.z);
        ivec3 base         = ivec3(floatBitsToInt(cell_min));
        ivec3 off          = ivec3((1 << adv_scale_exp) - 1);
        ivec3 neighbor_max = base + mix(off, ivec3(-1), side_mask);

        pos = min(origin - abs(dir) * tmax, intBitsToFloat(neighbor_max));

        uvec3 diff_pos = floatBitsToUint(pos) ^ floatBitsToUint(cell_min);
        uint combined  = (diff_pos.x | diff_pos.y | diff_pos.z) & 0xFFAAAAAAu;
        int diff_exp   = findMSB(int(combined));
        if (diff_exp > scale_exp) {
            scale_exp = diff_exp;
            if (diff_exp > 21) break;
            uint stk_idx = uint(scale_exp >> 1);
            node_idx     = gs_stack[group_id][stk_idx];
            node         = contree_node_data.data[node_offset + node_idx];
        }
    }

    // if we ended in a leaf
    if ((node.packed_0 & 1u) != 0u && scale_exp <= 21) {
        res.is_hit = true;

        vec3 centered_pos = floor_scale(pos, scale_exp);
        float offset = uintBitsToFloat(0x3f800000u | (1u << (scale_exp - 1))) - 1.0;
        centered_pos += offset;

        bvec3 flip   = greaterThan(dir, vec3(0.0));
        pos          = get_mirrored_pos(pos, dir, false);
        centered_pos = mix(centered_pos, 3.0 - centered_pos, flip);

        uint child_idx = uint(get_node_cell_index(pos, scale_exp));
        uint bits      = child_mask_bitcount_below(node, child_idx);

        res.pos        = pos;
        res.center_pos = centered_pos;
        res.voxel_addr = leaf_offset + (node.packed_0 >> 1u) + bits;
    }

    return res;
}

ContreeMarchingResult contree_marching(vec3 o,              // world-space ray origin
                                       vec3 d,              // world-space ray direction
                                       vec3 chunk_position, // world-space min corner of the chunk
                                       vec3 chunk_scaling,  // size of the chunk along each axis
                                       bool coarse,         // if coarse ray is used
                                       uint node_offset,    // offset in the global node buffer
                                       uint leaf_offset     // offset in the global leaf buffer
) {
    vec3 local_o = (o - chunk_position) / chunk_scaling + 1.0;
    vec3 local_d = d / chunk_scaling;

    ContreeMarchingResult result =
        _contree_marching(local_o, local_d, coarse, node_offset, leaf_offset);

    result.pos        = (result.pos - 1.0) * chunk_scaling + chunk_position;
    result.center_pos = (result.center_pos - 1.0) * chunk_scaling + chunk_position;

    return result;
}

#endif // CONTREE_MARCHING_GLSL
