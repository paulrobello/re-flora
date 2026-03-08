/// GUI Adjustables Configuration
///
/// This file loads GUI parameters from config/gui.toml.
/// The config file is the single source of truth.
use crate::app::gui_config_loader::GuiConfigLoader;
use crate::app::gui_config_model::{GuiConfigFile, GuiParamKind};
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

    GuiAdjustables {
        debug_float: get_param!("debug_float", crate::gui_adjustables::FloatParam),
        debug_uint: get_param!("debug_uint", crate::gui_adjustables::UintParam),
        lod_distance: get_param!("lod_distance", crate::gui_adjustables::FloatParam),
        debug_bool: get_param!("debug_bool", crate::gui_adjustables::BoolParam),

        sun_altitude: get_param!("sun_altitude", crate::gui_adjustables::FloatParam),
        sun_azimuth: get_param!("sun_azimuth", crate::gui_adjustables::FloatParam),
        sun_size: get_param!("sun_size", crate::gui_adjustables::FloatParam),
        sun_color: get_param!("sun_color", crate::gui_adjustables::ColorParam),
        sun_luminance: get_param!("sun_luminance", crate::gui_adjustables::FloatParam),
        ambient_light: get_param!("ambient_light", crate::gui_adjustables::ColorParam),
        auto_daynight_cycle: get_param!("auto_daynight_cycle", crate::gui_adjustables::BoolParam),
        time_of_day: get_param!("time_of_day", crate::gui_adjustables::FloatParam),
        latitude: get_param!("latitude", crate::gui_adjustables::FloatParam),
        season: get_param!("season", crate::gui_adjustables::FloatParam),
        day_cycle_minutes: get_param!("day_cycle_minutes", crate::gui_adjustables::FloatParam),

        starlight_iterations: get_param!("starlight_iterations", crate::gui_adjustables::IntParam),
        starlight_formuparam: get_param!(
            "starlight_formuparam",
            crate::gui_adjustables::FloatParam
        ),
        starlight_volsteps: get_param!("starlight_volsteps", crate::gui_adjustables::IntParam),
        starlight_stepsize: get_param!("starlight_stepsize", crate::gui_adjustables::FloatParam),
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
        is_changing_lum_phi: get_param!("is_changing_lum_phi", crate::gui_adjustables::BoolParam),
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
        leaves_inner_radius: get_param!("leaves_inner_radius", crate::gui_adjustables::FloatParam),
        leaves_outer_radius: get_param!("leaves_outer_radius", crate::gui_adjustables::FloatParam),
        leaves_bottom_color: get_param!("leaves_bottom_color", crate::gui_adjustables::ColorParam),
        leaves_tip_color: get_param!("leaves_tip_color", crate::gui_adjustables::ColorParam),

        particle_full_update_seconds: get_param!(
            "particle_full_update_seconds",
            crate::gui_adjustables::FloatParam
        ),

        butterflies_enabled: get_param!("butterflies_enabled", crate::gui_adjustables::BoolParam),
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
        butterfly_wing_color_low: get_param!(
            "butterfly_wing_color_low",
            crate::gui_adjustables::ColorParam
        ),
        butterfly_wing_color_high: get_param!(
            "butterfly_wing_color_high",
            crate::gui_adjustables::ColorParam
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
        sun_altitude: crate::gui_adjustables::FloatParam = 0.25, float(-1.0..=1.0), "Altitude (normalized)",
        sun_azimuth: crate::gui_adjustables::FloatParam = 0.8, float(0.0..=1.0), "Azimuth (normalized)",
        sun_size: crate::gui_adjustables::FloatParam = 0.065, float(0.0..=1.0), "Size (relative)",
        sun_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(255, 241, 224), color, "Sun Color",
        sun_luminance: crate::gui_adjustables::FloatParam = 1.65, float(0.0..=10.0), "Sun Luminance",
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
        butterfly_wing_color_low: crate::gui_adjustables::ColorParam = Color32::from_rgb(242, 230, 140), color, "Wing Color Low",
        butterfly_wing_color_high: crate::gui_adjustables::ColorParam = Color32::from_rgb(255, 247, 184), color, "Wing Color High",
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
        load_from_config(&config)
    }
}
