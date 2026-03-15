// ocean rendering utilities: ocean plane shading and environment selection
#ifndef OCEAN_GLSL
#define OCEAN_GLSL

vec3 compute_ocean_color(vec3 view_dir, out bool hit_ocean) {
    const float water_level = 0.0;

    vec3 ro = camera_info.pos.xyz;
    vec3 rd = normalize(view_dir);

    float denom = rd.y;
    if (denom >= -0.001) {
        hit_ocean = false;
        return vec3(0.0);
    }

    float t = (water_level - ro.y) / denom;
    if (t <= 0.0) {
        hit_ocean = false;
        return vec3(0.0);
    }

    vec3 p = ro + rd * t;

    float time = float(env_info.frame_serial_idx) * 0.016 * gui_input.ocean_time_multiplier;

    // perlin-based normal perturbation for reflections (flat plane intersection kept intact)
    const int OCEAN_NOISE_SEED      = 9041;
    const float OCEAN_TIME_SCROLL_X = 0.15;
    const float OCEAN_TIME_SCROLL_Z = 0.21;

    fnl_state state = fnlCreateState(OCEAN_NOISE_SEED);
    state.noise_type   = FNL_NOISE_PERLIN;
    state.fractal_type = FNL_FRACTAL_FBM;
    state.octaves      = 4;
    state.frequency    = gui_input.ocean_noise_frequency;

    vec2 sample_pos = p.xz;
    float n_x = fnlGetNoise2D(state, sample_pos.x + time * OCEAN_TIME_SCROLL_X, sample_pos.y);
    float n_z = fnlGetNoise2D(state, sample_pos.x, sample_pos.y + time * OCEAN_TIME_SCROLL_Z);

    vec2 n = vec2(n_x, n_z); // [-1, 1]
    vec3 wave_normal = normalize(
        vec3(n.x * gui_input.ocean_normal_amplitude, 1.0, n.y * gui_input.ocean_normal_amplitude));

    // colors come from GUI in sRGB
    vec3 deep       = srgb_to_linear(gui_input.ocean_deep_color);
    vec3 water_base = deep;

    vec3 up = vec3(0.0, 1.0, 0.0);
    float facing = clamp(dot(wave_normal, -rd), 0.0, 1.0);

    // slightly stronger fresnel to emphasize reflection at grazing angles
    float fresnel = pow(1.0 - facing, 2.0);

    // mirror view direction across the ideal water plane for environment reflection
    vec3 refl_dir_env = normalize(vec3(rd.x, -rd.y, rd.z));
    vec3 sky_ref      = compute_sky_with_sun_and_stars(refl_dir_env);

    // boost reflection intensity a bit for a clearer environment reflection
    float reflection_weight = clamp(fresnel * 1.4, 0.0, 1.0);
    vec3 water_color        = mix(water_base, sky_ref, reflection_weight);

    // explicit sun glint: specular highlight based on sun direction
    vec3 L = normalize(sun_info.sun_dir);
    vec3 V = normalize(-rd);
    vec3 R = reflect(-L, wave_normal);
    float spec = max(dot(R, V), 0.0);
    spec = pow(spec, 64.0);

    float sun_visibility = sun_luminance_from_dir(sun_info.sun_dir, 1.0);
    vec3 sun_glint = sun_info.sun_color * sun_info.sun_display_luminance * (0.4 * spec * sun_visibility);

    water_color += sun_glint;

    hit_ocean = true;
    return water_color;
}

vec3 compute_environment_color(vec3 view_dir) {
    bool hit_ocean;
    vec3 ocean_color = compute_ocean_color(view_dir, hit_ocean);
    if (hit_ocean) {
        return ocean_color;
    }
    return compute_sky_with_sun_and_stars(view_dir);
}

#endif // OCEAN_GLSL
