float sun_luminance_from_dir(vec3 sun_dir, float base_luminance) {
    float day_factor = smoothstep(-0.05, 0.05, sun_dir.y);
    return base_luminance * day_factor;
}
