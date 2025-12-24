#version 450

#extension GL_GOOGLE_include_directive : require

#include "../include/core/packer.glsl"

layout(push_constant) uniform PC {
    float time;
    vec3 bottom_color;
    vec3 tip_color;
}
pc;

// these are vertex-rate attributes
layout(location = 0) in uint in_packed_data;

// these are instance-rate attributes
layout(location = 1) in uvec3 in_instance_pos;
layout(location = 2) in uint in_instance_ty_seed;

layout(location = 0) out vec3 vert_color;

layout(set = 0, binding = 0) uniform U_GuiInput {
    float debug_float;
    uint debug_bool;
    uint debug_uint;
}
gui_input;

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

layout(set = 0, binding = 4) uniform U_ShadowCameraInfo {
    vec4 pos;
    mat4 view_mat;
    mat4 view_mat_inv;
    mat4 proj_mat;
    mat4 proj_mat_inv;
    mat4 view_proj_mat;
    mat4 view_proj_mat_inv;
}
shadow_camera_info;

layout(set = 0, binding = 5) uniform sampler2D shadow_map_tex_for_vsm_ping;

#include "../include/core/color.glsl"
#include "../include/core/hash.glsl"
#include "../include/flora_registry.glsl"
#include "../include/instance.glsl"
#include "../include/vsm.glsl"
#include "../include/wind.glsl"
#include "./billboard.glsl"
#include "./palette.glsl"
#include "./unpacker.glsl"

const float scaling_factor          = 1.0 / 256.0;
const float grass_min_height_voxels = 3.0;
const float grass_max_height_voxels = 8.0;
const float grass_bucket_count      = grass_max_height_voxels - grass_min_height_voxels + 1.0;

float renormalize_gradient(float gradient, float visible_span) {
    return clamp(gradient / visible_span, 0.0, 1.0);
}

float sample_grass_height(uint seed) {
    uint h      = wellons_hash(seed);
    uint bucket = h % uint(grass_bucket_count);
    return grass_min_height_voxels + float(bucket);
}

float get_shadow_weight(ivec3 vox_local_pos) {
    vec3 vox_dir_normalized            = normalize(vec3(vox_local_pos));
    float shadow_negative_side_dropoff = max(0.0, dot(-vox_dir_normalized, sun_info.sun_dir));
    shadow_negative_side_dropoff       = pow(shadow_negative_side_dropoff, 2.0);
    float shadow_weight                = 1.0 - shadow_negative_side_dropoff;

    shadow_weight = max(0.7, shadow_weight);
    return shadow_weight;
}

vec3 clamp_to_grid(vec3 position) {
    // lower the grid size here to reduce chunky feeling, but maintain a good impression of pixel
    // vibe
    const float clamp_fac = scaling_factor * 0.5;
    return round(position / clamp_fac) * clamp_fac;
}

void main() {
    ivec3 vox_local_pos;
    uvec3 vert_offset_in_vox;
    float color_gradient;
    float wind_gradient;
    unpack_vertex_data(vox_local_pos, vert_offset_in_vox, color_gradient, wind_gradient,
                       in_packed_data);

    uint instance_ty   = decode_instance_ty(in_instance_ty_seed);
    uint instance_seed = decode_instance_seed(in_instance_ty_seed);
    bool is_grass      = instance_ty == FLORA_SPECIES_GRASS;
    float grass_height_voxels =
        is_grass ? sample_grass_height(instance_seed) : grass_max_height_voxels;
    float visible_gradient_span =
        max((grass_height_voxels - 1.0) / (grass_max_height_voxels - 1.0), 1e-3);
    if (is_grass) {
        color_gradient = renormalize_gradient(color_gradient, visible_gradient_span);
        wind_gradient  = renormalize_gradient(wind_gradient, visible_gradient_span);
    }
    bool should_trim_voxel = is_grass && (float(vox_local_pos.y) >= grass_height_voxels);

    vec3 instance_pos = in_instance_pos * scaling_factor;

    vec3 wind_vec    = get_wind(instance_pos, pc.time);
    vec3 wind_offset = wind_vec * wind_gradient * wind_gradient;
    vec3 anchor_pos  = clamp_to_grid((vox_local_pos + wind_offset) * scaling_factor + instance_pos);
    vec3 voxel_pos   = clamp_to_grid(anchor_pos + vec3(0.5) * scaling_factor);
    vec3 vert_pos = get_vert_pos_with_billboard(camera_info.view_mat, voxel_pos, vert_offset_in_vox,
                                                scaling_factor);

    if (should_trim_voxel) {
        voxel_pos = anchor_pos;
        vert_pos  = anchor_pos;
    }

    float shadow_weight =
        get_shadow_weight_vsm(shadow_camera_info.view_proj_mat, vec4(voxel_pos, 1.0));
    shadow_weight *= get_shadow_weight(vox_local_pos);

    gl_Position = camera_info.view_proj_mat * vec4(vert_pos, 1.0);

    uint palette_seed        = combine_color_seed(instance_seed);
    vec3 bottom_color_linear = srgb_to_linear(pc.bottom_color);
    vec3 tip_color_linear    = sample_tip_palette(instance_ty, palette_seed, pc.tip_color);
    vec3 interpolated_color  = mix(bottom_color_linear, tip_color_linear, color_gradient);

    vec3 sun_light = sun_info.sun_color * sun_info.sun_luminance;
    vert_color     = interpolated_color * (sun_light * shadow_weight + shading_info.ambient_light);
}
