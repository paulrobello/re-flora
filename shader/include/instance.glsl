#ifndef INSTANCE_GLSL
#define INSTANCE_GLSL

struct Instance {
    uvec3 pos;
    // Lower 12 bits: type, upper 20 bits: seed
    uint ty_seed;
};

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
