pub(crate) fn calculate_sun_position(time_of_day: f32, latitude: f32, season: f32) -> (f32, f32) {
    use std::f32::consts::PI;

    // time_of_day: 0.0 = midnight, 0.5 = noon, 1.0 = midnight
    // latitude: -1.0 = south pole, 0.0 = equator, 1.0 = north pole
    // season: 0.0 = winter solstice, 0.5 = summer solstice

    let hour_angle = (time_of_day - 0.5) * 2.0 * PI;
    let seasonal_angle = season * 2.0 * PI;
    let declination = -23.44_f32.to_radians() * seasonal_angle.cos();

    let elevation = (declination.sin() * (latitude * PI * 0.5).sin()
        + declination.cos() * (latitude * PI * 0.5).cos() * hour_angle.cos())
    .asin();

    let azimuth = if hour_angle.cos() == 0.0 {
        if hour_angle > 0.0 {
            PI
        } else {
            0.0
        }
    } else {
        (declination.sin() * (latitude * PI * 0.5).cos()
            - declination.cos() * (latitude * PI * 0.5).sin() * hour_angle.cos())
        .atan2(hour_angle.sin())
    };

    let sun_altitude = (elevation / (PI * 0.5)).clamp(-1.0, 1.0);
    let sun_azimuth = ((azimuth + PI) / (2.0 * PI)) % 1.0;
    (sun_altitude, sun_azimuth)
}
