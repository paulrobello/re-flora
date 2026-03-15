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
layout(location = 0) in uvec2 in_packed_data;

// these are instance-rate attributes
layout(location = 1) in uvec3 in_instance_pos;
layout(location = 2) in uint in_instance_ty_seed;
layout(location = 3) in uint in_instance_growth_start_tick;

layout(location = 0) out vec3 vert_color;

layout(set = 0, binding = 0) uniform U_GuiInput {
    float debug_float;
    uint debug_bool;
    uint debug_uint;
    vec3 flora_instance_hsv_offset_max;
    vec3 flora_voxel_hsv_offset_max;
    vec3 grass_bottom_dark;
    vec3 grass_bottom_light;
    vec3 grass_tip_dark;
    vec3 grass_tip_light;
}
gui_input;

layout(set = 0, binding = 1) uniform U_SunInfo {
    vec3 sun_dir;
    float sun_size;
    vec3 sun_color;
    float sun_luminance;
    float sun_display_luminance;
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

layout(set = 0, binding = 6) uniform U_FloraGrowthInfo {
    uint flora_tick;
    uint sprout_delay_ticks;
    uint full_growth_ticks;
}
flora_growth_info;

#include "../include/core/color.glsl"
#include "../include/core/hash.glsl"
#include "../include/depth_offset.glsl"
#include "../include/flora_registry.glsl"
#include "../include/instance.glsl"
#include "../include/sunlight.glsl"
#include "../include/vsm.glsl"
#include "../include/wind.glsl"
#include "./color_variation.glsl"
#include "./grass_band_color.glsl"
#include "./palette.glsl"
#include "./unpacker.glsl"

const float scaling_factor             = 1.0 / 256.0;
const uint grass_min_height_voxels     = 3u;
const uint grass_max_height_voxels     = 8u;
const float grass_height_mean_voxels   = 6.0;
const float grass_height_stddev_voxels = 1.0;

// bucketed wind update for flora instances (mirrors particle update buckets)
const uint  FLORA_UPDATE_BUCKET_COUNT = 4u;
const float FLORA_FULL_UPDATE_SECONDS = 0.15f;

float sample_standard_normal(uint seed) {
    float sum  = 0.0;
    uint state = seed ^ 0xA511E9B3u;
    for (uint i = 0u; i < 12u; ++i) {
        state = wellons_hash(state + i * 0x9E3779B9u);
        sum += construct_float_01(state);
    }
    return sum - 6.0;
}

uint sample_grass_height(uint seed) {
    float sampled_height =
        grass_height_mean_voxels + sample_standard_normal(seed) * grass_height_stddev_voxels;
    sampled_height =
        clamp(sampled_height, float(grass_min_height_voxels), float(grass_max_height_voxels));
    return uint(round(sampled_height));
}

float grass_growth_factor(uint growth_start_tick) {
    if (flora_growth_info.full_growth_ticks <= flora_growth_info.sprout_delay_ticks) {
        return 1.0;
    }
    uint age_ticks = flora_growth_info.flora_tick - growth_start_tick;
    return smoothstep(float(flora_growth_info.sprout_delay_ticks),
                      float(flora_growth_info.full_growth_ticks), float(age_ticks));
}

float get_shadow_weight(ivec3 vox_local_pos) {
    vec3 vox_dir_normalized            = normalize(vec3(vox_local_pos));
    float shadow_negative_side_dropoff = max(0.0, dot(-vox_dir_normalized, sun_info.sun_dir));
    shadow_negative_side_dropoff       = pow(shadow_negative_side_dropoff, 2.0);
    float shadow_weight                = 1.0 - shadow_negative_side_dropoff;

    shadow_weight = max(0.7, shadow_weight);
    return shadow_weight;
}

// remap global time to a per-instance bucketed time T
// bucket size is fixed (FLORA_UPDATE_BUCKET_COUNT) and full cycle is FLORA_FULL_UPDATE_SECONDS
float flora_bucketed_time(float raw_time, uint instance_seed) {
    const uint bucket_count = FLORA_UPDATE_BUCKET_COUNT;
    if (bucket_count <= 1u || FLORA_FULL_UPDATE_SECONDS <= 0.0f) {
        return raw_time;
    }

    float full_cycle = FLORA_FULL_UPDATE_SECONDS;
    float step       = full_cycle / float(bucket_count);

    // global scheduler tick index
    float s = floor(raw_time / step);

    uint bucket_id = instance_seed % bucket_count;

    float last_step_index;
    if (s < float(bucket_id)) {
        // bucket has not received an update yet; pin to t = 0
        last_step_index = 0.0;
    } else {
        // last scheduler tick where this bucket was active: n = bucket_id + k * bucket_count
        float k = floor((s - float(bucket_id)) / float(bucket_count));
        last_step_index = float(bucket_id) + k * float(bucket_count);
    }

    return last_step_index * step;
}

void main() {
    ivec3 vox_local_pos;
    uvec3 vert_offset_in_vox;
    ivec3 gradient_origin;
    uint max_length;
    unpack_vertex_data(vox_local_pos, vert_offset_in_vox, gradient_origin, max_length,
                       in_packed_data);

    float base_gradient  = compute_gradient(vox_local_pos, gradient_origin, max_length);
    float color_gradient = base_gradient;
    float wind_gradient  = base_gradient;

    uint instance_ty   = decode_instance_ty(in_instance_ty_seed);
    uint instance_seed = decode_instance_seed(in_instance_ty_seed);
    bool is_grass      = instance_ty == FLORA_SPECIES_GRASS;
    uint grass_height_voxels =
        is_grass ? sample_grass_height(instance_seed) : grass_max_height_voxels;
    float grass_height_voxels_f = float(grass_height_voxels);
    bool should_trim_voxel      = false;

    if (is_grass) {
        float growth_factor         = grass_growth_factor(in_instance_growth_start_tick);
        float grown_height_voxels_f = floor(grass_height_voxels_f * growth_factor + 0.001);
        should_trim_voxel           = float(vox_local_pos.y) >= grown_height_voxels_f;
    }

    vec3 instance_pos = in_instance_pos * scaling_factor;

    float wind_time = flora_bucketed_time(pc.time, instance_seed);
    vec3 wind_vec    = get_wind(instance_pos, wind_time);
    vec3 wind_offset = wind_vec * wind_gradient * wind_gradient;
    vec3 anchor_pos  = (vec3(vox_local_pos) + wind_offset) * scaling_factor + instance_pos;
    vec3 voxel_pos   = anchor_pos + vec3(0.5) * scaling_factor;
    vec3 vert_pos    = anchor_pos + vec3(vert_offset_in_vox) * scaling_factor;

    if (should_trim_voxel) {
        voxel_pos = anchor_pos;
        vert_pos  = anchor_pos;
    }

    float shadow_weight =
        get_shadow_weight_vsm(shadow_camera_info.view_proj_mat, vec4(voxel_pos, 1.0));
    shadow_weight *= get_shadow_weight(vox_local_pos);

    // Apply depth offset to prevent z-fighting between instances
    gl_Position =
        apply_depth_offset(vert_pos, in_instance_pos, camera_info.view_mat, camera_info.proj_mat);

    vec3 base_color_linear;
    if (is_grass) {
        vec3 grass_bottom_color_linear;
        vec3 grass_tip_color_linear;
        sample_grass_band_gradient(vec2(float(in_instance_pos.x), float(in_instance_pos.z)),
                                   grass_bottom_color_linear, grass_tip_color_linear);
        base_color_linear = mix(grass_bottom_color_linear, grass_tip_color_linear, color_gradient);
    } else {
        uint palette_seed        = combine_color_seed(instance_seed);
        vec3 bottom_color_linear = srgb_to_linear(pc.bottom_color);
        vec3 tip_color_linear    = sample_tip_palette(instance_ty, palette_seed, pc.tip_color);
        vec3 interpolated_color  = mix(bottom_color_linear, tip_color_linear, color_gradient);
        vec3 instance_color_variation =
            signed_unit_noise(float(instance_seed)) * gui_input.flora_instance_hsv_offset_max;
        vec3 voxel_color_variation =
            signed_unit_noise(vec4(vec3(vox_local_pos), float(instance_seed))) *
            gui_input.flora_voxel_hsv_offset_max;
        vec3 total_color_variation = instance_color_variation + voxel_color_variation;
        base_color_linear          = apply_hsv_offset(interpolated_color, total_color_variation);
    }

    float sun_luminance = sun_luminance_from_dir(sun_info.sun_dir, sun_info.sun_luminance);
    vec3 sun_light      = sun_info.sun_color * sun_luminance;
    vert_color          = base_color_linear * (sun_light * shadow_weight + shading_info.ambient_light);
}
