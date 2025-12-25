#ifndef UNPACKER_GLSL
#define UNPACKER_GLSL

const uint BIT_PER_POS    = 7u;
const uint BIT_PER_OFFSET = 1u;
const uint BIT_PER_ORIGIN = 7u;

const uint POS_BITS        = BIT_PER_POS * 3u;
const uint OFFSET_BITS     = BIT_PER_OFFSET * 3u;
const uint ORIGIN_BITS     = BIT_PER_ORIGIN * 3u;
const uint MAX_LENGTH_BITS = 32u - ORIGIN_BITS;

const uint POS_MASK        = (1u << BIT_PER_POS) - 1u;
const uint OFFSET_MASK     = (1u << BIT_PER_OFFSET) - 1u;
const uint ORIGIN_MASK     = (1u << BIT_PER_ORIGIN) - 1u;
const uint MAX_LENGTH_MASK = (1u << MAX_LENGTH_BITS) - 1u;

void unpack_vertex_data(out ivec3 o_vox_local_pos, out uvec3 o_vert_offset_in_vox,
                        out ivec3 o_origin, out uint o_max_length, uvec2 packed_data) {
    uint packed_pos_offset     = packed_data.x;
    uint packed_origin_length  = packed_data.y;

    uint pos_x = packed_pos_offset & POS_MASK;
    uint pos_y = (packed_pos_offset >> BIT_PER_POS) & POS_MASK;
    uint pos_z = (packed_pos_offset >> (BIT_PER_POS * 2u)) & POS_MASK;

    const int POS_OFFSET = 1 << (BIT_PER_POS - 1u);
    o_vox_local_pos      = ivec3(pos_x, pos_y, pos_z) - POS_OFFSET;

    uint offset_packed = (packed_pos_offset >> POS_BITS) & ((1u << OFFSET_BITS) - 1u);
    o_vert_offset_in_vox =
        uvec3(offset_packed & OFFSET_MASK, (offset_packed >> BIT_PER_OFFSET) & OFFSET_MASK,
              (offset_packed >> (BIT_PER_OFFSET * 2u)) & OFFSET_MASK);

    uint origin_x = packed_origin_length & ORIGIN_MASK;
    uint origin_y = (packed_origin_length >> BIT_PER_ORIGIN) & ORIGIN_MASK;
    uint origin_z = (packed_origin_length >> (BIT_PER_ORIGIN * 2u)) & ORIGIN_MASK;

    const int ORIGIN_OFFSET = 1 << (BIT_PER_ORIGIN - 1u);
    o_origin                = ivec3(origin_x, origin_y, origin_z) - ORIGIN_OFFSET;

    o_max_length = (packed_origin_length >> ORIGIN_BITS) & MAX_LENGTH_MASK;
}

float compute_gradient(ivec3 vox_local_pos, ivec3 origin, uint max_length) {
    float denom   = max(float(max_length), 1.0);
    float dist    = length(vec3(vox_local_pos - origin));
    return clamp(dist / denom, 0.0, 1.0);
}

#endif // UNPACKER_GLSL
