#ifndef WIND_VOLUME_GLSL
#define WIND_VOLUME_GLSL

vec3 sample_wind_volume(vec3 world_pos) {
    vec3 wind_uv     = clamp(world_pos / wind_volume_info.world_chunk_extent, vec3(0.0), vec3(1.0));
    vec2 wind_planar = texture(wind_volume_tex, wind_uv).xy;
    return vec3(wind_planar.x, 0.0, wind_planar.y);
}

#endif // WIND_VOLUME_GLSL
