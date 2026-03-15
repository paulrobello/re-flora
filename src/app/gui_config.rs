/// GUI Adjustables Configuration
///
/// This file loads GUI parameters from config/gui.toml.
/// The config file is the single source of truth.
use crate::app::gui_config_loader::GuiConfigLoader;
use crate::app::gui_config_model::{GuiConfigFile, GuiParamKind, GuiParamValue};
use crate::declare_gui_adjustables;
use egui::Color32;

fn parse_color(hex: &str) -> Color32 {
    let hex = hex.trim_start_matches('#');
    let (r, g, b, a) = match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).expect("invalid red");
            let g = u8::from_str_radix(&hex[2..4], 16).expect("invalid green");
            let b = u8::from_str_radix(&hex[4..6], 16).expect("invalid blue");
            (r, g, b, 255)
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).expect("invalid red");
            let g = u8::from_str_radix(&hex[2..4], 16).expect("invalid green");
            let b = u8::from_str_radix(&hex[4..6], 16).expect("invalid blue");
            let a = u8::from_str_radix(&hex[6..8], 16).expect("invalid alpha");
            (r, g, b, a)
        }
        _ => panic!(
            "Invalid color format: #{}. Expected #RRGGBB or #RRGGBBAA",
            hex
        ),
    };
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

fn color_to_hex(color: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
}

fn load_from_config(config: &GuiConfigFile) -> GuiAdjustables {
    let mut params: std::collections::HashMap<String, Box<dyn std::any::Any>> =
        std::collections::HashMap::new();

    for section in &config.section {
        for param in &section.param {
            match &param.kind {
                GuiParamKind::Float => {
                    if let Some((value, min, max)) = param.value.get_float() {
                        let min = min.unwrap_or(0.0);
                        let max = max.unwrap_or(1.0);
                        params.insert(
                            param.id.clone(),
                            Box::new(crate::gui_adjustables::FloatParam::new(value, min..=max)),
                        );
                    }
                }
                GuiParamKind::Int => {
                    if let Some((value, min, max)) = param.value.get_int() {
                        let min = min.unwrap_or(0);
                        let max = max.unwrap_or(100);
                        params.insert(
                            param.id.clone(),
                            Box::new(crate::gui_adjustables::IntParam::new(value, min..=max)),
                        );
                    }
                }
                GuiParamKind::Uint => {
                    if let Some((value, min, max)) = param.value.get_uint() {
                        let min = min.unwrap_or(0);
                        let max = max.unwrap_or(100);
                        params.insert(
                            param.id.clone(),
                            Box::new(crate::gui_adjustables::UintParam::new(value, min..=max)),
                        );
                    }
                }
                GuiParamKind::Bool => {
                    if let Some(value) = param.value.get_bool() {
                        params.insert(
                            param.id.clone(),
                            Box::new(crate::gui_adjustables::BoolParam::new(value)),
                        );
                    }
                }
                GuiParamKind::Color => {
                    if let Some(hex) = param.value.get_color() {
                        params.insert(
                            param.id.clone(),
                            Box::new(crate::gui_adjustables::ColorParam::new(parse_color(&hex))),
                        );
                    }
                }
            }
        }
    }

    macro_rules! get_param {
        ($id:literal, $type:ty) => {
            *params
                .remove($id)
                .unwrap_or_else(|| panic!("Missing parameter: {}", $id))
                .downcast::<$type>()
                .unwrap()
        };
    }

    if !params.is_empty() {
        let unwired: Vec<_> = params.keys().cloned().collect();
        log::warn!(
            "GUI config has params that are not wired into GuiAdjustables: {:?}",
            unwired
        );
    }

    GuiAdjustables {
            debug_float: get_param!("debug_float", crate::gui_adjustables::FloatParam),
            debug_uint: get_param!("debug_uint", crate::gui_adjustables::UintParam),
            lod_distance: get_param!("lod_distance", crate::gui_adjustables::FloatParam),
            debug_bool: get_param!("debug_bool", crate::gui_adjustables::BoolParam),

            sun_size: get_param!("sun_size", crate::gui_adjustables::FloatParam),
            sun_color: get_param!("sun_color", crate::gui_adjustables::ColorParam),
            sun_luminance: get_param!("sun_luminance", crate::gui_adjustables::FloatParam),
            sun_display_luminance: get_param!(
                "sun_display_luminance",
                crate::gui_adjustables::FloatParam
            ),
            ambient_light: get_param!("ambient_light", crate::gui_adjustables::ColorParam),
            auto_daynight_cycle: get_param!(
                "auto_daynight_cycle",
                crate::gui_adjustables::BoolParam
            ),
            time_of_day: get_param!("time_of_day", crate::gui_adjustables::FloatParam),
            latitude: get_param!("latitude", crate::gui_adjustables::FloatParam),
            season: get_param!("season", crate::gui_adjustables::FloatParam),
            day_cycle_minutes: get_param!("day_cycle_minutes", crate::gui_adjustables::FloatParam),

            starlight_iterations: get_param!(
                "starlight_iterations",
                crate::gui_adjustables::IntParam
            ),
            starlight_formuparam: get_param!(
                "starlight_formuparam",
                crate::gui_adjustables::FloatParam
            ),
            starlight_volsteps: get_param!("starlight_volsteps", crate::gui_adjustables::IntParam),
            starlight_stepsize: get_param!(
                "starlight_stepsize",
                crate::gui_adjustables::FloatParam
            ),
            starlight_zoom: get_param!("starlight_zoom", crate::gui_adjustables::FloatParam),
            starlight_tile: get_param!("starlight_tile", crate::gui_adjustables::FloatParam),
            starlight_speed: get_param!("starlight_speed", crate::gui_adjustables::FloatParam),
            starlight_brightness: get_param!(
                "starlight_brightness",
                crate::gui_adjustables::FloatParam
            ),
            starlight_darkmatter: get_param!(
                "starlight_darkmatter",
                crate::gui_adjustables::FloatParam
            ),
            starlight_distfading: get_param!(
                "starlight_distfading",
                crate::gui_adjustables::FloatParam
            ),
            starlight_saturation: get_param!(
                "starlight_saturation",
                crate::gui_adjustables::FloatParam
            ),

            temporal_position_phi: get_param!(
                "temporal_position_phi",
                crate::gui_adjustables::FloatParam
            ),
            temporal_alpha: get_param!("temporal_alpha", crate::gui_adjustables::FloatParam),

            god_ray_max_depth: get_param!("god_ray_max_depth", crate::gui_adjustables::FloatParam),
            god_ray_max_checks: get_param!("god_ray_max_checks", crate::gui_adjustables::UintParam),
            god_ray_weight: get_param!("god_ray_weight", crate::gui_adjustables::FloatParam),

            phi_c: get_param!("phi_c", crate::gui_adjustables::FloatParam),
            phi_n: get_param!("phi_n", crate::gui_adjustables::FloatParam),
            phi_p: get_param!("phi_p", crate::gui_adjustables::FloatParam),
            min_phi_z: get_param!("min_phi_z", crate::gui_adjustables::FloatParam),
            max_phi_z: get_param!("max_phi_z", crate::gui_adjustables::FloatParam),
            phi_z_stable_sample_count: get_param!(
                "phi_z_stable_sample_count",
                crate::gui_adjustables::FloatParam
            ),
            is_changing_lum_phi: get_param!(
                "is_changing_lum_phi",
                crate::gui_adjustables::BoolParam
            ),
            is_spatial_denoising_enabled: get_param!(
                "is_spatial_denoising_enabled",
                crate::gui_adjustables::BoolParam
            ),
            a_trous_iteration_count: get_param!(
                "a_trous_iteration_count",
                crate::gui_adjustables::UintParam
            ),

            grass_bottom_dark_color: get_param!(
                "grass_bottom_dark_color",
                crate::gui_adjustables::ColorParam
            ),
            grass_bottom_light_color: get_param!(
                "grass_bottom_light_color",
                crate::gui_adjustables::ColorParam
            ),
            grass_tip_dark_color: get_param!(
                "grass_tip_dark_color",
                crate::gui_adjustables::ColorParam
            ),
            grass_tip_light_color: get_param!(
                "grass_tip_light_color",
                crate::gui_adjustables::ColorParam
            ),

            ocean_deep_color: get_param!("ocean_deep_color", crate::gui_adjustables::ColorParam),
            ocean_shallow_color: get_param!(
                "ocean_shallow_color",
                crate::gui_adjustables::ColorParam
            ),
            ocean_normal_amplitude: get_param!(
                "ocean_normal_amplitude",
                crate::gui_adjustables::FloatParam
            ),
            ocean_noise_frequency: get_param!(
                "ocean_noise_frequency",
                crate::gui_adjustables::FloatParam
            ),
            ocean_time_multiplier: get_param!(
                "ocean_time_multiplier",
                crate::gui_adjustables::FloatParam
            ),

            ember_bloom_bottom_color: get_param!(
                "ember_bloom_bottom_color",
                crate::gui_adjustables::ColorParam
            ),
            ember_bloom_tip_color: get_param!(
                "ember_bloom_tip_color",
                crate::gui_adjustables::ColorParam
            ),

            flora_instance_hue_offset: get_param!(
                "flora_instance_hue_offset",
                crate::gui_adjustables::FloatParam
            ),
            flora_instance_saturation_offset: get_param!(
                "flora_instance_saturation_offset",
                crate::gui_adjustables::FloatParam
            ),
            flora_instance_value_offset: get_param!(
                "flora_instance_value_offset",
                crate::gui_adjustables::FloatParam
            ),
            flora_voxel_hue_offset: get_param!(
                "flora_voxel_hue_offset",
                crate::gui_adjustables::FloatParam
            ),
            flora_voxel_saturation_offset: get_param!(
                "flora_voxel_saturation_offset",
                crate::gui_adjustables::FloatParam
            ),
            flora_voxel_value_offset: get_param!(
                "flora_voxel_value_offset",
                crate::gui_adjustables::FloatParam
            ),

            leaves_inner_density: get_param!(
                "leaves_inner_density",
                crate::gui_adjustables::FloatParam
            ),
            leaves_outer_density: get_param!(
                "leaves_outer_density",
                crate::gui_adjustables::FloatParam
            ),
            leaves_inner_radius: get_param!(
                "leaves_inner_radius",
                crate::gui_adjustables::FloatParam
            ),
            leaves_outer_radius: get_param!(
                "leaves_outer_radius",
                crate::gui_adjustables::FloatParam
            ),
            leaves_bottom_color: get_param!(
                "leaves_bottom_color",
                crate::gui_adjustables::ColorParam
            ),
            leaves_tip_color: get_param!("leaves_tip_color", crate::gui_adjustables::ColorParam),

            particle_full_update_seconds: get_param!(
                "particle_full_update_seconds",
                crate::gui_adjustables::FloatParam
            ),

            butterflies_enabled: get_param!(
                "butterflies_enabled",
                crate::gui_adjustables::BoolParam
            ),
            butterflies_per_chunk: get_param!(
                "butterflies_per_chunk",
                crate::gui_adjustables::FloatParam
            ),
            butterfly_wander_radius: get_param!(
                "butterfly_wander_radius",
                crate::gui_adjustables::FloatParam
            ),
            butterfly_height_offset_min: get_param!(
                "butterfly_height_offset_min",
                crate::gui_adjustables::FloatParam
            ),
            butterfly_height_offset_max: get_param!(
                "butterfly_height_offset_max",
                crate::gui_adjustables::FloatParam
            ),
            butterfly_size: get_param!("butterfly_size", crate::gui_adjustables::FloatParam),
            butterfly_drift_strength_min: get_param!(
                "butterfly_drift_strength_min",
                crate::gui_adjustables::FloatParam
            ),
            butterfly_drift_strength_max: get_param!(
                "butterfly_drift_strength_max",
                crate::gui_adjustables::FloatParam
            ),
            butterfly_drift_frequency_min: get_param!(
                "butterfly_drift_frequency_min",
                crate::gui_adjustables::FloatParam
            ),
            butterfly_drift_frequency_max: get_param!(
                "butterfly_drift_frequency_max",
                crate::gui_adjustables::FloatParam
            ),
            butterfly_steering_strength: get_param!(
                "butterfly_steering_strength",
                crate::gui_adjustables::FloatParam
            ),
            butterfly_bob_frequency_hz: get_param!(
                "butterfly_bob_frequency_hz",
                crate::gui_adjustables::FloatParam
            ),
            butterfly_bob_strength: get_param!(
                "butterfly_bob_strength",
                crate::gui_adjustables::FloatParam
            ),
            butterfly_lifetime_min: get_param!(
                "butterfly_lifetime_min",
                crate::gui_adjustables::FloatParam
            ),
            butterfly_lifetime_max: get_param!(
                "butterfly_lifetime_max",
                crate::gui_adjustables::FloatParam
            ),

            voxel_dirt_color: get_param!("voxel_dirt_color", crate::gui_adjustables::ColorParam),
            voxel_cherry_wood_color: get_param!(
                "voxel_cherry_wood_color",
                crate::gui_adjustables::ColorParam
            ),
            voxel_oak_wood_color: get_param!(
                "voxel_oak_wood_color",
                crate::gui_adjustables::ColorParam
            ),
            voxel_color_variance: get_param!(
                "voxel_color_variance",
                crate::gui_adjustables::FloatParam
            ),
        }
}

declare_gui_adjustables! {
    [Debug] {
        debug_float: crate::gui_adjustables::FloatParam = 0.0, float(0.0..=10.0), "Debug Float",
        debug_uint: crate::gui_adjustables::UintParam = 0, uint(0..=100), "Debug UInt",
        lod_distance: crate::gui_adjustables::FloatParam = 0.0, float(0.0..=10.0), "LOD Distance",
        debug_bool: crate::gui_adjustables::BoolParam = true, bool, "Debug Bool",
    },

    [Sky] {
        sun_size: crate::gui_adjustables::FloatParam = 0.065, float(0.0..=1.0), "Size (relative)",
        sun_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(255, 241, 224), color, "Sun Color",
        sun_luminance: crate::gui_adjustables::FloatParam = 1.65, float(0.0..=10.0), "Sun Luminance",
        sun_display_luminance: crate::gui_adjustables::FloatParam = 1.65, float(0.0..=10.0), "Sun Display Luminance",
        ambient_light: crate::gui_adjustables::ColorParam = Color32::from_rgb(50, 50, 50), color, "Ambient Light",
        auto_daynight_cycle: crate::gui_adjustables::BoolParam = true, bool, "Auto Day/Night Cycle",
        time_of_day: crate::gui_adjustables::FloatParam = 0.45, float(0.0..=1.0), "Time of Day",
        latitude: crate::gui_adjustables::FloatParam = -0.7, float(-1.0..=1.0), "Latitude",
        season: crate::gui_adjustables::FloatParam = 0.25, float(0.0..=1.0), "Season",
        day_cycle_minutes: crate::gui_adjustables::FloatParam = 30.0, float(0.1..=60.0), "Day Cycle Duration (Minutes)",
    },

    [Starlight] {
        starlight_iterations: crate::gui_adjustables::IntParam = 18, int(1..=30), "Iterations",
        starlight_formuparam: crate::gui_adjustables::FloatParam = 0.42, float(0.0..=1.0), "Form Parameter",
        starlight_volsteps: crate::gui_adjustables::IntParam = 12, int(1..=50), "Volume Steps",
        starlight_stepsize: crate::gui_adjustables::FloatParam = 0.27, float(0.01..=1.0), "Step Size",
        starlight_zoom: crate::gui_adjustables::FloatParam = 0.1, float(0.1..=2.0), "Zoom",
        starlight_tile: crate::gui_adjustables::FloatParam = 1.02, float(0.1..=2.0), "Tile",
        starlight_speed: crate::gui_adjustables::FloatParam = 0.077, float(0.001..=0.1), "Speed",
        starlight_brightness: crate::gui_adjustables::FloatParam = 0.0021, float(0.0001..=0.01), "Brightness",
        starlight_darkmatter: crate::gui_adjustables::FloatParam = 0.57, float(0.0..=1.0), "Dark Matter",
        starlight_distfading: crate::gui_adjustables::FloatParam = 0.46, float(0.0..=1.0), "Distance Fading",
        starlight_saturation: crate::gui_adjustables::FloatParam = 0.97, float(0.0..=1.0), "Saturation",
    },

    [Temporal] {
        temporal_position_phi: crate::gui_adjustables::FloatParam = 0.8, float(0.0..=1.0), "Position Phi",
        temporal_alpha: crate::gui_adjustables::FloatParam = 0.08, float(0.0..=1.0), "Alpha",
    },

    [GodRay] {
        god_ray_max_depth: crate::gui_adjustables::FloatParam = 1.0, float(0.1..=10.0), "Max Depth",
        god_ray_max_checks: crate::gui_adjustables::UintParam = 32, uint(1..=64), "Max Checks",
        god_ray_weight: crate::gui_adjustables::FloatParam = 0.3, float(0.0..=1.0), "Weight",
    },

    [Spatial] {
        phi_c: crate::gui_adjustables::FloatParam = 0.75, float(0.0..=1.0), "Phi C",
        phi_n: crate::gui_adjustables::FloatParam = 20.0, float(0.0..=1.0), "Phi N",
        phi_p: crate::gui_adjustables::FloatParam = 0.05, float(0.0..=1.0), "Phi P",
        min_phi_z: crate::gui_adjustables::FloatParam = 0.0, float(0.0..=1.0), "Min Phi Z",
        max_phi_z: crate::gui_adjustables::FloatParam = 0.5, float(0.0..=1.0), "Max Phi Z",
        phi_z_stable_sample_count: crate::gui_adjustables::FloatParam = 0.05, float(0.0..=1.0), "Phi Z Stable Sample Count",
        is_changing_lum_phi: crate::gui_adjustables::BoolParam = true, bool, "Changing Luminance Phi",
        is_spatial_denoising_enabled: crate::gui_adjustables::BoolParam = true, bool, "Enable Spatial Denoising",
        a_trous_iteration_count: crate::gui_adjustables::UintParam = 3, uint(1..=5), "A-Trous Iterations",
    },

    [Grass] {
        grass_bottom_dark_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(0, 82, 64), color, "Bottom Dark",
        grass_bottom_light_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(0, 175, 108), color, "Bottom Light",
        grass_tip_dark_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(218, 219, 0), color, "Tip Dark",
        grass_tip_light_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(148, 190, 0), color, "Tip Light",
    },

    [Ocean] {
        ocean_deep_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(10, 60, 130), color, "Deep Ocean Color",
        ocean_shallow_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(40, 150, 210), color, "Shallow Water Color",
        ocean_normal_amplitude: crate::gui_adjustables::FloatParam = 0.18, float(0.0..=0.6), "Normal Amplitude",
        ocean_noise_frequency: crate::gui_adjustables::FloatParam = 0.0045, float(0.0005..=1.0), "Noise Frequency",
        ocean_time_multiplier: crate::gui_adjustables::FloatParam = 1.0, float(0.0..=5.0), "Time Multiplier",
    },

    [EmberBloom] {
        ember_bloom_bottom_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(88, 0, 50), color, "Bottom Color",
        ember_bloom_tip_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(255, 181, 255), color, "Tip Color",
    },

    [FloraVariation] {
        flora_instance_hue_offset: crate::gui_adjustables::FloatParam = 0.00, float(0.0..=1.0), "Instance Hue Offset Max",
        flora_instance_saturation_offset: crate::gui_adjustables::FloatParam = 0.00, float(0.0..=1.0), "Instance Saturation Offset Max",
        flora_instance_value_offset: crate::gui_adjustables::FloatParam = 0.00, float(0.0..=1.0), "Instance Value Offset Max",
        flora_voxel_hue_offset: crate::gui_adjustables::FloatParam = 0.00, float(0.0..=1.0), "Voxel Hue Offset Max",
        flora_voxel_saturation_offset: crate::gui_adjustables::FloatParam = 0.00, float(0.0..=1.0), "Voxel Saturation Offset Max",
        flora_voxel_value_offset: crate::gui_adjustables::FloatParam = 0.00, float(0.0..=1.0), "Voxel Value Offset Max",
    },

    [Leaves] {
        leaves_inner_density: crate::gui_adjustables::FloatParam = 0.38, float(0.0..=1.0), "Inner Density",
        leaves_outer_density: crate::gui_adjustables::FloatParam = 0.45, float(0.0..=1.0), "Outer Density",
        leaves_inner_radius: crate::gui_adjustables::FloatParam = 12.0, float(1.0..=64.0), "Inner Radius",
        leaves_outer_radius: crate::gui_adjustables::FloatParam = 17.0, float(1.0..=64.0), "Outer Radius",
        leaves_bottom_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(239, 239, 239), color, "Bottom Color",
        leaves_tip_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(255, 137, 241), color, "Tip Color",
    },

    [Particles] {
        particle_full_update_seconds: crate::gui_adjustables::FloatParam = crate::particles::PARTICLE_FULL_UPDATE_SECONDS_DEFAULT, float(0.05..=5.0), "Full Update Time (s)",
    },

    [Butterflies] {
        butterflies_enabled: crate::gui_adjustables::BoolParam = true, bool, "Enable Butterflies",
        butterflies_per_chunk: crate::gui_adjustables::FloatParam = 0.5, float(0.0..=2.0), "Butterflies Per Chunk",
        butterfly_wander_radius: crate::gui_adjustables::FloatParam = 2.5, float(0.5..=8.0), "Wander Radius",
        butterfly_height_offset_min: crate::gui_adjustables::FloatParam = 0.06, float(0.0..=4.0), "Height Offset Min",
        butterfly_height_offset_max: crate::gui_adjustables::FloatParam = 0.14, float(0.0..=4.0), "Height Offset Max",
        butterfly_size: crate::gui_adjustables::FloatParam = 0.018, float(0.001..=0.03), "Size",
        butterfly_drift_strength_min: crate::gui_adjustables::FloatParam = 0.6, float(0.0..=3.0), "Flutter Strength Min",
        butterfly_drift_strength_max: crate::gui_adjustables::FloatParam = 1.4, float(0.0..=3.0), "Flutter Strength Max",
        butterfly_drift_frequency_min: crate::gui_adjustables::FloatParam = 1.5, float(0.5..=6.0), "Flutter Speed Min",
        butterfly_drift_frequency_max: crate::gui_adjustables::FloatParam = 3.5, float(0.5..=6.0), "Flutter Speed Max",
        butterfly_steering_strength: crate::gui_adjustables::FloatParam = 0.9, float(0.0..=3.0), "Home Pull Strength",
        butterfly_bob_frequency_hz: crate::gui_adjustables::FloatParam = 2.2, float(0.0..=8.0), "Vertical Bob Frequency (Hz)",
        butterfly_bob_strength: crate::gui_adjustables::FloatParam = 1.4, float(0.0..=6.0), "Vertical Bob Strength",
        butterfly_lifetime_min: crate::gui_adjustables::FloatParam = 10.0, float(1.0..=60.0), "Lifetime Min (s)",
        butterfly_lifetime_max: crate::gui_adjustables::FloatParam = 15.0, float(1.0..=60.0), "Lifetime Max (s)",
    },

    [Voxel] {
        voxel_dirt_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(95, 95, 95), color, "Dirt Color",
        voxel_cherry_wood_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(202, 176, 92), color, "Cherry Wood Color",
        voxel_oak_wood_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(166, 144, 75), color, "Oak Wood Color",
        voxel_color_variance: crate::gui_adjustables::FloatParam = 1.0, float(0.0..=2.0), "Hash Color Variance",
    },
}

impl Default for GuiAdjustables {
    fn default() -> Self {
        let config = GuiConfigLoader::load();
        Self::from_config(&config)
    }
}

impl GuiAdjustables {
    pub fn from_config(config: &crate::app::gui_config_model::GuiConfigFile) -> Self {
        load_from_config(config)
    }
    const SAVE_DENYLIST: &'static [&'static str] = &["time_of_day"];

    pub fn save_to_config(&self) -> std::io::Result<()> {
        let mut config = GuiConfigLoader::load();

        for section in &mut config.section {
            for param in &mut section.param {
                if Self::SAVE_DENYLIST.contains(&param.id.as_str()) {
                    continue;
                }

                let value_updated = match param.kind {
                    GuiParamKind::Float => {
                        if let Some(field) = Self::get_float_param(self, &param.id) {
                            param.value.set_float(field.value);
                            true
                        } else {
                            false
                        }
                    }
                    GuiParamKind::Int => {
                        if let Some(field) = Self::get_int_param(self, &param.id) {
                            param.value.set_int(field.value);
                            true
                        } else {
                            false
                        }
                    }
                    GuiParamKind::Uint => {
                        if let Some(field) = Self::get_uint_param(self, &param.id) {
                            param.value.set_uint(field.value);
                            true
                        } else {
                            false
                        }
                    }
                    GuiParamKind::Bool => {
                        if let Some(field) = Self::get_bool_param(self, &param.id) {
                            param.value.set_bool(field.value);
                            true
                        } else {
                            false
                        }
                    }
                    GuiParamKind::Color => {
                        if let Some(field) = Self::get_color_param(self, &param.id) {
                            param.value.set_color(color_to_hex(field.value));
                            true
                        } else {
                            false
                        }
                    }
                };
                if !value_updated {
                    log::warn!(
                        "Failed to update config value for param '{}' in section '{}'",
                        param.id,
                        section.name
                    );
                }
            }
        }

        GuiConfigLoader::save(&config)
    }

    #[allow(dead_code)]
    fn get_float_param<'a>(
        adjustables: &'a GuiAdjustables,
        id: &str,
    ) -> Option<&'a crate::gui_adjustables::FloatParam> {
        match id {
            "debug_float" => Some(&adjustables.debug_float),
            "lod_distance" => Some(&adjustables.lod_distance),
            "sun_size" => Some(&adjustables.sun_size),
            "sun_luminance" => Some(&adjustables.sun_luminance),
            "sun_display_luminance" => Some(&adjustables.sun_display_luminance),
            "time_of_day" => Some(&adjustables.time_of_day),
            "latitude" => Some(&adjustables.latitude),
            "season" => Some(&adjustables.season),
            "day_cycle_minutes" => Some(&adjustables.day_cycle_minutes),
            "starlight_formuparam" => Some(&adjustables.starlight_formuparam),
            "starlight_stepsize" => Some(&adjustables.starlight_stepsize),
            "starlight_zoom" => Some(&adjustables.starlight_zoom),
            "starlight_tile" => Some(&adjustables.starlight_tile),
            "starlight_speed" => Some(&adjustables.starlight_speed),
            "starlight_brightness" => Some(&adjustables.starlight_brightness),
            "starlight_darkmatter" => Some(&adjustables.starlight_darkmatter),
            "starlight_distfading" => Some(&adjustables.starlight_distfading),
            "starlight_saturation" => Some(&adjustables.starlight_saturation),
            "temporal_position_phi" => Some(&adjustables.temporal_position_phi),
            "temporal_alpha" => Some(&adjustables.temporal_alpha),
            "god_ray_max_depth" => Some(&adjustables.god_ray_max_depth),
            "god_ray_weight" => Some(&adjustables.god_ray_weight),
            "phi_c" => Some(&adjustables.phi_c),
            "phi_n" => Some(&adjustables.phi_n),
            "phi_p" => Some(&adjustables.phi_p),
            "min_phi_z" => Some(&adjustables.min_phi_z),
            "max_phi_z" => Some(&adjustables.max_phi_z),
            "phi_z_stable_sample_count" => Some(&adjustables.phi_z_stable_sample_count),
            "flora_instance_hue_offset" => Some(&adjustables.flora_instance_hue_offset),
            "flora_instance_saturation_offset" => {
                Some(&adjustables.flora_instance_saturation_offset)
            }
            "flora_instance_value_offset" => Some(&adjustables.flora_instance_value_offset),
            "flora_voxel_hue_offset" => Some(&adjustables.flora_voxel_hue_offset),
            "flora_voxel_saturation_offset" => Some(&adjustables.flora_voxel_saturation_offset),
            "flora_voxel_value_offset" => Some(&adjustables.flora_voxel_value_offset),
            "leaves_inner_density" => Some(&adjustables.leaves_inner_density),
            "leaves_outer_density" => Some(&adjustables.leaves_outer_density),
            "leaves_inner_radius" => Some(&adjustables.leaves_inner_radius),
            "leaves_outer_radius" => Some(&adjustables.leaves_outer_radius),
            "particle_full_update_seconds" => Some(&adjustables.particle_full_update_seconds),
            "butterflies_per_chunk" => Some(&adjustables.butterflies_per_chunk),
            "butterfly_wander_radius" => Some(&adjustables.butterfly_wander_radius),
            "butterfly_height_offset_min" => Some(&adjustables.butterfly_height_offset_min),
            "butterfly_height_offset_max" => Some(&adjustables.butterfly_height_offset_max),
            "butterfly_size" => Some(&adjustables.butterfly_size),
            "butterfly_drift_strength_min" => Some(&adjustables.butterfly_drift_strength_min),
            "butterfly_drift_strength_max" => Some(&adjustables.butterfly_drift_strength_max),
            "butterfly_drift_frequency_min" => Some(&adjustables.butterfly_drift_frequency_min),
            "butterfly_drift_frequency_max" => Some(&adjustables.butterfly_drift_frequency_max),
            "butterfly_steering_strength" => Some(&adjustables.butterfly_steering_strength),
            "butterfly_bob_frequency_hz" => Some(&adjustables.butterfly_bob_frequency_hz),
            "butterfly_bob_strength" => Some(&adjustables.butterfly_bob_strength),
            "butterfly_lifetime_min" => Some(&adjustables.butterfly_lifetime_min),
            "butterfly_lifetime_max" => Some(&adjustables.butterfly_lifetime_max),
            "ocean_normal_amplitude" => Some(&adjustables.ocean_normal_amplitude),
            "ocean_noise_frequency" => Some(&adjustables.ocean_noise_frequency),
            "ocean_time_multiplier" => Some(&adjustables.ocean_time_multiplier),
            "voxel_color_variance" => Some(&adjustables.voxel_color_variance),
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn get_int_param<'a>(
        adjustables: &'a GuiAdjustables,
        id: &str,
    ) -> Option<&'a crate::gui_adjustables::IntParam> {
        match id {
            "starlight_iterations" => Some(&adjustables.starlight_iterations),
            "starlight_volsteps" => Some(&adjustables.starlight_volsteps),
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn get_uint_param<'a>(
        adjustables: &'a GuiAdjustables,
        id: &str,
    ) -> Option<&'a crate::gui_adjustables::UintParam> {
        match id {
            "debug_uint" => Some(&adjustables.debug_uint),
            "god_ray_max_checks" => Some(&adjustables.god_ray_max_checks),
            "a_trous_iteration_count" => Some(&adjustables.a_trous_iteration_count),
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn get_bool_param<'a>(
        adjustables: &'a GuiAdjustables,
        id: &str,
    ) -> Option<&'a crate::gui_adjustables::BoolParam> {
        match id {
            "debug_bool" => Some(&adjustables.debug_bool),
            "auto_daynight_cycle" => Some(&adjustables.auto_daynight_cycle),
            "is_changing_lum_phi" => Some(&adjustables.is_changing_lum_phi),
            "is_spatial_denoising_enabled" => Some(&adjustables.is_spatial_denoising_enabled),
            "butterflies_enabled" => Some(&adjustables.butterflies_enabled),
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn get_color_param<'a>(
        adjustables: &'a GuiAdjustables,
        id: &str,
    ) -> Option<&'a crate::gui_adjustables::ColorParam> {
        match id {
            "sun_color" => Some(&adjustables.sun_color),
            "ambient_light" => Some(&adjustables.ambient_light),
            "grass_bottom_dark_color" => Some(&adjustables.grass_bottom_dark_color),
            "grass_bottom_light_color" => Some(&adjustables.grass_bottom_light_color),
            "grass_tip_dark_color" => Some(&adjustables.grass_tip_dark_color),
            "grass_tip_light_color" => Some(&adjustables.grass_tip_light_color),
            "ocean_deep_color" => Some(&adjustables.ocean_deep_color),
            "ocean_shallow_color" => Some(&adjustables.ocean_shallow_color),
            "ember_bloom_bottom_color" => Some(&adjustables.ember_bloom_bottom_color),
            "ember_bloom_tip_color" => Some(&adjustables.ember_bloom_tip_color),
            "leaves_bottom_color" => Some(&adjustables.leaves_bottom_color),
            "leaves_tip_color" => Some(&adjustables.leaves_tip_color),
            "voxel_dirt_color" => Some(&adjustables.voxel_dirt_color),
            "voxel_cherry_wood_color" => Some(&adjustables.voxel_cherry_wood_color),
            "voxel_oak_wood_color" => Some(&adjustables.voxel_oak_wood_color),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn get_float_param_mut<'a>(
        adjustables: &'a mut GuiAdjustables,
        id: &str,
    ) -> Option<&'a mut crate::gui_adjustables::FloatParam> {
        match id {
            "debug_float" => Some(&mut adjustables.debug_float),
            "lod_distance" => Some(&mut adjustables.lod_distance),
            "sun_size" => Some(&mut adjustables.sun_size),
            "sun_luminance" => Some(&mut adjustables.sun_luminance),
            "sun_display_luminance" => Some(&mut adjustables.sun_display_luminance),
            "time_of_day" => Some(&mut adjustables.time_of_day),
            "latitude" => Some(&mut adjustables.latitude),
            "season" => Some(&mut adjustables.season),
            "day_cycle_minutes" => Some(&mut adjustables.day_cycle_minutes),
            "starlight_formuparam" => Some(&mut adjustables.starlight_formuparam),
            "starlight_stepsize" => Some(&mut adjustables.starlight_stepsize),
            "starlight_zoom" => Some(&mut adjustables.starlight_zoom),
            "starlight_tile" => Some(&mut adjustables.starlight_tile),
            "starlight_speed" => Some(&mut adjustables.starlight_speed),
            "starlight_brightness" => Some(&mut adjustables.starlight_brightness),
            "starlight_darkmatter" => Some(&mut adjustables.starlight_darkmatter),
            "starlight_distfading" => Some(&mut adjustables.starlight_distfading),
            "starlight_saturation" => Some(&mut adjustables.starlight_saturation),
            "temporal_position_phi" => Some(&mut adjustables.temporal_position_phi),
            "temporal_alpha" => Some(&mut adjustables.temporal_alpha),
            "god_ray_max_depth" => Some(&mut adjustables.god_ray_max_depth),
            "god_ray_weight" => Some(&mut adjustables.god_ray_weight),
            "phi_c" => Some(&mut adjustables.phi_c),
            "phi_n" => Some(&mut adjustables.phi_n),
            "phi_p" => Some(&mut adjustables.phi_p),
            "min_phi_z" => Some(&mut adjustables.min_phi_z),
            "max_phi_z" => Some(&mut adjustables.max_phi_z),
            "phi_z_stable_sample_count" => Some(&mut adjustables.phi_z_stable_sample_count),
            "flora_instance_hue_offset" => Some(&mut adjustables.flora_instance_hue_offset),
            "flora_instance_saturation_offset" => {
                Some(&mut adjustables.flora_instance_saturation_offset)
            }
            "flora_instance_value_offset" => Some(&mut adjustables.flora_instance_value_offset),
            "flora_voxel_hue_offset" => Some(&mut adjustables.flora_voxel_hue_offset),
            "flora_voxel_saturation_offset" => Some(&mut adjustables.flora_voxel_saturation_offset),
            "flora_voxel_value_offset" => Some(&mut adjustables.flora_voxel_value_offset),
            "leaves_inner_density" => Some(&mut adjustables.leaves_inner_density),
            "leaves_outer_density" => Some(&mut adjustables.leaves_outer_density),
            "leaves_inner_radius" => Some(&mut adjustables.leaves_inner_radius),
            "leaves_outer_radius" => Some(&mut adjustables.leaves_outer_radius),
            "particle_full_update_seconds" => Some(&mut adjustables.particle_full_update_seconds),
            "butterflies_per_chunk" => Some(&mut adjustables.butterflies_per_chunk),
            "butterfly_wander_radius" => Some(&mut adjustables.butterfly_wander_radius),
            "butterfly_height_offset_min" => Some(&mut adjustables.butterfly_height_offset_min),
            "butterfly_height_offset_max" => Some(&mut adjustables.butterfly_height_offset_max),
            "butterfly_size" => Some(&mut adjustables.butterfly_size),
            "butterfly_drift_strength_min" => Some(&mut adjustables.butterfly_drift_strength_min),
            "butterfly_drift_strength_max" => Some(&mut adjustables.butterfly_drift_strength_max),
            "butterfly_drift_frequency_min" => Some(&mut adjustables.butterfly_drift_frequency_min),
            "butterfly_drift_frequency_max" => Some(&mut adjustables.butterfly_drift_frequency_max),
            "butterfly_steering_strength" => Some(&mut adjustables.butterfly_steering_strength),
            "butterfly_bob_frequency_hz" => Some(&mut adjustables.butterfly_bob_frequency_hz),
            "butterfly_bob_strength" => Some(&mut adjustables.butterfly_bob_strength),
            "butterfly_lifetime_min" => Some(&mut adjustables.butterfly_lifetime_min),
            "butterfly_lifetime_max" => Some(&mut adjustables.butterfly_lifetime_max),
            "ocean_normal_amplitude" => Some(&mut adjustables.ocean_normal_amplitude),
            "ocean_noise_frequency" => Some(&mut adjustables.ocean_noise_frequency),
            "ocean_time_multiplier" => Some(&mut adjustables.ocean_time_multiplier),
            "voxel_color_variance" => Some(&mut adjustables.voxel_color_variance),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn get_int_param_mut<'a>(
        adjustables: &'a mut GuiAdjustables,
        id: &str,
    ) -> Option<&'a mut crate::gui_adjustables::IntParam> {
        match id {
            "starlight_iterations" => Some(&mut adjustables.starlight_iterations),
            "starlight_volsteps" => Some(&mut adjustables.starlight_volsteps),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn get_uint_param_mut<'a>(
        adjustables: &'a mut GuiAdjustables,
        id: &str,
    ) -> Option<&'a mut crate::gui_adjustables::UintParam> {
        match id {
            "debug_uint" => Some(&mut adjustables.debug_uint),
            "god_ray_max_checks" => Some(&mut adjustables.god_ray_max_checks),
            "a_trous_iteration_count" => Some(&mut adjustables.a_trous_iteration_count),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn get_bool_param_mut<'a>(
        adjustables: &'a mut GuiAdjustables,
        id: &str,
    ) -> Option<&'a mut crate::gui_adjustables::BoolParam> {
        match id {
            "debug_bool" => Some(&mut adjustables.debug_bool),
            "auto_daynight_cycle" => Some(&mut adjustables.auto_daynight_cycle),
            "is_changing_lum_phi" => Some(&mut adjustables.is_changing_lum_phi),
            "is_spatial_denoising_enabled" => Some(&mut adjustables.is_spatial_denoising_enabled),
            "butterflies_enabled" => Some(&mut adjustables.butterflies_enabled),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn get_color_param_mut<'a>(
        adjustables: &'a mut GuiAdjustables,
        id: &str,
    ) -> Option<&'a mut crate::gui_adjustables::ColorParam> {
        match id {
            "sun_color" => Some(&mut adjustables.sun_color),
            "ambient_light" => Some(&mut adjustables.ambient_light),
            "grass_bottom_dark_color" => Some(&mut adjustables.grass_bottom_dark_color),
            "grass_bottom_light_color" => Some(&mut adjustables.grass_bottom_light_color),
            "grass_tip_dark_color" => Some(&mut adjustables.grass_tip_dark_color),
            "grass_tip_light_color" => Some(&mut adjustables.grass_tip_light_color),
            "ocean_deep_color" => Some(&mut adjustables.ocean_deep_color),
            "ocean_shallow_color" => Some(&mut adjustables.ocean_shallow_color),
            "ember_bloom_bottom_color" => Some(&mut adjustables.ember_bloom_bottom_color),
            "ember_bloom_tip_color" => Some(&mut adjustables.ember_bloom_tip_color),
            "leaves_bottom_color" => Some(&mut adjustables.leaves_bottom_color),
            "leaves_tip_color" => Some(&mut adjustables.leaves_tip_color),
            "voxel_dirt_color" => Some(&mut adjustables.voxel_dirt_color),
            "voxel_cherry_wood_color" => Some(&mut adjustables.voxel_cherry_wood_color),
            "voxel_oak_wood_color" => Some(&mut adjustables.voxel_oak_wood_color),
            _ => None,
        }
    }
}

pub fn render_gui_from_config(
    ui: &mut egui::Ui,
    config: &GuiConfigFile,
    adjustables: &mut GuiAdjustables,
) {
    use crate::app::gui_config_model::GuiParamKind;

    for section in &config.section {
        ui.collapsing(&section.name, |ui| {
            for param in &section.param {
                match (&param.kind, &param.value) {
                    (GuiParamKind::Float, GuiParamValue::Float { min, max, .. }) => {
                        if let Some(field) =
                            GuiAdjustables::get_float_param_mut(adjustables, &param.id)
                        {
                            let range = min.unwrap_or(0.0)..=max.unwrap_or(1.0);
                            ui.add(egui::Slider::new(&mut field.value, range).text(&param.label));
                        } else {
                            ui.label(format!("[UNWIRED] {}", param.label));
                        }
                    }
                    (GuiParamKind::Int, GuiParamValue::Int { min, max, .. }) => {
                        if let Some(field) =
                            GuiAdjustables::get_int_param_mut(adjustables, &param.id)
                        {
                            let range = min.unwrap_or(0)..=max.unwrap_or(100);
                            ui.add(egui::Slider::new(&mut field.value, range).text(&param.label));
                        } else {
                            ui.label(format!("[UNWIRED] {}", param.label));
                        }
                    }
                    (GuiParamKind::Uint, GuiParamValue::Uint { min, max, .. }) => {
                        if let Some(field) =
                            GuiAdjustables::get_uint_param_mut(adjustables, &param.id)
                        {
                            let range = min.unwrap_or(0)..=max.unwrap_or(100);
                            ui.add(egui::Slider::new(&mut field.value, range).text(&param.label));
                        } else {
                            ui.label(format!("[UNWIRED] {}", param.label));
                        }
                    }
                    (GuiParamKind::Bool, GuiParamValue::Bool { .. }) => {
                        if let Some(field) =
                            GuiAdjustables::get_bool_param_mut(adjustables, &param.id)
                        {
                            ui.checkbox(&mut field.value, &param.label);
                        } else {
                            ui.label(format!("[UNWIRED] {}", param.label));
                        }
                    }
                    (GuiParamKind::Color, GuiParamValue::Color { .. }) => {
                        if let Some(field) =
                            GuiAdjustables::get_color_param_mut(adjustables, &param.id)
                        {
                            ui.horizontal(|ui| {
                                ui.label(&param.label);
                                ui.color_edit_button_srgba(&mut field.value);
                            });
                        } else {
                            ui.label(format!("[UNWIRED] {}", param.label));
                        }
                    }
                    _ => {
                        ui.label(format!("[TYPE MISMATCH] {}", param.label));
                    }
                }
            }
        });
    }
}
