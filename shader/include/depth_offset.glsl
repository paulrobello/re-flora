#ifndef DEPTH_OFFSET_GLSL
#define DEPTH_OFFSET_GLSL

// Common depth offset utilities for preventing z-fighting in instanced geometry

// Compute hash from instance position for consistent per-instance offset
uint compute_instance_hash(uvec3 instance_pos) {
    return instance_pos.x * 73856093u ^ instance_pos.y * 19349663u ^ instance_pos.z * 83492791u;
}

uint compute_instance_hash(vec3 instance_pos_ws) {
    const float hash_grid_scale = 256.0;
    uvec3 quantized_pos = uvec3(round(instance_pos_ws * hash_grid_scale));
    return compute_instance_hash(quantized_pos);
}

// Apply view-space depth offset based on instance hash to prevent z-fighting
// This gives consistent world-space offset regardless of distance from camera
vec4 apply_depth_offset(vec3 world_pos, uvec3 instance_pos, mat4 view_mat, mat4 proj_mat) {
    uint hash          = compute_instance_hash(instance_pos);
    float depth_offset = fract(float(hash) * 0.0001) * 5e-4;

    // Transform to view space, apply offset, then project
    vec4 view_pos = view_mat * vec4(world_pos, 1.0);
    view_pos.z -= depth_offset; // Push away from camera in view space
    return proj_mat * view_pos;
}

vec4 apply_depth_offset(vec3 world_pos, vec3 instance_pos_ws, mat4 view_mat, mat4 proj_mat) {
    uint hash          = compute_instance_hash(instance_pos_ws);
    float depth_offset = fract(float(hash) * 0.0001) * 5e-4;

    vec4 view_pos = view_mat * vec4(world_pos, 1.0);
    view_pos.z -= depth_offset;
    return proj_mat * view_pos;
}

#endif // DEPTH_OFFSET_GLSL
