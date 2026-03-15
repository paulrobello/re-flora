/// GUI Adjustables Configuration
///
/// This file loads GUI parameters from config/gui.toml.
/// The config file is the single source of truth.
use crate::app::gui_config_loader::GuiConfigLoader;
use crate::app::gui_config_model::{GuiConfigFile, GuiParamKind, GuiParamValue};
use crate::declare_gui_adjustables;
use egui::Color32;

mod generated {
    include!("generated/gui_adjustables_gen.rs");
}

pub use generated::{
    GeneratedGuiAdjustables, GeneratedGuiParamDescriptor, GENERATED_GUI_PARAMS,
    GENERATED_SCHEMA_VERSION,
};

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
    let generated = GeneratedGuiAdjustables::from_config(config);

    GuiAdjustables {
        debug_float: generated.debug_float,
        debug_uint: generated.debug_uint,
        lod_distance: generated.lod_distance,
        debug_bool: generated.debug_bool,

        sun_size: generated.sun_size,
        sun_color: generated.sun_color,
        sun_luminance: generated.sun_luminance,
        sun_display_luminance: generated.sun_display_luminance,
        ambient_light: generated.ambient_light,
        auto_daynight_cycle: generated.auto_daynight_cycle,
        time_of_day: generated.time_of_day,
        latitude: generated.latitude,
        season: generated.season,
        day_cycle_minutes: generated.day_cycle_minutes,

        starlight_iterations: generated.starlight_iterations,
        starlight_formuparam: generated.starlight_formuparam,
        starlight_volsteps: generated.starlight_volsteps,
        starlight_stepsize: generated.starlight_stepsize,
        starlight_zoom: generated.starlight_zoom,
        starlight_tile: generated.starlight_tile,
        starlight_speed: generated.starlight_speed,
        starlight_brightness: generated.starlight_brightness,
        starlight_darkmatter: generated.starlight_darkmatter,
        starlight_distfading: generated.starlight_distfading,
        starlight_saturation: generated.starlight_saturation,

        temporal_position_phi: generated.temporal_position_phi,
        temporal_alpha: generated.temporal_alpha,

        god_ray_max_depth: generated.god_ray_max_depth,
        god_ray_max_checks: generated.god_ray_max_checks,
        god_ray_weight: generated.god_ray_weight,

        phi_c: generated.phi_c,
        phi_n: generated.phi_n,
        phi_p: generated.phi_p,
        min_phi_z: generated.min_phi_z,
        max_phi_z: generated.max_phi_z,
        phi_z_stable_sample_count: generated.phi_z_stable_sample_count,
        is_changing_lum_phi: generated.is_changing_lum_phi,
        is_spatial_denoising_enabled: generated.is_spatial_denoising_enabled,
        a_trous_iteration_count: generated.a_trous_iteration_count,

        grass_bottom_dark_color: generated.grass_bottom_dark_color,
        grass_bottom_light_color: generated.grass_bottom_light_color,
        grass_tip_dark_color: generated.grass_tip_dark_color,
        grass_tip_light_color: generated.grass_tip_light_color,

        ocean_deep_color: generated.ocean_deep_color,
        ocean_shallow_color: generated.ocean_shallow_color,
        ocean_normal_amplitude: generated.ocean_normal_amplitude,
        ocean_noise_frequency: generated.ocean_noise_frequency,
        ocean_time_multiplier: generated.ocean_time_multiplier,

        ember_bloom_bottom_color: generated.ember_bloom_bottom_color,
        ember_bloom_tip_color: generated.ember_bloom_tip_color,

        flora_instance_hue_offset: generated.flora_instance_hue_offset,
        flora_instance_saturation_offset: generated.flora_instance_saturation_offset,
        flora_instance_value_offset: generated.flora_instance_value_offset,
        flora_voxel_hue_offset: generated.flora_voxel_hue_offset,
        flora_voxel_saturation_offset: generated.flora_voxel_saturation_offset,
        flora_voxel_value_offset: generated.flora_voxel_value_offset,

        leaves_inner_density: generated.leaves_inner_density,
        leaves_outer_density: generated.leaves_outer_density,
        leaves_inner_radius: generated.leaves_inner_radius,
        leaves_outer_radius: generated.leaves_outer_radius,
        leaves_bottom_color: generated.leaves_bottom_color,
        leaves_tip_color: generated.leaves_tip_color,

        particle_full_update_seconds: generated.particle_full_update_seconds,

        butterflies_enabled: generated.butterflies_enabled,
        butterflies_per_chunk: generated.butterflies_per_chunk,
        butterfly_wander_radius: generated.butterfly_wander_radius,
        butterfly_height_offset_min: generated.butterfly_height_offset_min,
        butterfly_height_offset_max: generated.butterfly_height_offset_max,
        butterfly_size: generated.butterfly_size,
        butterfly_drift_strength_min: generated.butterfly_drift_strength_min,
        butterfly_drift_strength_max: generated.butterfly_drift_strength_max,
        butterfly_drift_frequency_min: generated.butterfly_drift_frequency_min,
        butterfly_drift_frequency_max: generated.butterfly_drift_frequency_max,
        butterfly_steering_strength: generated.butterfly_steering_strength,
        butterfly_bob_frequency_hz: generated.butterfly_bob_frequency_hz,
        butterfly_bob_strength: generated.butterfly_bob_strength,
        butterfly_lifetime_min: generated.butterfly_lifetime_min,
        butterfly_lifetime_max: generated.butterfly_lifetime_max,

        voxel_dirt_color: generated.voxel_dirt_color,
        voxel_cherry_wood_color: generated.voxel_cherry_wood_color,
        voxel_oak_wood_color: generated.voxel_oak_wood_color,
        voxel_color_variance: generated.voxel_color_variance,
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
        generated::get_float_param(adjustables, id)
    }

    #[allow(dead_code)]
    fn get_int_param<'a>(
        adjustables: &'a GuiAdjustables,
        id: &str,
    ) -> Option<&'a crate::gui_adjustables::IntParam> {
        generated::get_int_param(adjustables, id)
    }

    #[allow(dead_code)]
    fn get_uint_param<'a>(
        adjustables: &'a GuiAdjustables,
        id: &str,
    ) -> Option<&'a crate::gui_adjustables::UintParam> {
        generated::get_uint_param(adjustables, id)
    }

    #[allow(dead_code)]
    fn get_bool_param<'a>(
        adjustables: &'a GuiAdjustables,
        id: &str,
    ) -> Option<&'a crate::gui_adjustables::BoolParam> {
        generated::get_bool_param(adjustables, id)
    }

    #[allow(dead_code)]
    fn get_color_param<'a>(
        adjustables: &'a GuiAdjustables,
        id: &str,
    ) -> Option<&'a crate::gui_adjustables::ColorParam> {
        generated::get_color_param(adjustables, id)
    }

    #[allow(dead_code)]
    pub fn get_float_param_mut<'a>(
        adjustables: &'a mut GuiAdjustables,
        id: &str,
    ) -> Option<&'a mut crate::gui_adjustables::FloatParam> {
        generated::get_float_param_mut(adjustables, id)
    }

    #[allow(dead_code)]
    pub fn get_int_param_mut<'a>(
        adjustables: &'a mut GuiAdjustables,
        id: &str,
    ) -> Option<&'a mut crate::gui_adjustables::IntParam> {
        generated::get_int_param_mut(adjustables, id)
    }

    #[allow(dead_code)]
    pub fn get_uint_param_mut<'a>(
        adjustables: &'a mut GuiAdjustables,
        id: &str,
    ) -> Option<&'a mut crate::gui_adjustables::UintParam> {
        generated::get_uint_param_mut(adjustables, id)
    }

    #[allow(dead_code)]
    pub fn get_bool_param_mut<'a>(
        adjustables: &'a mut GuiAdjustables,
        id: &str,
    ) -> Option<&'a mut crate::gui_adjustables::BoolParam> {
        generated::get_bool_param_mut(adjustables, id)
    }

    #[allow(dead_code)]
    pub fn get_color_param_mut<'a>(
        adjustables: &'a mut GuiAdjustables,
        id: &str,
    ) -> Option<&'a mut crate::gui_adjustables::ColorParam> {
        generated::get_color_param_mut(adjustables, id)
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
