#ifndef INSTANCE_GLSL
#define INSTANCE_GLSL

struct Instance {
    uint pos_x;
    uint pos_y;
    uint pos_z;
    // Lower 12 bits: type, upper 20 bits: seed
    uint ty_seed;
    uint growth_start_tick;
};

uvec3 get_instance_pos(Instance instance) {
    return uvec3(instance.pos_x, instance.pos_y, instance.pos_z);
}

void set_instance_pos(inout Instance instance, uvec3 pos) {
    instance.pos_x = pos.x;
    instance.pos_y = pos.y;
    instance.pos_z = pos.z;
}

const uint INSTANCE_TY_BITS   = 12u;
const uint INSTANCE_SEED_BITS = 20u;
const uint INSTANCE_TY_MASK   = (1u << INSTANCE_TY_BITS) - 1u;
const uint INSTANCE_SEED_MASK = (1u << INSTANCE_SEED_BITS) - 1u;

uint pack_instance_ty_seed(uint ty, uint seed) {
    return (ty & INSTANCE_TY_MASK) | ((seed & INSTANCE_SEED_MASK) << INSTANCE_TY_BITS);
}

uint decode_instance_ty(uint ty_seed) { return ty_seed & INSTANCE_TY_MASK; }

uint decode_instance_seed(uint ty_seed) {
    return (ty_seed >> INSTANCE_TY_BITS) & INSTANCE_SEED_MASK;
}

#endif // INSTANCE_GLSL
