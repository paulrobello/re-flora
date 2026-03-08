/// GUI Adjustables Configuration
///
/// This file contains the declarative definition of all GUI-adjustable parameters.
/// To add a new parameter, simply add one line in the appropriate section.
use crate::declare_gui_adjustables;
use egui::Color32;

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
        grass_bottom_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(55, 107, 0), color, "Bottom Color",
        grass_tip_color: crate::gui_adjustables::ColorParam = Color32::from_rgb(119, 176, 7), color, "Tip Color",
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
