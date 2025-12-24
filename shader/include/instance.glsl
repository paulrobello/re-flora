#ifndef INSTANCE_GLSL
#define INSTANCE_GLSL

struct Instance {
    uvec3 pos;
    uint ty;
    uint bottom_color_seed;
    uint tip_color_seed;
    uint height;
    uint padding1;
};

#endif // INSTANCE_GLSL
