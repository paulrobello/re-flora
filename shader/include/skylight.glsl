#ifndef SKYLIGHT_GLSL
#define SKYLIGHT_GLSL

#include "../include/core/color.glsl"

struct SkyColors {
    vec3 top_color;
    vec3 bottom_color;
};

// -----------------------------------------------------------------------
// Metal/MoltenVK optimized sky color computation.
// All keyframe colors are pre-linearized (sRGB→linear at compile time).
// No const struct arrays, no loops — just flat if-else chains.
// This avoids the Metal constant-array performance penalty (~26ms→~1ms).
// -----------------------------------------------------------------------

// Pre-linearized time-of-day keyframe colors.
// sun_alt thresholds: -0.40, -0.25, -0.15, -0.07, -0.03, 0.00, 0.10, 0.30, 0.50, 0.80, 1.00
SkyColors interpolate_time_keyframes(float s) {
    // Pre-linearized keyframe colors (srgb_to_linear applied at compile time)
    // Deep Night (s <= -0.40)
    const vec3 T0 = vec3(0.001214, 0.001518, 0.003035);
    const vec3 B0 = vec3(0.002428, 0.003035, 0.006995);
    // Pre-Dawn (s = -0.25)
    const vec3 T1 = vec3(0.006049, 0.008023, 0.029557);
    const vec3 B1 = vec3(0.038204, 0.019382, 0.042311);
    // Blue Hour start (s = -0.15)
    const vec3 T2 = vec3(0.005182, 0.010330, 0.061246);
    const vec3 B2 = vec3(0.014444, 0.011612, 0.064803);
    // Blue Hour peak (s = -0.07)
    const vec3 T3 = vec3(0.008023, 0.023153, 0.127438);
    const vec3 B3 = vec3(0.031896, 0.034340, 0.155926);
    // Dawn/Dusk Glow (s = -0.03)
    const vec3 T4 = vec3(0.011612, 0.029557, 0.141263);
    const vec3 B4 = vec3(0.545724, 0.045186, 0.090842);
    // Sunrise/Sunset (s = 0.00)
    const vec3 T5 = vec3(0.061246, 0.114435, 0.428690);
    const vec3 B5 = vec3(1.000000, 0.171441, 0.026241);
    // Golden Hour (s = 0.10)
    const vec3 T6 = vec3(0.187821, 0.341914, 0.938686);
    const vec3 B6 = vec3(1.000000, 0.485150, 0.114435);
    // Morning (s = 0.30)
    const vec3 T7 = vec3(0.042311, 0.223228, 0.715694);
    const vec3 B7 = vec3(0.127438, 0.351533, 0.715694);
    // Mid-day (s = 0.50)
    const vec3 T8 = vec3(0.019382, 0.149960, 0.644480);
    const vec3 B8 = vec3(0.090842, 0.283149, 0.679542);
    // High Noon (s = 0.80)
    const vec3 T9 = vec3(0.011612, 0.107023, 0.545724);
    const vec3 B9 = vec3(0.064803, 0.230740, 0.610496);
    // Late Afternoon (s = 1.00)
    const vec3 T10 = vec3(0.023153, 0.141263, 0.644480);
    const vec3 B10 = vec3(0.097587, 0.296138, 0.679542);

    // Unrolled if-else chain — Metal handles this far better than loop-over-array
    SkyColors r;
    if (s <= -0.40) {
        r.top_color = T0; r.bottom_color = B0;
    } else if (s < -0.25) {
        float t = (s + 0.40) / 0.15;
        r.top_color = mix(T0, T1, t); r.bottom_color = mix(B0, B1, t);
    } else if (s < -0.15) {
        float t = (s + 0.25) / 0.10;
        r.top_color = mix(T1, T2, t); r.bottom_color = mix(B1, B2, t);
    } else if (s < -0.07) {
        float t = (s + 0.15) / 0.08;
        r.top_color = mix(T2, T3, t); r.bottom_color = mix(B2, B3, t);
    } else if (s < -0.03) {
        float t = (s + 0.07) / 0.04;
        r.top_color = mix(T3, T4, t); r.bottom_color = mix(B3, B4, t);
    } else if (s < 0.0) {
        float t = (s + 0.03) / 0.03;
        r.top_color = mix(T4, T5, t); r.bottom_color = mix(B4, B5, t);
    } else if (s < 0.1) {
        float t = s / 0.1;
        r.top_color = mix(T5, T6, t); r.bottom_color = mix(B5, B6, t);
    } else if (s < 0.3) {
        float t = (s - 0.1) / 0.2;
        r.top_color = mix(T6, T7, t); r.bottom_color = mix(B6, B7, t);
    } else if (s < 0.5) {
        float t = (s - 0.3) / 0.2;
        r.top_color = mix(T7, T8, t); r.bottom_color = mix(B7, B8, t);
    } else if (s < 0.8) {
        float t = (s - 0.5) / 0.3;
        r.top_color = mix(T8, T9, t); r.bottom_color = mix(B8, B9, t);
    } else if (s < 1.0) {
        float t = (s - 0.8) / 0.2;
        r.top_color = mix(T9, T10, t); r.bottom_color = mix(B9, B10, t);
    } else {
        r.top_color = T10; r.bottom_color = B10;
    }
    return r;
}

// View altitude blend factor — piecewise linear approximation without arrays.
// Original keyframes: (-1.0,0.0) (-0.15,0.03) (0.0,0.55) (0.08,0.72) (0.2,0.86) (0.4,0.96) (1.0,1.0)
float interpolate_view_altitude(float a) {
    if (a <= -1.0) return 0.0;
    if (a < -0.15) return mix(0.0, 0.03, (a + 1.0) / 0.85);
    if (a < 0.0)   return mix(0.03, 0.55, (a + 0.15) / 0.15);
    if (a < 0.08)  return mix(0.55, 0.72, a / 0.08);
    if (a < 0.2)   return mix(0.72, 0.86, (a - 0.08) / 0.12);
    if (a < 0.4)   return mix(0.86, 0.96, (a - 0.2) / 0.2);
    if (a < 1.0)   return mix(0.96, 1.0, (a - 0.4) / 0.6);
    return 1.0;
}

// Henyey-Greenstein phase function for Mie scattering.
// Uses x * sqrt(x) instead of pow(x, 1.5) for Metal performance.
float hg_phase(float cos_theta, float g) {
    float g2 = g * g;
    float base = 1.0 + g2 - 2.0 * g * cos_theta;
    return (1.0 - g2) / (4.0 * 3.14159265 * base * sqrt(base));
}

vec3 get_sky_color(vec3 view_dir, vec3 sun_dir) {
    float altitude     = view_dir.y;
    float sun_altitude = sun_dir.y;

    SkyColors sky_colors = interpolate_time_keyframes(sun_altitude);

    float blend_factor  = interpolate_view_altitude(altitude);
    blend_factor        = smoothstep(0.0, 1.0, blend_factor);
    vec3 base_sky_color = mix(sky_colors.bottom_color, sky_colors.top_color, blend_factor);

    // Henyey-Greenstein Mie scattering halo around the sun
    float cos_theta   = dot(view_dir, sun_dir);
    float g           = mix(0.76, 0.82, clamp(1.0 - abs(sun_altitude), 0.0, 1.0));
    float halo_phase  = hg_phase(cos_theta, g);

    // Normalize: HG peak at cos_theta=1 — use x*x*x instead of pow(x,3)
    float g2          = g * g;
    float one_minus_g = 1.0 - g;
    float hg_peak     = (1.0 - g2) / (4.0 * 3.14159265 * one_minus_g * one_minus_g * one_minus_g);
    float halo_norm   = halo_phase / hg_peak;

    float halo_strength = halo_norm * mix(0.35, 0.18, clamp(sun_altitude * 2.0, 0.0, 1.0));

    float sun_blend_factor = interpolate_view_altitude(sun_altitude);
    sun_blend_factor       = smoothstep(0.0, 1.0, sun_blend_factor);
    vec3 halo_color        = mix(sky_colors.bottom_color, sky_colors.top_color, sun_blend_factor);

    return mix(base_sky_color, halo_color, clamp(halo_strength, 0.0, 1.0));
}

vec3 get_sky_color_with_sun(vec3 view_dir, vec3 sun_dir, vec3 sun_color, float sun_luminance,
                            float sun_size) {
    vec3 sky_color_linear = get_sky_color(view_dir, sun_dir);
    float sun_dist        = 1.0 - dot(view_dir, sun_dir);
    sun_dist /= sun_size;

    float sun = 0.05 / max(sun_dist, 0.001) + 0.02;

    vec3 sun_contribution = vec3(sun / 0.477, sun + 0.5, sun + 0.8);
    sun_contribution *= sun_luminance * 0.2;

    vec3 luminance_sun_color = sun_color * sun_contribution;
    float sun_blend_factor = clamp(sun * 0.1, 0.0, 1.0);

    return mix(sky_color_linear, luminance_sun_color, sun_blend_factor);
}

#endif // SKYLIGHT_GLSL
