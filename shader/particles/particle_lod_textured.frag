#version 450

layout(location = 0) in vec4 vert_color;
layout(location = 1) in vec2 vert_uv;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 6) uniform sampler2D particle_lod_tex;

void main() {
    vec4 texel = texture(particle_lod_tex, vert_uv);
    float alpha = vert_color.a * texel.a;
    float alpha_mask = step(0.5, alpha);

    // Discard-free alpha test: transparent pixels output no color and are pushed to far depth.
    // This avoids SPIR-V demote/discard capability requirements on unsupported devices.
    float masked_alpha = alpha * alpha_mask;
    vec3 rgb = vert_color.rgb * texel.rgb * masked_alpha;
    out_color = vec4(rgb, masked_alpha);
    gl_FragDepth = mix(1.0, gl_FragCoord.z, alpha_mask);
}
