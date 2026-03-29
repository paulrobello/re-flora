#ifndef SKYLIGHT_GLSL
#define SKYLIGHT_GLSL

#include "../include/core/color.glsl"

struct SkyColors {
    vec3 top_color;
    vec3 bottom_color;
};

// keyframe structure for time-of-day transitions
struct TimeOfDayKeyframe {
    float sun_altitude;
    vec3 top_color;
    vec3 bottom_color;
};

// keyframe structure for view altitude transitions
struct ViewAltitudeKeyframe {
    float view_altitude;
    float blend_factor; // 0.0 = bottom color, 1.0 = top color
};

// time-of-day keyframes - Colors informed by Preetham/Hosek-Wilkie sky model reference values.
// Zenith (top) is deep Rayleigh-scattering blue; horizon (bottom) is pale/desaturated due to
// longer atmospheric path length. Sunrise/sunset transitions use warm-to-cool gradients matching
// real-world aerosol scattering behavior.
const int TIME_KEYFRAME_COUNT                               = 11;
const TimeOfDayKeyframe TIME_KEYFRAMES[TIME_KEYFRAME_COUNT] = {
    // 1. Deep Night: Almost pitch black with a hint of deep, cold blue.
    {-0.4, vec3(4.0, 5.0, 10.0) / 255.0, vec3(8.0, 10.0, 20.0) / 255.0},

    // 2. Pre-Dawn / Post-Dusk: First/last hint of light; warm purple-pink on the horizon,
    // zenith still very dark blue-grey. Sun is deep below the horizon.
    {-0.25, vec3(18.0, 22.0, 48.0) / 255.0, vec3(55.0, 38.0, 58.0) / 255.0},

    // 3. Blue Hour start: Rich indigo zenith, violet-blue horizon. Sun is ~-6 deg (nautical
    // twilight). This keyframe is placed well before sunrise so the blue hour has real duration.
    {-0.15, vec3(16.0, 26.0, 70.0) / 255.0, vec3(32.0, 28.0, 72.0) / 255.0},

    // 4. Blue Hour peak: The whole sky is a vivid, cool blue. Horizon picks up a faint warm tint
    // signalling the approaching sun — but blue still dominates.
    {-0.07, vec3(22.0, 42.0, 100.0) / 255.0, vec3(50.0, 52.0, 110.0) / 255.0},

    // 5. Dawn/Dusk Glow: Blue hour fades; deep blue above, warm crimson-magenta at the horizon.
    // Sun is just below the horizon (~-3 deg civil twilight).
    {-0.03, vec3(28.0, 48.0, 105.0) / 255.0, vec3(195.0, 60.0, 85.0) / 255.0},

    // 6. Sunrise/Sunset: Lighter steel-blue zenith meeting a fiery orange-amber horizon.
    {0.0, vec3(70.0, 95.0, 175.0) / 255.0, vec3(255.0, 115.0, 45.0) / 255.0},

    // 7. Golden Hour: Warm amber-orange horizon; zenith transitions to a slightly warmer blue
    // (less harsh than pure blue) to ease the gradient.
    {0.1, vec3(120.0, 158.0, 248.0) / 255.0, vec3(255.0, 185.0, 95.0) / 255.0},

    // 7. Morning: Crisp blue zenith; horizon is a soft, saturated mid-blue — no white.
    {0.3, vec3(58.0, 130.0, 220.0) / 255.0, vec3(100.0, 160.0, 220.0) / 255.0},

    // 8. Mid-day: Deep Rayleigh blue at zenith; horizon is a clear mid-blue — retains saturation
    // so the sky reads as blue from edge to edge, not hazy-white.
    {0.5, vec3(38.0, 108.0, 210.0) / 255.0, vec3(85.0, 145.0, 215.0) / 255.0},

    // 9. High Noon: Deepest Rayleigh blue at zenith; horizon stays solidly blue.
    {0.8, vec3(28.0, 92.0, 195.0) / 255.0, vec3(72.0, 132.0, 205.0) / 255.0},

    // 10. Late Afternoon: Zenith warms slightly toward afternoon; horizon follows.
    {1.0, vec3(42.0, 105.0, 210.0) / 255.0, vec3(88.0, 148.0, 215.0) / 255.0}};

// view altitude keyframes - The horizon line blends mostly toward the sky (top) color so it
// reads as clear blue, not washed-out haze. A gentle gradient from ~0 to ~0.3 altitude gives
// natural depth without a visible white band.
const int VIEW_KEYFRAME_COUNT                                  = 7;
const ViewAltitudeKeyframe VIEW_KEYFRAMES[VIEW_KEYFRAME_COUNT] = {
    // looking straight down - full bottom (ground) color
    {-1.0, 0.0},
    // below horizon - very little sky color
    {-0.15, 0.03},
    // at the horizon line - mostly sky-tinted; bottom color only faintly present
    {0.0, 0.55},
    // just above horizon - quickly approaches zenith color
    {0.08, 0.72},
    // low sky - strongly zenith-dominated
    {0.2, 0.86},
    // mid sky - almost full zenith color
    {0.4, 0.96},
    // zenith - full top (deep Rayleigh-blue) color
    {1.0, 1.0}};

// interpolate between time-of-day keyframes
SkyColors interpolate_time_keyframes(float sun_altitude) {
    SkyColors result;

    // handle edge cases
    if (sun_altitude <= TIME_KEYFRAMES[0].sun_altitude) {
        result.top_color    = srgb_to_linear(TIME_KEYFRAMES[0].top_color);
        result.bottom_color = srgb_to_linear(TIME_KEYFRAMES[0].bottom_color);
        return result;
    }

    if (sun_altitude >= TIME_KEYFRAMES[TIME_KEYFRAME_COUNT - 1].sun_altitude) {
        result.top_color    = srgb_to_linear(TIME_KEYFRAMES[TIME_KEYFRAME_COUNT - 1].top_color);
        result.bottom_color = srgb_to_linear(TIME_KEYFRAMES[TIME_KEYFRAME_COUNT - 1].bottom_color);
        return result;
    }

    // find the two keyframes to interpolate between
    for (int i = 0; i < TIME_KEYFRAME_COUNT - 1; i++) {
        if (sun_altitude >= TIME_KEYFRAMES[i].sun_altitude &&
            sun_altitude < TIME_KEYFRAMES[i + 1].sun_altitude) {
            float t = (sun_altitude - TIME_KEYFRAMES[i].sun_altitude) /
                      (TIME_KEYFRAMES[i + 1].sun_altitude - TIME_KEYFRAMES[i].sun_altitude);

            result.top_color = srgb_to_linear(
                mix(TIME_KEYFRAMES[i].top_color, TIME_KEYFRAMES[i + 1].top_color, t));
            result.bottom_color = srgb_to_linear(
                mix(TIME_KEYFRAMES[i].bottom_color, TIME_KEYFRAMES[i + 1].bottom_color, t));
            return result;
        }
    }

    // fallback (shouldn't reach here)
    result.top_color    = srgb_to_linear(TIME_KEYFRAMES[0].top_color);
    result.bottom_color = srgb_to_linear(TIME_KEYFRAMES[0].bottom_color);
    return result;
}

// interpolate view altitude blend factor
float interpolate_view_altitude(float view_altitude) {
    // handle edge cases
    if (view_altitude <= VIEW_KEYFRAMES[0].view_altitude) {
        return VIEW_KEYFRAMES[0].blend_factor;
    }

    if (view_altitude >= VIEW_KEYFRAMES[VIEW_KEYFRAME_COUNT - 1].view_altitude) {
        return VIEW_KEYFRAMES[VIEW_KEYFRAME_COUNT - 1].blend_factor;
    }

    // find the two keyframes to interpolate between
    for (int i = 0; i < VIEW_KEYFRAME_COUNT - 1; i++) {
        if (view_altitude >= VIEW_KEYFRAMES[i].view_altitude &&
            view_altitude < VIEW_KEYFRAMES[i + 1].view_altitude) {
            float t = (view_altitude - VIEW_KEYFRAMES[i].view_altitude) /
                      (VIEW_KEYFRAMES[i + 1].view_altitude - VIEW_KEYFRAMES[i].view_altitude);

            return mix(VIEW_KEYFRAMES[i].blend_factor, VIEW_KEYFRAMES[i + 1].blend_factor, t);
        }
    }

    // fallback (shouldn't reach here)
    return VIEW_KEYFRAMES[0].blend_factor;
}

// sun altitude ranges from -1 to 1
SkyColors get_sky_color_by_sun_altitude(float sun_altitude) {
    return interpolate_time_keyframes(sun_altitude);
}

// Henyey-Greenstein phase function for Mie scattering.
// g controls anisotropy: ~0.76 matches real aerosol forward scattering (Preetham 1999, p.4).
// cos_theta is the cosine of the angle between view and sun direction.
float hg_phase(float cos_theta, float g) {
    float g2 = g * g;
    return (1.0 - g2) / (4.0 * 3.14159265 * pow(1.0 + g2 - 2.0 * g * cos_theta, 1.5));
}

vec3 get_sky_color(vec3 view_dir, vec3 sun_dir) {
    // altitude range now matches sun altitude range (-1 to 1)
    float altitude     = view_dir.y;
    float sun_altitude = sun_dir.y;

    SkyColors sky_colors = get_sky_color_by_sun_altitude(sun_altitude);

    // use keyframe-based view altitude interpolation
    float blend_factor  = interpolate_view_altitude(altitude);
    blend_factor        = smoothstep(0.0, 1.0, blend_factor);
    vec3 base_sky_color = mix(sky_colors.bottom_color, sky_colors.top_color, blend_factor);

    // Henyey-Greenstein Mie scattering halo around the sun.
    // g = 0.76 is the industry-standard aerosol anisotropy factor (Preetham 1999).
    // When the sun is near the horizon, aerosols are more concentrated, so we
    // increase g slightly to produce a tighter, brighter forward-scattering lobe.
    float cos_theta   = dot(view_dir, sun_dir);
    float g           = mix(0.76, 0.82, clamp(1.0 - abs(sun_altitude), 0.0, 1.0));
    float halo_phase  = hg_phase(cos_theta, g);

    // normalize: HG peak at cos_theta=1 is (1-g^2)/(4*pi*(1+g^2-2g)^1.5) = (1-g^2)/(4*pi*(1-g)^3)
    float g2          = g * g;
    float hg_peak     = (1.0 - g2) / (4.0 * 3.14159265 * pow(1.0 - g, 3.0));
    float halo_norm   = halo_phase / hg_peak; // 0..1

    // reduce halo strength at high sun altitude (less haze, cleaner sky)
    float halo_strength = halo_norm * mix(0.35, 0.18, clamp(sun_altitude * 2.0, 0.0, 1.0));

    // derive halo tint from sky color at the sun's elevation
    float sun_blend_factor = interpolate_view_altitude(sun_altitude);
    sun_blend_factor       = smoothstep(0.0, 1.0, sun_blend_factor);
    vec3 halo_color        = mix(sky_colors.bottom_color, sky_colors.top_color, sun_blend_factor);

    // blend halo with base sky color
    vec3 sky_color = mix(base_sky_color, halo_color, clamp(halo_strength, 0.0, 1.0));

    return sky_color;
}

vec3 get_sky_color_with_sun(vec3 view_dir, vec3 sun_dir, vec3 sun_color, float sun_luminance,
                            float sun_size) {
    vec3 sky_color_linear = get_sky_color(view_dir, sun_dir);
    float sun_dist        = 1.0 - dot(view_dir, sun_dir);
    sun_dist /= sun_size;

    float sun = 0.05 / max(sun_dist, 0.001) + 0.02;

    vec3 sun_contribution = vec3(sun / 0.477, sun + 0.5, sun + 0.8);

    // scale by sun luminance and apply size factor
    sun_contribution *= sun_luminance * 0.2;

    // blend the sun contribution with the base sky color
    vec3 luminance_sun_color = sun_color * sun_contribution;

    // use a falloff based on distance for smooth blending
    float sun_blend_factor = clamp(sun * 0.1, 0.0, 1.0);

    return mix(sky_color_linear, luminance_sun_color, sun_blend_factor);
}

#endif // SKYLIGHT_GLSL
