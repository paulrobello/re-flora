#ifndef VOXEL_COLORS_GLSL
#define VOXEL_COLORS_GLSL

#include "./core/color.glsl"
#include "./voxel_types.glsl"

vec3 _voxel_color_by_type_srgb(uint voxel_type) {
    if (voxel_type == VOXEL_TYPE_EMPTY) {
        return vec3(0.0);
    } else if (voxel_type == VOXEL_TYPE_DIRT) {
        return voxel_colors.dirt_color;
    } else if (voxel_type == VOXEL_TYPE_CHERRY_WOOD) {
        return voxel_colors.trunk_color;
    } else if (voxel_type == VOXEL_TYPE_OAK_WOOD) {
        // Keep oak wood visibly darker than cherry wood by default.
        return voxel_colors.trunk_color * 0.82;
    }
    return vec3(0.0);
}

vec3 voxel_color_by_type_unorm(uint voxel_type) {
    return srgb_to_linear(_voxel_color_by_type_srgb(voxel_type));
}

#endif // VOXEL_COLORS_GLSL
