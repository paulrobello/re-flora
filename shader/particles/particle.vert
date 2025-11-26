#version 450

#extension GL_GOOGLE_include_directive : require

#include "../foliage/unpacker.glsl"
#include "../include/core/color.glsl"

layout(location = 0) in uint in_packed_data;
layout(location = 1) in vec3 in_instance_pos;
layout(location = 2) in float in_instance_size;
layout(location = 3) in vec4 in_instance_color;

layout(location = 0) out vec4 vert_color;

layout(set = 0, binding = 1) uniform U_SunInfo {
    vec3 sun_dir;
    float sun_size;
    vec3 sun_color;
    float sun_luminance;
    float sun_altitude;
    float sun_azimuth;
}
sun_info;

layout(set = 0, binding = 2) uniform U_ShadingInfo { vec3 ambient_light; }
shading_info;

layout(set = 0, binding = 3) uniform U_CameraInfo {
    vec4 pos;
    mat4 view_mat;
    mat4 view_mat_inv;
    mat4 proj_mat;
    mat4 proj_mat_inv;
    mat4 view_proj_mat;
    mat4 view_proj_mat_inv;
}
camera_info;

const float BASE_VOXEL_SCALE = 1.0 / 256.0;

void main() {
    ivec3 vox_local_pos;
    uvec3 vert_offset_in_vox;
    float color_gradient;
    float wind_gradient;
    unpack_vertex_data(vox_local_pos, vert_offset_in_vox, color_gradient, wind_gradient,
                       in_packed_data);

    float scale = max(in_instance_size, 0.001);
    vec3 local_anchor = vec3(vox_local_pos) * BASE_VOXEL_SCALE * scale;
    vec3 vertex_offset = vec3(vert_offset_in_vox) * BASE_VOXEL_SCALE * scale;

    vec3 vertex_pos = in_instance_pos + local_anchor + vertex_offset;

    gl_Position = camera_info.view_proj_mat * vec4(vertex_pos, 1.0);

    vec3 sun_light = sun_info.sun_color * sun_info.sun_luminance;
    vec3 lighting = max(vec3(0.0), sun_light + shading_info.ambient_light);
    vec3 linear_color = srgb_to_linear(in_instance_color.rgb);
    vert_color = vec4(linear_color * lighting, in_instance_color.a);
}
