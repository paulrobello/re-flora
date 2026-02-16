#version 450

layout(location = 0) in vec4 vert_color;
layout(location = 1) in vec2 vert_uv;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 6) uniform sampler2D particle_lod_tex;

void main() {
    vec4 texel = texture(particle_lod_tex, vert_uv);
    float alpha = vert_color.a * texel.a;
    vec3 rgb = vert_color.rgb * texel.rgb * alpha;
    out_color = vec4(rgb, alpha);
}
