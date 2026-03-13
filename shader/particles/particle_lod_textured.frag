#version 450

layout(location = 0) in vec4 vert_color;
layout(location = 1) in vec2 vert_uv;
layout(location = 2) flat in uint vert_tex_index;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 6) uniform sampler2DArray particle_lod_tex_lut;

void main() {
    vec4 texel       = texture(particle_lod_tex_lut, vec3(vert_uv, float(vert_tex_index)));
    float tex_alpha  = texel.a;
    float alpha_mask = step(0.5, tex_alpha);

    // Discard-free alpha test: transparent pixels output no color and are pushed to far depth.
    // This avoids SPIR-V demote/discard capability requirements on unsupported devices.
    float base_alpha   = tex_alpha * alpha_mask;
    float fade         = vert_color.a;
    float masked_alpha = base_alpha * fade;

    // keep color intensity driven by lighting/texture and texture coverage,
    // and let fade only influence the final alpha so we don't "double fade"
    // the rgb during blending.
    vec3 rgb           = vert_color.rgb * texel.rgb * base_alpha;
    out_color          = vec4(rgb, masked_alpha);
    gl_FragDepth       = mix(1.0, gl_FragCoord.z, alpha_mask);
}
