#[allow(unused)]
use crate::util::Timer;

use crate::app::GuiAdjustables;
use crate::audio::{SpatialSoundManager, TreeAudioManager};
use crate::builder::{ContreeBuilder, PlainBuilder, SceneAccelBuilder, SurfaceBuilder};
use crate::flora::species;
use crate::geom::{build_bvh, UAabb3};
use crate::particles::{
    ButterflyEmitter, ButterflyEmitterDesc, FallenLeafEmitter, LeafEmitterDesc, ParticleEmitter,
    ParticleForces, ParticleSnapshot, ParticleSystem, PARTICLE_CAPACITY,
};
use crate::procedual_placer::{generate_positions, PlacerDesc};
use crate::tracer::{Tracer, TracerDesc};
use crate::tree_gen::{Tree, TreeDesc};
use crate::util::{cluster_positions, ClusterResult, TimeInfo, BENCH};
use crate::util::{get_sun_dir, ShaderCompiler};
use crate::vkn::{Allocator, CommandBuffer, Device, Fence, Semaphore, SwapchainDesc};
use crate::{
    egui_renderer::EguiRenderer,
    vkn::{Swapchain, VulkanContext, VulkanContextDesc},
    window::{WindowMode, WindowState, WindowStateDesc},
};
use anyhow::{Context, Result};
use ash::vk;
use egui::style::WidgetVisuals;
use egui::{Color32, FontData, FontDefinitions, FontFamily, RichText};
use glam::{UVec3, Vec2, Vec3, Vec4};
use gpu_allocator::vulkan::AllocatorCreateDesc;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use winit::event::DeviceEvent;
use winit::{
    event::{ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::KeyCode,
    window::WindowId,
};

const LEAF_CLUSTER_DISTANCE: f32 = 0.08;
const CUSTOM_GUI_FONT_PATH: Option<&str> = Some("assets/font/ark-pixel-font-12px-monospaced-ttf-v2025.10.20/ark-pixel-12px-monospaced-zh_cn.ttf");
const CUSTOM_GUI_FONT_NAME: &str = "re_flora_gui_font";

const PANEL_BG: Color32 = Color32::from_rgb(35, 40, 40);
const PANEL_LIGHT: Color32 = Color32::from_rgb(50, 58, 58);
const PANEL_DARK: Color32 = Color32::from_rgb(25, 28, 28);
const TEXT_COLOR: Color32 = Color32::from_rgb(235, 230, 215);
const GOLD_ACCENT: Color32 = Color32::from_rgb(235, 165, 60);
const FLOWER_ACCENT: Color32 = Color32::from_rgb(190, 160, 210);
const SAGE_ACCENT: Color32 = Color32::from_rgb(110, 140, 120);
const SHADOW_COLOR: Color32 = Color32::from_rgb(75, 60, 85);

#[derive(Debug, Clone)]
pub struct TreeVariationConfig {
    pub size_variance: f32,
    pub trunk_thickness_variance: f32,
    pub trunk_thickness_min_variance: f32,
    pub spread_variance: f32,
    pub randomness_variance: f32,
    pub vertical_tendency_variance: f32,
    pub branch_probability_variance: f32,
    pub leaves_size_level_variance: f32,
    pub iterations_variance: f32,
    pub tree_height_variance: f32,
    pub length_dropoff_variance: f32,
    pub thickness_reduction_variance: f32,
}

impl Default for TreeVariationConfig {
    fn default() -> Self {
        TreeVariationConfig {
            size_variance: 0.0,
            trunk_thickness_variance: 0.0,
            trunk_thickness_min_variance: 0.0,
            spread_variance: 0.0,
            randomness_variance: 0.0,
            vertical_tendency_variance: 0.0,
            branch_probability_variance: 0.0,
            leaves_size_level_variance: 0.0,
            iterations_variance: 0.0,
            tree_height_variance: 0.0,
            length_dropoff_variance: 0.0,
            thickness_reduction_variance: 0.0,
        }
    }
}

impl TreeVariationConfig {
    pub fn edit_by_gui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.heading("Variation Settings");

        changed |= ui
            .add(egui::Slider::new(&mut self.size_variance, 0.0..=1.0).text("Size Variance"))
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut self.trunk_thickness_variance, 0.0..=1.0)
                    .text("Thickness Variance"),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut self.trunk_thickness_min_variance, 0.0..=1.0)
                    .text("Min Thickness Variance"),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut self.iterations_variance, 0.0..=5.0)
                    .text("Iterations Variance"),
            )
            .changed();

        ui.separator();
        ui.heading("Shape Variation");

        changed |= ui
            .add(
                egui::Slider::new(&mut self.tree_height_variance, 0.0..=1.0)
                    .text("Height Variance"),
            )
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut self.spread_variance, 0.0..=1.0).text("Spread Variance"))
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut self.vertical_tendency_variance, 0.0..=1.0)
                    .text("Vertical Tendency Variance"),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut self.length_dropoff_variance, 0.0..=1.0)
                    .text("Length Dropoff Variance"),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut self.thickness_reduction_variance, 0.0..=1.0)
                    .text("Thickness Reduction Variance"),
            )
            .changed();

        ui.separator();
        ui.heading("Branching Variation");

        changed |= ui
            .add(
                egui::Slider::new(&mut self.branch_probability_variance, 0.0..=1.0)
                    .text("Branch Probability Variance"),
            )
            .changed();

        ui.separator();
        ui.heading("Detail Variation");

        changed |= ui
            .add(
                egui::Slider::new(&mut self.randomness_variance, 0.0..=1.0)
                    .text("Randomness Variance"),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut self.leaves_size_level_variance, 0.0..=5.0)
                    .text("Leaves Size Variance"),
            )
            .changed();

        changed
    }
}

#[derive(Clone, Copy, Debug)]
enum TreePlacement {
    /// Place the tree at the given horizontal position and query terrain height.
    Terrain(Vec2),
    /// Place the tree at an exact world position (height already resolved).
    World(Vec3),
}

#[derive(Clone, Copy, Debug, Default)]
struct TreeAddOptions {
    clean_before_add: bool,
    assign_new_id: bool,
}

impl TreeAddOptions {
    fn with_cleanup(mut self) -> Self {
        self.clean_before_add = true;
        self
    }

    fn with_new_id(mut self) -> Self {
        self.assign_new_id = true;
        self
    }
}

#[derive(Clone, Debug)]
struct TreeRecord {
    position: Vec3,
    bound: UAabb3,
}

struct TreeLeafEmitter {
    tree_id: u32,
    emitter: FallenLeafEmitter,
}

impl TreeLeafEmitter {
    fn new(tree_id: u32, emitter: FallenLeafEmitter) -> Self {
        Self { tree_id, emitter }
    }

    fn tree_id(&self) -> u32 {
        self.tree_id
    }
}

impl ParticleEmitter for TreeLeafEmitter {
    fn update(&mut self, system: &mut ParticleSystem, dt: f32, time: f32) {
        self.emitter.update(system, dt, time);
    }
}

struct FrameSync {
    image_available: Semaphore,
    fence: Fence,
    command_buffer: CommandBuffer,
}

pub struct App {
    egui_renderer: EguiRenderer,
    is_resize_pending: bool,
    swapchain: Swapchain,
    window_state: WindowState,
    frames_in_flight: Vec<FrameSync>,
    current_frame: usize,
    image_render_finished_semaphores: Vec<Semaphore>,
    images_in_flight: Vec<vk::Fence>,
    time_info: TimeInfo,
    accumulated_mouse_delta: Vec2,
    smoothed_mouse_delta: Vec2,

    tracer: Tracer,

    // builders
    plain_builder: PlainBuilder,
    surface_builder: SurfaceBuilder,
    contree_builder: ContreeBuilder,
    scene_accel_builder: SceneAccelBuilder,

    // gui adjustables
    gui_adjustables: GuiAdjustables,
    debug_tree_pos: Vec3,
    config_panel_visible: bool,
    is_fly_mode: bool,

    debug_tree_desc: TreeDesc,
    tree_variation_config: TreeVariationConfig,
    regenerate_trees_requested: bool,
    prev_bound: UAabb3,
    tree_records: HashMap<u32, TreeRecord>,

    // multi-tree management
    next_tree_id: u32,
    single_tree_id: u32, // ID for GUI single tree mode

    particle_system: ParticleSystem,
    leaf_emitters: Vec<TreeLeafEmitter>,
    tree_leaf_emitter_indices: HashMap<u32, Vec<usize>>,
    leaf_emitter_desc: LeafEmitterDesc,
    butterfly_emitters: Vec<ButterflyEmitter>,
    butterfly_emitter_desc: ButterflyEmitterDesc,
    particle_snapshots: Vec<ParticleSnapshot>,
    particle_snapshots_lod: Vec<ParticleSnapshot>,
    particle_forces: ParticleForces,

    // note: always keep the context to end, as it has to be destroyed last
    vulkan_ctx: VulkanContext,

    // Keep ownership so the shared PetalSonic engine outlives every subsystem.
    #[allow(dead_code)]
    spatial_sound_manager: SpatialSoundManager,
    tree_audio_manager: TreeAudioManager,
}

impl Drop for App {
    fn drop(&mut self) {
        // Ensure GPU work is done before resources begin destructing
        self.vulkan_ctx.device().wait_idle();
    }
}

const VOXEL_DIM_PER_CHUNK: UVec3 = UVec3::new(256, 256, 256);
const CHUNK_DIM: UVec3 = UVec3::new(5, 2, 5);
const FREE_ATLAS_DIM: UVec3 = UVec3::new(512, 512, 512);
const MAX_FRAMES_IN_FLIGHT: usize = 1;

impl App {
    pub fn new(_event_loop: &ActiveEventLoop) -> Result<Self> {
        let chunk_bound = UAabb3::new(UVec3::ZERO, CHUNK_DIM);
        let window_state = Self::create_window_state(_event_loop);
        let vulkan_ctx = Self::create_vulkan_context(&window_state);

        let shader_compiler = ShaderCompiler::new().unwrap();

        let device = vulkan_ctx.device();

        let gpu_allocator = {
            let allocator_create_info = AllocatorCreateDesc {
                instance: vulkan_ctx.instance().as_raw().clone(),
                device: device.as_raw().clone(),
                physical_device: vulkan_ctx.physical_device().as_raw(),
                debug_settings: Default::default(),
                buffer_device_address: true,
                allocation_sizes: Default::default(),
            };
            gpu_allocator::vulkan::Allocator::new(&allocator_create_info)
                .expect("Failed to create gpu allocator")
        };
        let allocator = Allocator::new(device, Arc::new(Mutex::new(gpu_allocator)));

        let swapchain = Swapchain::new(
            vulkan_ctx.clone(),
            window_state.window_extent(),
            SwapchainDesc {
                present_mode: vk::PresentModeKHR::MAILBOX,
                ..Default::default()
            },
        );

        let frames_in_flight = (0..MAX_FRAMES_IN_FLIGHT)
            .map(|_| FrameSync {
                image_available: Semaphore::new(device),
                fence: Fence::new(device, true),
                command_buffer: CommandBuffer::new(device, vulkan_ctx.command_pool()),
            })
            .collect::<Vec<_>>();
        let current_frame = 0;
        let swapchain_image_count = swapchain.image_count();
        let (image_render_finished_semaphores, images_in_flight) =
            Self::create_swapchain_image_syncs(device, swapchain_image_count);

        let renderer = EguiRenderer::new(
            vulkan_ctx.clone(),
            &window_state.window(),
            allocator.clone(),
            &shader_compiler,
            swapchain.get_render_pass(),
        );

        let mut plain_builder = PlainBuilder::new(
            vulkan_ctx.clone(),
            &shader_compiler,
            allocator.clone(),
            CHUNK_DIM * VOXEL_DIM_PER_CHUNK,
            FREE_ATLAS_DIM,
        );

        let mut surface_builder = SurfaceBuilder::new(
            vulkan_ctx.clone(),
            allocator.clone(),
            &shader_compiler,
            plain_builder.get_resources(),
            VOXEL_DIM_PER_CHUNK,
            chunk_bound,
        );

        let mut contree_builder = ContreeBuilder::new(
            vulkan_ctx.clone(),
            allocator.clone(),
            &shader_compiler,
            surface_builder.get_resources(),
            VOXEL_DIM_PER_CHUNK,
            512 * 1024 * 1024, // node buffer pool size
            512 * 1024 * 1024, // leaf buffer pool size
        );

        let mut scene_accel_builder = SceneAccelBuilder::new(
            vulkan_ctx.clone(),
            allocator.clone(),
            &shader_compiler,
            chunk_bound,
        )?;

        Self::init(
            &mut plain_builder,
            &mut surface_builder,
            &mut contree_builder,
            &mut scene_accel_builder,
        )?;

        // Shared spatial audio engine (PetalSonic) used by both the tracer (camera)
        // and the app-level tree ambience sources.
        let spatial_sound_manager = SpatialSoundManager::new(1024)?;
        let tree_audio_manager = TreeAudioManager::new(spatial_sound_manager.clone());

        let tracer = Tracer::new(
            vulkan_ctx.clone(),
            allocator.clone(),
            &shader_compiler,
            chunk_bound,
            window_state.window_extent(),
            contree_builder.get_resources(),
            scene_accel_builder.get_resources(),
            TracerDesc {
                scaling_factor: 0.5,
            },
            spatial_sound_manager.clone(),
        )?;

        let debug_tree_pos = Vec3::new(2.0, 0.2, 2.0);
        let gui_adjustables = GuiAdjustables::default();

        let color_to_vec4 = |color: Color32| -> Vec4 {
            Vec4::new(
                color.r() as f32 / 255.0,
                color.g() as f32 / 255.0,
                color.b() as f32 / 255.0,
                1.0,
            )
        };

        let particle_system = ParticleSystem::new(PARTICLE_CAPACITY);
        let leaf_emitters = Vec::new();
        let tree_leaf_emitter_indices = HashMap::new();
        let leaf_emitter_desc = LeafEmitterDesc {
            color_low: color_to_vec4(gui_adjustables.leaves_bottom_color.value),
            color_high: color_to_vec4(gui_adjustables.leaves_tip_color.value),
            ..LeafEmitterDesc::default()
        };
        let butterfly_emitters = Vec::new();
        let butterfly_emitter_desc = ButterflyEmitterDesc::default();
        let particle_snapshots = Vec::with_capacity(PARTICLE_CAPACITY);
        let particle_snapshots_lod = Vec::with_capacity(PARTICLE_CAPACITY);
        let particle_forces = ParticleForces {
            linear_damping: 0.08,
            ..ParticleForces::default()
        };

        let mut app = Self {
            vulkan_ctx,
            egui_renderer: renderer,
            window_state,

            accumulated_mouse_delta: Vec2::ZERO,
            smoothed_mouse_delta: Vec2::ZERO,

            swapchain,
            frames_in_flight,
            current_frame,
            image_render_finished_semaphores,
            images_in_flight,

            tracer,

            plain_builder,
            surface_builder,
            contree_builder,
            scene_accel_builder,

            is_resize_pending: false,
            time_info: TimeInfo::default(),

            gui_adjustables,
            debug_tree_pos,
            debug_tree_desc: TreeDesc::default(),
            tree_variation_config: TreeVariationConfig::default(),
            regenerate_trees_requested: false,
            prev_bound: Default::default(),
            tree_records: HashMap::new(),
            config_panel_visible: false,
            is_fly_mode: true,

            // multi-tree management
            next_tree_id: 1, // Start from 1, use 0 for GUI single tree
            single_tree_id: 0,

            particle_system,
            leaf_emitters,
            tree_leaf_emitter_indices,
            leaf_emitter_desc,
            butterfly_emitters,
            butterfly_emitter_desc,
            particle_snapshots,
            particle_snapshots_lod,
            particle_forces,

            spatial_sound_manager,
            tree_audio_manager,
        };

        app.configure_gui_font()?;
        app.ensure_map_butterfly_emitter();

        app.add_tree(
            app.debug_tree_desc.clone(),
            TreePlacement::Terrain(Vec2::new(app.debug_tree_pos.x, app.debug_tree_pos.z)),
            TreeAddOptions::default(),
        )?;

        // configure leaves with the app's actual density values (now that app struct exists)
        app.tracer.regenerate_leaves(
            app.gui_adjustables.leaves_inner_density.value,
            app.gui_adjustables.leaves_outer_density.value,
            app.gui_adjustables.leaves_inner_radius.value,
            app.gui_adjustables.leaves_outer_radius.value,
        )?;

        Ok(app)
    }

    fn configure_gui_font(&mut self) -> Result<()> {
        if let Some(font_path) = CUSTOM_GUI_FONT_PATH {
            let font_bytes = std::fs::read(font_path)
                .with_context(|| format!("Failed to read GUI font from {font_path}"))?;
            let ctx = self.egui_renderer.context();

            let mut fonts = FontDefinitions::default();
            fonts.font_data.insert(
                CUSTOM_GUI_FONT_NAME.to_owned(),
                FontData::from_owned(font_bytes).into(),
            );

            if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
                family.insert(0, CUSTOM_GUI_FONT_NAME.to_owned());
            }

            if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
                family.insert(0, CUSTOM_GUI_FONT_NAME.to_owned());
            }

            ctx.set_fonts(fonts);
            log::info!("Loaded custom GUI font from {}", font_path);
        }

        Ok(())
    }

    fn apply_gui_style(style: &mut egui::Style) {
        // --- GENERAL VISUALS ---
        style.visuals.override_text_color = Some(TEXT_COLOR);
        style.visuals.hyperlink_color = GOLD_ACCENT;

        // Selection (Text highlighting)
        style.visuals.selection.bg_fill = FLOWER_ACCENT.linear_multiply(0.4);
        style.visuals.selection.stroke = egui::Stroke::new(1.0, GOLD_ACCENT);

        // Window/Panel Backgrounds
        style.visuals.window_fill = PANEL_BG;
        style.visuals.panel_fill = PANEL_BG;

        // Input fields and deep backgrounds
        style.visuals.extreme_bg_color = PANEL_DARK;
        style.visuals.code_bg_color = PANEL_DARK;
        style.visuals.text_edit_bg_color = Some(PANEL_DARK);
        style.visuals.faint_bg_color = PANEL_DARK;

        // Pixel-art UI: keep panel corners square
        style.visuals.window_corner_radius = egui::CornerRadius::same(0);
        style.visuals.menu_corner_radius = egui::CornerRadius::same(0);

        // Border: Thinner and Earthy Green instead of Neon Cyan
        style.visuals.window_stroke = egui::Stroke::new(1.5, SAGE_ACCENT);

        // Shadows: Keep them to separate UI from the 3D world
        style.visuals.popup_shadow = egui::epaint::Shadow {
            offset: [4, 4],
            blur: 10, // Increased blur for a softer shadow
            spread: 0,
            color: SHADOW_COLOR,
        };
        style.visuals.window_shadow = egui::epaint::Shadow {
            offset: [6, 6],
            blur: 12,
            spread: 0,
            color: SHADOW_COLOR,
        };

        style.visuals.window_highlight_topmost = false;
        style.visuals.button_frame = true;
        style.visuals.collapsing_header_frame = true;
        style.visuals.slider_trailing_fill = true;

        // Make handles slightly rounder/softer
        style.visuals.handle_shape = egui::style::HandleShape::Rect { aspect_ratio: 0.6 };

        // --- SPACING ---
        style.spacing.item_spacing = egui::Vec2::new(10.0, 8.0);
        style.spacing.button_padding = egui::Vec2::new(10.0, 6.0);
        style.spacing.window_margin = egui::Margin::symmetric(14, 14);
        style.spacing.menu_margin = egui::Margin::symmetric(10, 8);
        style.spacing.indent = 20.0; // Slightly more indentation for hierarchy
        style.spacing.interact_size = egui::Vec2::new(40.0, 24.0); // Wider sliders
        style.spacing.slider_width = 200.0;
        style.spacing.icon_spacing = 8.0;

        // Scrollbars
        style.spacing.scroll.floating = true;
        style.spacing.scroll.bar_width = 8.0;
        style.spacing.scroll.floating_width = 4.0;
        style.spacing.scroll.foreground_color = true;
        style.spacing.scroll.dormant_background_opacity = 0.0;
        style.spacing.scroll.active_background_opacity = 0.4;
        style.spacing.scroll.interact_background_opacity = 0.6;
        style.spacing.scroll.dormant_handle_opacity = 0.6;
        style.spacing.scroll.active_handle_opacity = 0.9;
        style.spacing.scroll.interact_handle_opacity = 1.0;

        // --- WIDGET STATES ---

        // Non-interactive (Labels, etc)
        style.visuals.widgets.noninteractive = Self::widget_visuals(
            Color32::TRANSPARENT, // Transparent background for labels
            Color32::TRANSPARENT,
            SAGE_ACCENT, // Border color for separators
            TEXT_COLOR,
            1.0, // Thinner stroke
        );

        // Inactive (Buttons, Sliders not hovered)
        style.visuals.widgets.inactive = Self::widget_visuals(
            PANEL_LIGHT, // Slightly lighter than BG
            PANEL_LIGHT,
            Color32::TRANSPARENT, // No border when inactive for a cleaner look
            TEXT_COLOR,
            0.0,
        );

        // Hovered
        style.visuals.widgets.hovered = Self::widget_visuals(
            Color32::from_rgb(65, 75, 75), // Lighten up
            Color32::from_rgb(65, 75, 75),
            FLOWER_ACCENT, // Lavender border on hover
            GOLD_ACCENT,   // Text turns Gold on hover
            1.5,
        );

        // Active (Clicked / Dragging)
        style.visuals.widgets.active = Self::widget_visuals(
            GOLD_ACCENT, // Fill with Gold
            GOLD_ACCENT,
            GOLD_ACCENT,
            Color32::from_rgb(30, 35, 30), // Dark text on Gold background
            1.0,
        );

        // Open (Menu / Combo box open)
        style.visuals.widgets.open =
            Self::widget_visuals(PANEL_LIGHT, PANEL_LIGHT, GOLD_ACCENT, TEXT_COLOR, 1.5);
    }

    fn widget_visuals(
        bg_fill: Color32,
        weak_bg_fill: Color32,
        stroke_color: Color32,
        text_color: Color32,
        stroke_width: f32,
    ) -> WidgetVisuals {
        WidgetVisuals {
            bg_fill,
            weak_bg_fill,
            bg_stroke: egui::Stroke::new(stroke_width, stroke_color),
            corner_radius: egui::CornerRadius::same(4), // Slightly rounded widgets
            fg_stroke: egui::Stroke::new(1.5, text_color),
            expansion: 0.0,
        }
    }

    fn generate_procedural_trees(&mut self) -> Result<()> {
        // clear all procedural trees (keep single tree with ID 0)
        self.clear_procedural_trees()?;
        // remove the standalone debug tree so only procedural forest remains
        self.remove_tree(self.single_tree_id)?;

        self.plain_builder.chunk_init(
            self.prev_bound.min(),
            self.prev_bound.max() - self.prev_bound.min(),
        )?;

        let world_size = CHUNK_DIM * VOXEL_DIM_PER_CHUNK;
        let map_padding = 50.0;
        let map_dimensions = Vec2::new(
            world_size.x as f32 - map_padding * 2.0,
            world_size.z as f32 - map_padding * 2.0,
        );
        let grid_size = 120.0;
        let mut placer_desc = PlacerDesc::new(42);
        placer_desc.threshold = 0.55;

        let tree_positions_2d = generate_positions(
            map_dimensions,
            Vec2::new(map_padding, map_padding),
            grid_size,
            &placer_desc,
        );

        log::info!("Generated {} procedural trees", tree_positions_2d.len());

        // batch query all terrain heights at once
        let tree_positions_3d = self.query_terrain_heights_for_positions(&tree_positions_2d)?;

        let mut rng = rand::rng();

        // plant all trees with known heights and unique IDs
        for tree_pos in tree_positions_3d.iter() {
            let mut tree_desc = self.debug_tree_desc.clone();
            tree_desc.seed = rng.random_range(1..10000);

            self.apply_tree_variations(&mut tree_desc, &mut rng);
            self.add_tree(
                tree_desc,
                TreePlacement::World(*tree_pos),
                TreeAddOptions::default().with_new_id(),
            )?;
        }

        Ok(())
    }

    fn clear_procedural_trees(&mut self) -> Result<()> {
        // remove all procedural tree leaves (IDs >= 1), keep single tree (ID 0)
        let tree_ids_to_remove: Vec<u32> = self
            .tree_records
            .keys()
            .copied()
            .filter(|&id| id >= 1)
            .collect();

        for tree_id in tree_ids_to_remove {
            self.remove_tree(tree_id)?;
        }

        log::info!("Cleared all procedural trees and their sound sources");
        Ok(())
    }

    fn remove_tree(&mut self, tree_id: u32) -> Result<()> {
        self.tracer
            .remove_tree_leaves(&mut self.surface_builder.resources, tree_id)?;
        self.tree_audio_manager.remove_tree(tree_id);
        self.remove_leaf_emitter(tree_id);
        match self.tree_records.remove(&tree_id) {
            Some(record) => {
                log::debug!(
                    "Removed tree {} at position {:?}, bound {:?}",
                    tree_id,
                    record.position,
                    record.bound
                );
            }
            None => {
                log::debug!("Tree {} was not registered during removal", tree_id);
            }
        }
        Ok(())
    }

    fn edit_tree_with_variance(
        tree_desc: &mut TreeDesc,
        tree_variation_config: &mut TreeVariationConfig,
        ui: &mut egui::Ui,
    ) -> (bool, bool) {
        let mut regenerate_pressed = false;

        if ui.button("🌲 Regenerate Procedural Trees").clicked() {
            regenerate_pressed = true;
        }

        ui.separator();

        let tree_changed = tree_desc.edit_by_gui(ui);

        ui.separator();

        tree_variation_config.edit_by_gui(ui);

        (tree_changed, regenerate_pressed)
    }

    fn calculate_sun_position(&mut self, time_of_day: f32, latitude: f32, season: f32) {
        use std::f32::consts::PI;

        // time of day: 0.0 = midnight, 0.5 = noon, 1.0 = midnight
        // latitude: -1.0 = south pole, 0.0 = equator, 1.0 = north pole
        // season: 0.0 = winter solstice, 0.25 = spring equinox, 0.5 = summer solstice, 0.75 = autumn equinox

        // convert time to hour angle (radians)
        // solar noon is at time_of_day = 0.5
        let hour_angle = (time_of_day - 0.5) * 2.0 * PI;

        // solar declination based on season
        // season of 0.0 = winter solstice (max negative declination)
        // season of 0.5 = summer solstice (max positive declination)
        let seasonal_angle = season * 2.0 * PI;
        let declination = -23.44_f32.to_radians() * (seasonal_angle).cos(); // Earth's axial tilt

        // calculate solar elevation (altitude)
        let elevation = (declination.sin() * (latitude * PI * 0.5).sin()
            + declination.cos() * (latitude * PI * 0.5).cos() * hour_angle.cos())
        .asin();

        // calculate solar azimuth
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

        // normalize elevation to -1.0 to 1.0 range (matching current altitude range)
        self.gui_adjustables.sun_altitude.value = (elevation / (PI * 0.5)).clamp(-1.0, 1.0);

        // normalize azimuth to 0.0 to 1.0 range (matching current azimuth range)
        self.gui_adjustables.sun_azimuth.value = ((azimuth + PI) / (2.0 * PI)) % 1.0;
    }

    fn apply_tree_variations(&self, tree_desc: &mut TreeDesc, rng: &mut impl Rng) {
        let config = &self.tree_variation_config;

        if config.size_variance > 0.0 {
            tree_desc.size *= 1.0 + rng.random_range(-config.size_variance..=config.size_variance);
        }

        if config.trunk_thickness_variance > 0.0 {
            tree_desc.trunk_thickness *= 1.0
                + rng.random_range(
                    -config.trunk_thickness_variance..=config.trunk_thickness_variance,
                );
        }

        if config.trunk_thickness_min_variance > 0.0 {
            tree_desc.trunk_thickness_min *= 1.0
                + rng.random_range(
                    -config.trunk_thickness_min_variance..=config.trunk_thickness_min_variance,
                );
        }

        if config.spread_variance > 0.0 {
            tree_desc.spread *=
                1.0 + rng.random_range(-config.spread_variance..=config.spread_variance);
        }

        if config.randomness_variance > 0.0 {
            tree_desc.randomness = (tree_desc.randomness
                + rng.random_range(-config.randomness_variance..=config.randomness_variance))
            .clamp(0.0, 1.0);
        }

        if config.vertical_tendency_variance > 0.0 {
            tree_desc.vertical_tendency = (tree_desc.vertical_tendency
                + rng.random_range(
                    -config.vertical_tendency_variance..=config.vertical_tendency_variance,
                ))
            .clamp(-1.0, 1.0);
        }

        if config.branch_probability_variance > 0.0 {
            tree_desc.branch_probability = (tree_desc.branch_probability
                + rng.random_range(
                    -config.branch_probability_variance..=config.branch_probability_variance,
                ))
            .clamp(0.0, 1.0);
        }

        if config.tree_height_variance > 0.0 {
            tree_desc.tree_height *=
                1.0 + rng.random_range(-config.tree_height_variance..=config.tree_height_variance);
        }

        if config.length_dropoff_variance > 0.0 {
            tree_desc.length_dropoff = (tree_desc.length_dropoff
                + rng.random_range(
                    -config.length_dropoff_variance..=config.length_dropoff_variance,
                ))
            .clamp(0.1, 1.0);
        }

        if config.thickness_reduction_variance > 0.0 {
            tree_desc.thickness_reduction = (tree_desc.thickness_reduction
                + rng.random_range(
                    -config.thickness_reduction_variance..=config.thickness_reduction_variance,
                ))
            .clamp(0.0, 1.0);
        }

        if config.iterations_variance > 0.0 {
            let variation =
                rng.random_range(-config.iterations_variance..=config.iterations_variance);
            tree_desc.iterations =
                ((tree_desc.iterations as f32 + variation).round() as u32).clamp(1, 12);
        }

        if config.leaves_size_level_variance > 0.0 {
            let variation = rng.random_range(
                -config.leaves_size_level_variance..=config.leaves_size_level_variance,
            );
            tree_desc.leaves_size_level =
                ((tree_desc.leaves_size_level as f32 + variation).round() as u32).clamp(0, 8);
        }
    }

    fn init(
        plain_builder: &mut PlainBuilder,
        surface_builder: &mut SurfaceBuilder,
        contree_builder: &mut ContreeBuilder,
        scene_accel_builder: &mut SceneAccelBuilder,
    ) -> Result<()> {
        plain_builder.chunk_init(UVec3::new(0, 0, 0), VOXEL_DIM_PER_CHUNK * CHUNK_DIM)?;

        let chunk_pos_to_build_min = UVec3::new(0, 0, 0);
        let chunk_pos_to_build_max = CHUNK_DIM;

        for x in chunk_pos_to_build_min.x..chunk_pos_to_build_max.x {
            for y in chunk_pos_to_build_min.y..chunk_pos_to_build_max.y {
                for z in chunk_pos_to_build_min.z..chunk_pos_to_build_max.z {
                    let chunk_idx = UVec3::new(x, y, z);
                    let this_bound = UAabb3::new(
                        chunk_idx * VOXEL_DIM_PER_CHUNK,
                        (chunk_idx + UVec3::ONE) * VOXEL_DIM_PER_CHUNK - UVec3::ONE,
                    );
                    Self::mesh_generate(
                        surface_builder,
                        contree_builder,
                        scene_accel_builder,
                        this_bound,
                    )?;
                }
            }
        }

        BENCH.lock().unwrap().summary();
        Ok(())
    }

    fn create_window_state(event_loop: &ActiveEventLoop) -> WindowState {
        const WINDOW_TITLE_DEBUG: &str = "Re: Flora - debug build";
        const WINDOW_TITLE_RELEASE: &str = "Re: Flora - release build";
        let using_mode = if cfg!(debug_assertions) {
            WINDOW_TITLE_DEBUG
        } else {
            WINDOW_TITLE_RELEASE
        };
        let window_descriptor = WindowStateDesc {
            title: using_mode.to_owned(),
            window_mode: WindowMode::BorderlessFullscreen,
            cursor_locked: true,
            cursor_visible: false,
            ..Default::default()
        };
        WindowState::new(event_loop, &window_descriptor)
    }

    fn create_vulkan_context(window_state: &WindowState) -> VulkanContext {
        VulkanContext::new(
            &window_state.window(),
            VulkanContextDesc {
                name: "Re: Flora".into(),
            },
        )
    }

    pub fn on_terminate(&mut self, event_loop: &ActiveEventLoop) {
        // ensure all command buffers are done executing before terminating anything
        self.vulkan_ctx.device().wait_idle();
        event_loop.exit();
    }

    fn query_terrain_heights_for_positions(&mut self, positions_2d: &[Vec2]) -> Result<Vec<Vec3>> {
        if positions_2d.is_empty() {
            return Ok(vec![]);
        }

        let query_positions: Vec<Vec2> = positions_2d
            .iter()
            .map(|pos| Vec2::new(pos.x, pos.y))
            .collect();

        // batch query all terrain heights
        let terrain_heights = self.tracer.query_terrain_heights_batch(&query_positions)?;

        // convert back to world coordinates and create Vec3s
        let positions_3d = positions_2d
            .iter()
            .zip(terrain_heights.iter())
            .map(|(pos_2d, &height)| Vec3::new(pos_2d.x, height, pos_2d.y))
            .collect();

        Ok(positions_3d)
    }

    /// If we need to clean up first, do it before querying terrain to avoid
    /// getting the wrong height due to existing tree geometry blocking the terrain query
    fn clean_up_prev_tree(&mut self) -> Result<()> {
        // Remove any previously created spatial audio sources for trees so we
        // don't leak looping sounds when rebuilding the tree geometry.
        self.tree_audio_manager.remove_all();

        self.plain_builder.chunk_init(
            self.prev_bound.min(),
            self.prev_bound.max() - self.prev_bound.min(),
        )?;

        // force mesh regeneration after cleanup to ensure terrain is properly accessible for querying
        Self::mesh_generate(
            &mut self.surface_builder,
            &mut self.contree_builder,
            &mut self.scene_accel_builder,
            self.prev_bound,
        )?;

        Ok(())
    }

    fn add_tree(
        &mut self,
        tree_desc: TreeDesc,
        placement: TreePlacement,
        options: TreeAddOptions,
    ) -> Result<()> {
        if options.clean_before_add {
            self.clean_up_prev_tree()?;
        }

        let tree_pos = match placement {
            TreePlacement::Terrain(horizontal) => {
                let terrain_height = self
                    .tracer
                    .query_terrain_height(Vec2::new(horizontal.x, horizontal.y))?;
                Vec3::new(horizontal.x, terrain_height, horizontal.y)
            }
            TreePlacement::World(world) => world,
        };

        let tree_id = if options.assign_new_id {
            let current_id = self.next_tree_id;
            self.next_tree_id += 1;
            current_id
        } else {
            self.single_tree_id
        };

        let tree = Tree::new(tree_desc);
        let mut round_cones = Vec::with_capacity(tree.trunks().len());
        for tree_trunk in tree.trunks() {
            let mut round_cone = tree_trunk.clone();
            round_cone.transform(tree_pos * 256.0);
            round_cones.push(round_cone);
        }

        let mut leaves_data_sequential = vec![0; round_cones.len()];
        for (i, item) in leaves_data_sequential.iter_mut().enumerate() {
            *item = i as u32;
        }

        let mut aabbs = Vec::with_capacity(round_cones.len());
        for round_cone in &round_cones {
            aabbs.push(round_cone.aabb());
        }

        let bvh_nodes = build_bvh(&aabbs, &leaves_data_sequential).unwrap();
        let this_bound = UAabb3::new(bvh_nodes[0].aabb.min_uvec3(), bvh_nodes[0].aabb.max_uvec3());

        self.plain_builder.chunk_modify(&bvh_nodes, &round_cones)?;

        let relative_leaf_positions = tree.relative_leaf_positions();
        let world_leaf_positions = relative_leaf_positions
            .iter()
            .map(|leaf_pos| *leaf_pos / 256.0 + tree_pos)
            .collect::<Vec<_>>();
        let offseted_leaf_positions = relative_leaf_positions
            .iter()
            .map(|leaf_pos| *leaf_pos + tree_pos * 256.0)
            .collect::<Vec<_>>();

        let quantized_leaf_positions = {
            let set = offseted_leaf_positions
                .iter()
                .map(|pos| pos.as_uvec3())
                .collect::<HashSet<_>>();
            set.into_iter().collect::<Vec<_>>()
        };

        self.tracer.add_tree_leaves(
            &mut self.surface_builder.resources,
            tree_id,
            &quantized_leaf_positions,
        )?;

        Self::mesh_generate(
            &mut self.surface_builder,
            &mut self.contree_builder,
            &mut self.scene_accel_builder,
            this_bound.union_with(&self.prev_bound),
        )?;

        self.prev_bound = this_bound.union_with(&self.prev_bound);

        // Cluster leaf positions once for both audio and particle systems
        let leaf_clusters = cluster_positions(&world_leaf_positions, LEAF_CLUSTER_DISTANCE);

        self.tree_audio_manager.add_tree_sources_from_clusters(
            tree_id,
            tree_pos,
            &leaf_clusters,
            false,
            true,
        )?;

        self.tree_records.insert(
            tree_id,
            TreeRecord {
                position: tree_pos,
                bound: this_bound,
            },
        );

        self.upsert_tree_leaf_emitter(tree_id, tree_pos, &this_bound, &leaf_clusters);

        Ok(())
    }

    fn upsert_tree_leaf_emitter(
        &mut self,
        tree_id: u32,
        tree_pos: Vec3,
        bound: &UAabb3,
        clusters: &[ClusterResult],
    ) {
        // Remove existing emitters for this tree first
        self.remove_leaf_emitter(tree_id);

        if clusters.is_empty() {
            return;
        }

        let mut emitter_indices = Vec::with_capacity(clusters.len());

        for cluster in clusters {
            // Create extent based on the tree bound
            let (_center, _extent) = Self::compute_leaf_emitter_region(tree_pos, bound);

            // Use cluster position as the emitter center
            let cluster_center = cluster.pos;

            // Create emitter with cluster-specific seed and tree leaf colors
            let mut emitter = FallenLeafEmitter::new(
                cluster_center,
                Vec::new(), // We'll spawn from cluster center, not specific leaf positions
                tree_id as u64 + cluster.pos.x as u64 + cluster.pos.y as u64 + cluster.pos.z as u64,
                &self.leaf_emitter_desc,
            );

            // Scale spawn rate by cluster size (more leaves = more particles)
            // Base rate is divided by expected average cluster size, then scaled by actual size
            let cluster_size_multiplier = (cluster.items_count as f32).sqrt();
            emitter.spawn_rate = self.leaf_emitter_desc.spawn_rate * cluster_size_multiplier;

            let idx = self.leaf_emitters.len();
            self.leaf_emitters
                .push(TreeLeafEmitter::new(tree_id, emitter));
            emitter_indices.push(idx);
        }

        self.tree_leaf_emitter_indices
            .insert(tree_id, emitter_indices);
    }

    fn compute_leaf_emitter_region(tree_pos: Vec3, bound: &UAabb3) -> (Vec3, Vec3) {
        if bound.min() == bound.max() {
            return (
                tree_pos + Vec3::new(0.5, 1.3, 0.5),
                Vec3::new(2.0, 0.5, 2.0),
            );
        }

        let min = bound.min().as_vec3() / 256.0;
        let max = bound.max().as_vec3() / 256.0;
        let size = max - min;
        let center = min + size * 0.5;
        let extent = Vec3::new(
            (size.x * 0.5).max(0.75),
            (size.y * 0.25).max(0.5),
            (size.z * 0.5).max(0.75),
        );

        (center, extent)
    }

    fn remove_leaf_emitter(&mut self, tree_id: u32) {
        if let Some(indices) = self.tree_leaf_emitter_indices.remove(&tree_id) {
            // Remove all emitters for this tree, starting from the highest index to avoid index shifts
            let mut sorted_indices = indices;
            sorted_indices.sort_unstable_by(|a, b| b.cmp(a)); // Sort in descending order

            for index in sorted_indices {
                self.leaf_emitters.swap_remove(index);
                // Update the index map for any tree whose emitter was swapped
                if let Some(swapped) = self.leaf_emitters.get(index) {
                    if let Some(tree_indices) =
                        self.tree_leaf_emitter_indices.get_mut(&swapped.tree_id())
                    {
                        // Find and update the old index to the new index
                        if let Some(pos) = tree_indices
                            .iter()
                            .position(|&i| i == self.leaf_emitters.len())
                        {
                            tree_indices[pos] = index;
                        }
                    }
                }
            }
        }
    }

    fn ensure_map_butterfly_emitter(&mut self) {
        if !self.butterfly_emitters.is_empty() {
            return;
        }

        let (center, extent) = Self::map_butterfly_region();
        self.butterfly_emitters.push(ButterflyEmitter::new(
            center,
            extent,
            9_173,
            &self.butterfly_emitter_desc,
        ));
    }

    fn map_butterfly_region() -> (Vec3, Vec3) {
        let map_size = CHUNK_DIM.as_vec3();
        let center = Vec3::new(map_size.x * 0.5, 0.5, map_size.z * 0.5);
        let extent = Vec3::new(
            (map_size.x * 0.5).max(1.0),
            0.6,
            (map_size.z * 0.5).max(1.0),
        );
        (center, extent)
    }

    fn update_particle_simulation(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }

        self.ensure_map_butterfly_emitter();
        let wind_time = self.time_info.time_since_start();
        Self::drive_emitters(
            &mut self.butterfly_emitters,
            &mut self.particle_system,
            dt,
            wind_time,
        );
        self.constrain_butterflies_to_terrain();
        Self::drive_emitters(
            &mut self.leaf_emitters,
            &mut self.particle_system,
            dt,
            wind_time,
        );

        self.particle_system.update(dt, self.particle_forces);
        self.particle_system
            .write_snapshots(&mut self.particle_snapshots);
        self.split_particle_lod(
            self.tracer.camera_position(),
            self.gui_adjustables.lod_distance.value.max(0.0),
        );

        if let Err(err) = self
            .tracer
            .upload_particles_lod(&self.particle_snapshots, &self.particle_snapshots_lod)
        {
            log::error!("Failed to upload particles: {}", err);
        }
    }

    fn split_particle_lod(&mut self, camera_pos: Vec3, lod_distance: f32) {
        self.particle_snapshots_lod.clear();
        if lod_distance <= 0.0 {
            self.particle_snapshots_lod
                .append(&mut self.particle_snapshots);
            return;
        }

        // Similar to flora LOD behavior: split into near and far sets.
        const FAR_KEEP_STRIDE: usize = 4;
        let mut near_snapshots = Vec::with_capacity(self.particle_snapshots.len());
        for (idx, snapshot) in self.particle_snapshots.drain(..).enumerate() {
            let world_pos = snapshot.position.as_vec3() / 256.0;
            let distance = (world_pos - camera_pos).length();
            if distance <= lod_distance {
                near_snapshots.push(snapshot);
                continue;
            }

            if idx % FAR_KEEP_STRIDE == 0 {
                self.particle_snapshots_lod.push(snapshot);
            }
        }

        self.particle_snapshots = near_snapshots;
    }

    fn constrain_butterflies_to_terrain(&mut self) {
        let mut query_positions_xz = Vec::new();
        let mut query_targets: Vec<ButterflyQueryTarget> = Vec::new();

        for (emitter_index, emitter) in self.butterfly_emitters.iter_mut().enumerate() {
            let mut emitter_positions_xz = Vec::new();
            let mut emitter_handles = Vec::new();
            emitter.collect_ground_queries(
                &self.particle_system,
                &mut emitter_positions_xz,
                &mut emitter_handles,
            );
            query_targets.extend(
                emitter_handles
                    .into_iter()
                    .zip(emitter_positions_xz.into_iter())
                    .map(|(handle, pos_xz)| {
                        query_positions_xz.push(pos_xz);
                        ButterflyQueryTarget {
                            emitter_index,
                            handle,
                        }
                    }),
            );
        }

        if query_targets.is_empty() {
            return;
        }

        let heights = match self.tracer.query_terrain_heights_batch(&query_positions_xz) {
            Ok(heights) => heights,
            Err(err) => {
                log::error!("Failed terrain query for butterflies: {}", err);
                return;
            }
        };

        for (target, ground_height) in query_targets.into_iter().zip(heights.into_iter()) {
            if let Some(emitter) = self.butterfly_emitters.get(target.emitter_index) {
                emitter.constrain_to_ground(
                    &mut self.particle_system,
                    target.handle,
                    ground_height,
                );
            }
        }
    }

    fn drive_emitters<E: ParticleEmitter>(
        emitters: &mut [E],
        particle_system: &mut ParticleSystem,
        dt: f32,
        time: f32,
    ) {
        for emitter in emitters {
            emitter.update(particle_system, dt, time);
        }
    }

    fn mesh_generate(
        surface_builder: &mut SurfaceBuilder,
        contree_builder: &mut ContreeBuilder,
        scene_accel_builder: &mut SceneAccelBuilder,
        bound: UAabb3,
    ) -> Result<()> {
        let affected_chunk_indices = get_affected_chunk_indices(bound.min(), bound.max());

        for chunk_id in affected_chunk_indices {
            let atlas_offset = chunk_id * VOXEL_DIM_PER_CHUNK;

            let now = Instant::now();
            let res = surface_builder.build_surface(chunk_id);
            if let Err(e) = res {
                log::error!("Failed to build surface for chunk {}: {}", chunk_id, e);
                continue;
            }
            // we don't use the active_voxel_len here

            BENCH.lock().unwrap().record("build_surface", now.elapsed());

            let now = Instant::now();
            let res = contree_builder.build_and_alloc(atlas_offset).unwrap();
            BENCH
                .lock()
                .unwrap()
                .record("build_and_alloc", now.elapsed());

            if let Some(res) = res {
                let (node_buffer_offset, leaf_buffer_offset) = res;
                scene_accel_builder.update_scene_tex(
                    chunk_id,
                    node_buffer_offset,
                    leaf_buffer_offset,
                )?;
            } else {
                log::debug!("Don't need to update scene tex because the chunk is empty");
            }
        }
        return Ok(());

        fn get_affected_chunk_indices(min_bound: UVec3, max_bound: UVec3) -> Vec<UVec3> {
            let min_chunk_idx = min_bound / VOXEL_DIM_PER_CHUNK;
            let max_chunk_idx = max_bound / VOXEL_DIM_PER_CHUNK;

            let mut affacted = Vec::new();
            for x in min_chunk_idx.x..=max_chunk_idx.x {
                for y in min_chunk_idx.y..=max_chunk_idx.y {
                    for z in min_chunk_idx.z..=max_chunk_idx.z {
                        affacted.push(UVec3::new(x, y, z));
                    }
                }
            }
            affacted
        }
    }

    pub fn on_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        // if cursor is visible, feed the event to gui first, if the event is being consumed by gui, no need to handle it again later
        if self.window_state.is_cursor_visible() {
            let consumed = self
                .egui_renderer
                .on_window_event(&self.window_state.window(), &event)
                .consumed;

            if consumed {
                return;
            }
        }

        match event {
            // close the loop, therefore the window, when close button is clicked
            WindowEvent::CloseRequested => {
                self.on_terminate(event_loop);
            }

            // never happened and never tested, take caution
            WindowEvent::ScaleFactorChanged {
                scale_factor: _scale_factor,
                inner_size_writer: _inner_size_writer,
            } => {
                self.is_resize_pending = true;
            }

            // resize the window
            WindowEvent::Resized(_) => {
                self.is_resize_pending = true;
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && event.physical_key == KeyCode::Escape {
                    self.on_terminate(event_loop);
                    return;
                }

                if event.state == ElementState::Pressed && event.physical_key == KeyCode::KeyE {
                    self.config_panel_visible = !self.config_panel_visible;
                    if self.config_panel_visible {
                        self.window_state.set_cursor_visibility(true);
                        self.window_state.set_cursor_grab(false);
                    } else {
                        self.window_state.set_cursor_visibility(false);
                        self.window_state.set_cursor_grab(true);
                    }
                }

                if event.state == ElementState::Pressed && event.physical_key == KeyCode::KeyF {
                    self.window_state.toggle_fullscreen();
                }

                if event.state == ElementState::Pressed && event.physical_key == KeyCode::KeyG {
                    let was_fly_mode = self.is_fly_mode;
                    self.is_fly_mode = !self.is_fly_mode;

                    // reset velocity when switching from fly mode to walk mode
                    if was_fly_mode && !self.is_fly_mode {
                        self.tracer.reset_camera_velocity();
                    }
                }

                if !self.window_state.is_cursor_visible() {
                    self.tracer.handle_keyboard(&event);
                }
            }

            // redraw the window
            WindowEvent::RedrawRequested => {
                // when the windiw is resized, redraw is called afterwards, so when the window is minimized, return
                if self.window_state.is_minimized() {
                    return;
                }

                // resize the window if needed
                if self.is_resize_pending {
                    self.on_resize();
                }

                self.window_state.maintain_cursor_grab();

                self.time_info.update();
                let frame_delta_time = self.time_info.delta_time();
                let time_since_start = self.time_info.time_since_start();
                if let Err(err) = self.tree_audio_manager.update(time_since_start) {
                    log::warn!("Failed to update tree audio sources: {}", err);
                }

                if !self.window_state.is_cursor_visible() {
                    // grab the value and immediately reset the accumulator
                    let mouse_delta = self.accumulated_mouse_delta;
                    self.accumulated_mouse_delta = Vec2::ZERO;

                    let alpha = 0.4; // mouse smoothing factor: 0 = no smoothing, 1 = infinite smoothing
                    self.smoothed_mouse_delta =
                        self.smoothed_mouse_delta * alpha + mouse_delta * (1.0 - alpha);

                    self.tracer.handle_mouse(self.smoothed_mouse_delta);
                }

                let mut tree_desc_changed = false;
                self.egui_renderer
                    .update(&self.window_state.window(), |ctx| {
                        let mut style = (*ctx.style()).clone();
                        Self::apply_gui_style(&mut style);
                        ctx.set_style(style);

                        let mut config_panel_open = self.config_panel_visible;
                        if config_panel_open {
                            let config_frame = egui::containers::Frame {
                                fill: PANEL_BG,
                                inner_margin: egui::Margin::symmetric(20, 16),
                                corner_radius: egui::CornerRadius::same(0),
                                shadow: egui::epaint::Shadow {
                                    offset: [6, 6],
                                    blur: 0,
                                    spread: 0,
                                    color: SHADOW_COLOR,
                                },
                                stroke: egui::Stroke::new(
                                    3.0,
                                    SAGE_ACCENT,
                                ),
                                ..Default::default()
                            };

                            let content_rect = ctx.content_rect();
                            let panel_pos =
                                egui::pos2(content_rect.left(), content_rect.top());
                            let panel_size = egui::Vec2::new(
                                content_rect.width() * 0.24,
                                content_rect.height() * 0.6,
                            );

                            egui::Window::new("Configuration")
                                .id(egui::Id::new("config_panel"))
                                .open(&mut config_panel_open)
                                .frame(config_frame)
                                .resizable(true)
                                .movable(true)
                                .default_pos(panel_pos)
                                .default_size(panel_size)
                                .show(ctx, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.heading(
                                            RichText::new("Scene Configuration")
                                                .size(18.0)
                                                .color(GOLD_ACCENT),
                                        );
                                    });

                                    ui.add_space(4.0);
                                    ui.separator();
                                    ui.add_space(4.0);

                                    egui::ScrollArea::vertical()
                                        .auto_shrink([false; 2])
                                        .show(ui, |ui| {
                                        ui.collapsing("Debug Settings", |ui| {
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.debug_float.value,
                                                    0.0..=10.0,
                                                )
                                                .text("Debug Float"),
                                            );
                                            ui.add(
                                                egui::Slider::new(&mut self.gui_adjustables.debug_uint.value, 0..=100)
                                                    .text("Debug UInt"),
                                            );
                                            ui.add(
                                                egui::Slider::new(&mut self.gui_adjustables.lod_distance.value, 0.0..=10.0)
                                                    .text("LOD Distance"),
                                            );
                                            ui.add(egui::Checkbox::new(
                                                &mut self.gui_adjustables.debug_bool.value,
                                                "Debug Bool",
                                            ));
                                        });


                                        ui.collapsing("Sky Settings", |ui| {
                                            ui.add(egui::Checkbox::new(
                                                &mut self.gui_adjustables.auto_daynight_cycle.value,
                                                "Auto Day/Night Cycle",
                                            ));

                                            if self.gui_adjustables.auto_daynight_cycle.value {
                                                ui.add(
                                                    egui::Slider::new(
                                                        &mut self.gui_adjustables.time_of_day.value,
                                                        0.0..=1.0,
                                                    )
                                                    .text("Time of Day (0:00 - 23:59)")
                                                    .custom_formatter(|n, _| {
                                                        let hour = (n * 24.0) as u32 % 24;
                                                        let minute = (n * 24.0 * 60.0) as u32 % 60;
                                                        format!("{:02}:{:02}", hour, minute)
                                                    }),
                                                );

                                                ui.add(
                                                    egui::Slider::new(
                                                        &mut self.gui_adjustables.latitude.value,
                                                        -1.0..=1.0,
                                                    )
                                                    .text("Latitude (South Pole to North Pole)")
                                                    .custom_formatter(|n, _| {
                                                        if n < -0.5 {
                                                            format!("South ({:.1})", n)
                                                        } else if n > 0.5 {
                                                            format!("North ({:.1})", n)
                                                        } else {
                                                            format!("Equator ({:.1})", n)
                                                        }
                                                    }),
                                                );

                                                ui.add(
                                                    egui::Slider::new(&mut self.gui_adjustables.season.value, 0.0..=1.0)
                                                        .text("Season (Winter to Summer)")
                                                        .custom_formatter(|n, _| {
                                                            if n < 0.125 {
                                                                "Winter".to_string()
                                                            } else if n < 0.375 {
                                                                "Spring".to_string()
                                                            } else if n < 0.625 {
                                                                "Summer".to_string()
                                                            } else if n < 0.875 {
                                                                "Autumn".to_string()
                                                            } else {
                                                                "Winter".to_string()
                                                            }
                                                        }),
                                                );

                                                ui.add(
                                                    egui::Slider::new(
                                                        &mut self.gui_adjustables.day_cycle_minutes.value,
                                                        0.1..=60.0,
                                                    )
                                                    .text("Day Cycle Duration (Real Minutes)")
                                                    .custom_formatter(|n, _| {
                                                        if n < 1.0 {
                                                            format!("{:.1}s", n * 60.0)
                                                        } else {
                                                            format!("{:.1}m", n)
                                                        }
                                                    }),
                                                );

// read-only displays for calculated values
                                                ui.separator();
                                                ui.label(format!(
                                                    "Sun Altitude: {:.3}",
                                                    self.gui_adjustables.sun_altitude.value
                                                ));
                                                ui.label(format!(
                                                    "Sun Azimuth: {:.3}",
                                                    self.gui_adjustables.sun_azimuth.value
                                                ));
                                            } else {
                                                ui.add(
                                                    egui::Slider::new(
                                                        &mut self.gui_adjustables.sun_altitude.value,
                                                        -1.0..=1.0,
                                                    )
                                                    .text("Altitude (normalized)")
                                                    .smart_aim(false),
                                                );
                                                ui.add(
                                                    egui::Slider::new(
                                                        &mut self.gui_adjustables.sun_azimuth.value,
                                                        0.0..=1.0,
                                                    )
                                                    .text("Azimuth (normalized)"),
                                                );
                                            }
                                            ui.add(
                                                egui::Slider::new(&mut self.gui_adjustables.sun_size.value, 0.0..=1.0)
                                                    .text("Size (relative)"),
                                            );
                                            ui.horizontal(|ui| {
                                                ui.label("Sun Color:");
                                                ui.color_edit_button_srgba(&mut self.gui_adjustables.sun_color.value);
                                            });
                                            ui.horizontal(|ui| {
                                                ui.add(
                                                    egui::Slider::new(
                                                        &mut self.gui_adjustables.sun_luminance.value,
                                                        0.0..=10.0,
                                                    )
                                                    .text("Sun Luminance"),
                                                );
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("Ambient Light:");
                                                ui.color_edit_button_srgba(&mut self.gui_adjustables.ambient_light.value);
                                            });
                                        });

                                        ui.collapsing("Starlight Settings", |ui| {
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.starlight_iterations.value,
                                                    1..=30,
                                                )
                                                .text("Iterations"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.starlight_formuparam.value,
                                                    0.0..=1.0,
                                                )
                                                .text("Form Parameter"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.starlight_volsteps.value,
                                                    1..=50,
                                                )
                                                .text("Volume Steps"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.starlight_stepsize.value,
                                                    0.01..=1.0,
                                                )
                                                .text("Step Size"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.starlight_zoom.value,
                                                    0.1..=2.0,
                                                )
                                                .text("Zoom"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.starlight_tile.value,
                                                    0.1..=2.0,
                                                )
                                                .text("Tile"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.starlight_speed.value,
                                                    0.001..=0.1,
                                                )
                                                .text("Speed"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.starlight_brightness.value,
                                                    0.0001..=0.01,
                                                )
                                                .text("Brightness"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.starlight_darkmatter.value,
                                                    0.0..=1.0,
                                                )
                                                .text("Dark Matter"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.starlight_distfading.value,
                                                    0.0..=1.0,
                                                )
                                                .text("Distance Fading"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.starlight_saturation.value,
                                                    0.0..=1.0,
                                                )
                                                .text("Saturation"),
                                            );
                                        });

                                        ui.collapsing("Tree Settings", |ui| {
                                            ui.label("Position:");
                                            let x_changed = ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut self.debug_tree_pos.x,
                                                        0.0..=4.0,
                                                    )
                                                    .text("X"),
                                                )
                                                .changed();
                                            tree_desc_changed |= x_changed;

                                            let z_changed = ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut self.debug_tree_pos.z,
                                                        0.0..=4.0,
                                                    )
                                                    .text("Z"),
                                                )
                                                .changed();
                                            tree_desc_changed |= z_changed;

// debug terrain height when X or Z position changes
                                            if x_changed || z_changed {
// clean up existing tree chunks before querying to avoid blocking the ray
                                                if let Err(e) = self.plain_builder.chunk_init(
                                                    self.prev_bound.min(),
                                                    self.prev_bound.max() - self.prev_bound.min(),
                                                ) {
                                                    log::error!("Failed to clean up chunks for terrain query: {}", e);
                                                } else {
// force mesh regeneration after cleanup
                                                    if let Err(e) = Self::mesh_generate(
                                                        &mut self.surface_builder,
                                                        &mut self.contree_builder,
                                                        &mut self.scene_accel_builder,
                                                        self.prev_bound,
                                                    ) {
                                                        log::error!("Failed to regenerate mesh after cleanup: {}", e);
                                                    } else {
// now query terrain height with clean terrain
                                                        match self.tracer.query_terrain_height(glam::Vec2::new(
                                                            self.debug_tree_pos.x,
                                                            self.debug_tree_pos.z,
                                                        )) {
                                                            Ok(terrain_height) => {
                                                                let terrain_height_scaled = terrain_height * 256.0;
                                                                log::info!("Debug terrain query - Position: ({}, {}), Terrain height: {}", 
                                                                    self.debug_tree_pos.x, self.debug_tree_pos.z, terrain_height_scaled);
                                                            }
                                                            Err(e) => {
                                                                log::error!("Failed to query terrain height: {}", e);
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            ui.separator();

                                            let (tree_changed, regenerate_pressed) =
                                                Self::edit_tree_with_variance(
                                                    &mut self.debug_tree_desc,
                                                    &mut self.tree_variation_config,
                                                    ui,
                                                );
                                            tree_desc_changed |= tree_changed;

                                            if regenerate_pressed {
                                                self.regenerate_trees_requested = true;
                                            }
                                        });

                                        ui.collapsing("Temporal Settings", |ui| {
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.temporal_position_phi.value,
                                                    0.0..=1.0,
                                                )
                                                .text("Position Phi"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.temporal_alpha.value,
                                                    0.0..=1.0,
                                                )
                                                .text("Alpha"),
                                            );
                                        });

                                        ui.collapsing("God Ray Settings", |ui| {
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.god_ray_max_depth.value,
                                                    0.1..=10.0,
                                                )
                                                .text("Max Depth"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.god_ray_max_checks.value,
                                                    1..=64,
                                                )
                                                .text("Max Checks"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.god_ray_weight.value,
                                                    0.0..=2.0,
                                                )
                                                .text("Weight"),
                                            );
                                        });

                                        ui.collapsing("Spatial Settings", |ui| {
                                            ui.add(
                                                egui::Slider::new(&mut self.gui_adjustables.phi_c.value, 0.0..=1.0)
                                                    .text("Phi C"),
                                            );
                                            ui.add(
                                                egui::Slider::new(&mut self.gui_adjustables.phi_n.value, 0.0..=1.0)
                                                    .text("Phi N"),
                                            );
                                            ui.add(
                                                egui::Slider::new(&mut self.gui_adjustables.phi_p.value, 0.0..=1.0)
                                                    .text("Phi P"),
                                            );
                                            ui.add(
                                                egui::Slider::new(&mut self.gui_adjustables.min_phi_z.value, 0.0..=1.0)
                                                    .text("Min Phi Z"),
                                            );
                                            ui.add(
                                                egui::Slider::new(&mut self.gui_adjustables.max_phi_z.value, 0.0..=1.0)
                                                    .text("Max Phi Z"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.phi_z_stable_sample_count.value,
                                                    0.0..=1.0,
                                                )
                                                .text("Phi Z Stable Sample Count"),
                                            );
                                            ui.add(egui::Checkbox::new(
                                                &mut self.gui_adjustables.is_changing_lum_phi.value,
                                                "Changing Luminance Phi",
                                            ));
                                            ui.add(egui::Checkbox::new(
                                                &mut self.gui_adjustables.is_spatial_denoising_enabled.value,
                                                "Enable Spatial Denoising",
                                            ));
                                            ui.horizontal(|ui| {
                                                ui.label("A-Trous Iterations:");
                                                let mut iteration_value = self.gui_adjustables.a_trous_iteration_count.value as i32;
                                                if ui.add(egui::Slider::new(&mut iteration_value, 1..=5).step_by(2.0)).changed() {
                                                    // Ensure only odd values (1, 3, 5)
                                                    if iteration_value % 2 == 0 {
                                                        iteration_value += 1;
                                                    }
                                                    self.gui_adjustables.a_trous_iteration_count.value = iteration_value as u32;
                                                }
                                            });
                                        });


                                        ui.collapsing("Grass Settings", |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label("Bottom Color:");
                                                ui.color_edit_button_srgba(
                                                    &mut self.gui_adjustables.grass_bottom_color.value,
                                                );
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("Tip Color:");
                                                ui.color_edit_button_srgba(
                                                    &mut self.gui_adjustables.grass_tip_color.value,
                                                );
                                            });
                                        });

                                        ui.collapsing("Ember Bloom Settings", |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label("Bottom Color:");
                                                ui.color_edit_button_srgba(
                                                    &mut self.gui_adjustables.ember_bloom_bottom_color.value,
                                                );
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("Tip Color:");
                                                ui.color_edit_button_srgba(
                                                    &mut self.gui_adjustables.ember_bloom_tip_color.value,
                                                );
                                            });
                                        });

                                        ui.collapsing("Flora Color Variations", |ui| {
                                            ui.label("Instance HSV offset range (± value)");
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.flora_instance_hue_offset.value,
                                                    0.0..=1.0,
                                                )
                                                .text("Hue Offset Max"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.flora_instance_saturation_offset.value,
                                                    0.0..=1.0,
                                                )
                                                .text("Saturation Offset Max"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.flora_instance_value_offset.value,
                                                    0.0..=1.0,
                                                )
                                                .text("Value Offset Max"),
                                            );
                                            ui.separator();
                                            ui.label("Per-voxel HSV offset range (± value)");
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.flora_voxel_hue_offset.value,
                                                    0.0..=1.0,
                                                )
                                                .text("Hue Offset Max"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.flora_voxel_saturation_offset.value,
                                                    0.0..=1.0,
                                                )
                                                .text("Saturation Offset Max"),
                                            );
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.flora_voxel_value_offset.value,
                                                    0.0..=1.0,
                                                )
                                                .text("Value Offset Max"),
                                            );
                                        });

                                        ui.collapsing("Leaves Settings", |ui| {
                                            let mut leaves_changed = false;
                                            leaves_changed |= ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut self.gui_adjustables.leaves_inner_density.value,
                                                        0.0..=1.0,
                                                    )
                                                    .text("Inner Density"),
                                                )
                                                .changed();
                                            leaves_changed |= ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut self.gui_adjustables.leaves_outer_density.value,
                                                        0.0..=1.0,
                                                    )
                                                    .text("Outer Density"),
                                                )
                                                .changed();
                                            leaves_changed |= ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut self.gui_adjustables.leaves_inner_radius.value,
                                                        1.0..=64.0,
                                                    )
                                                    .text("Inner Radius"),
                                                )
                                                .changed();
                                            leaves_changed |= ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut self.gui_adjustables.leaves_outer_radius.value,
                                                        1.0..=64.0,
                                                    )
                                                    .text("Outer Radius"),
                                                )
                                                .changed();

                                            if leaves_changed {
                                                // ensure inner_radius is always <= outer_radius
                                                if self.gui_adjustables.leaves_inner_radius.value > self.gui_adjustables.leaves_outer_radius.value {
                                                    self.gui_adjustables.leaves_outer_radius.value = self.gui_adjustables.leaves_inner_radius.value;
                                                }

                                                if let Err(e) = self.tracer.regenerate_leaves(
                                                    self.gui_adjustables.leaves_inner_density.value,
                                                    self.gui_adjustables.leaves_outer_density.value,
                                                    self.gui_adjustables.leaves_inner_radius.value,
                                                    self.gui_adjustables.leaves_outer_radius.value,
                                                ) {
                                                    log::error!(
                                                        "Failed to regenerate leaves: {}",
                                                        e
                                                    );
                                                }
                                            }

                                            ui.separator();
                                            let mut bottom_color_changed = false;
                                            let mut tip_color_changed = false;
                                            ui.horizontal(|ui| {
                                                ui.label("Bottom Color:");
                                                bottom_color_changed = ui.color_edit_button_srgba(
                                                    &mut self.gui_adjustables.leaves_bottom_color.value,
                                                ).changed();
                                            });
                                        ui.horizontal(|ui| {
                                            ui.label("Tip Color:");
                                            tip_color_changed = ui.color_edit_button_srgba(
                                                &mut self.gui_adjustables.leaves_tip_color.value,
                                            ).changed();
                                        });

                                        // Update leaf emitter colors when foliage colors change
                                        if bottom_color_changed || tip_color_changed {
                                            let color_to_vec4 = |color: Color32| -> Vec4 {
                                                Vec4::new(
                                                    color.r() as f32 / 255.0,
                                                    color.g() as f32 / 255.0,
                                                    color.b() as f32 / 255.0,
                                                    1.0,
                                                )
                                            };
                                            self.leaf_emitter_desc.color_low = color_to_vec4(self.gui_adjustables.leaves_bottom_color.value);
                                            self.leaf_emitter_desc.color_high = color_to_vec4(self.gui_adjustables.leaves_tip_color.value);
                                            for tree_emitter in &mut self.leaf_emitters {
                                                tree_emitter.emitter.color_low = self.leaf_emitter_desc.color_low;
                                                tree_emitter.emitter.color_high = self.leaf_emitter_desc.color_high;
                                            }
                                        }
                                    });

                                    ui.collapsing("Particle Emitters", |ui| {
                                        ui.label(format!(
                                            "Active Particles: {}",
                                            self.particle_system.alive_count()
                                        ));

                                        ui.separator();
                                        ui.label("Fallen Leaves");

                                        let mut spawn_rate = self.leaf_emitter_desc.spawn_rate;
                                        let spawn_rate_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut spawn_rate,
                                                    0.0..=400.0,
                                                )
                                                .text("Spawn Rate (per s)"),
                                            )
                                            .changed();
                                        if spawn_rate_changed {
                                            self.leaf_emitter_desc.spawn_rate = spawn_rate;
                                            for tree_emitter in &mut self.leaf_emitters {
                                                tree_emitter.emitter.spawn_rate = spawn_rate;
                                            }
                                        }

                                        let mut wind_spawn_min =
                                            self.leaf_emitter_desc.wind_spawn_min_strength;
                                        let min_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut wind_spawn_min,
                                                    0.0..=1.0,
                                                )
                                                .text("Wind Start Strength"),
                                            )
                                            .changed();
                                        if min_changed {
                                            self.leaf_emitter_desc.wind_spawn_min_strength =
                                                wind_spawn_min;
                                            for tree_emitter in &mut self.leaf_emitters {
                                                tree_emitter.emitter.wind_spawn_min_strength =
                                                    wind_spawn_min;
                                            }
                                        }

                                        let mut wind_spawn_max =
                                            self.leaf_emitter_desc.wind_spawn_max_strength;
                                        let max_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut wind_spawn_max,
                                                    0.0..=1.0,
                                                )
                                                .text("Wind Full Strength"),
                                            )
                                            .changed();
                                        if max_changed {
                                            self.leaf_emitter_desc.wind_spawn_max_strength =
                                                wind_spawn_max;
                                            for tree_emitter in &mut self.leaf_emitters {
                                                tree_emitter.emitter.wind_spawn_max_strength =
                                                    wind_spawn_max;
                                            }
                                        }

                                        let mut wind_spawn_power =
                                            self.leaf_emitter_desc.wind_spawn_power;
                                        let power_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut wind_spawn_power,
                                                    0.1..=4.0,
                                                )
                                                .text("Wind Spawn Curve"),
                                            )
                                            .changed();
                                        if power_changed {
                                            self.leaf_emitter_desc.wind_spawn_power =
                                                wind_spawn_power;
                                            for tree_emitter in &mut self.leaf_emitters {
                                                tree_emitter.emitter.wind_spawn_power =
                                                    wind_spawn_power;
                                            }
                                        }

                                        ui.separator();

                                        let mut perlin_min_speed =
                                            self.particle_forces.speed_noise.min_speed;
                                        let mut perlin_max_speed =
                                            self.particle_forces.speed_noise.max_speed;
                                        let mut perlin_freq =
                                            self.particle_forces.speed_noise.frequency;
                                        let mut perlin_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut perlin_min_speed,
                                                    -1.0..=1.0,
                                                )
                                                .text("Perlin Speed Min"),
                                            )
                                            .changed();
                                        perlin_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut perlin_max_speed,
                                                    -1.0..=1.0,
                                                )
                                                .text("Perlin Speed Max"),
                                            )
                                            .changed();
                                        perlin_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut perlin_freq,
                                                    0.01..=2.0,
                                                )
                                                .text("Perlin Speed Frequency"),
                                            )
                                            .changed();
                                        if perlin_changed {
                                            if perlin_min_speed > perlin_max_speed {
                                                std::mem::swap(
                                                    &mut perlin_min_speed,
                                                    &mut perlin_max_speed,
                                                );
                                            }
                                            self.particle_forces.speed_noise.min_speed =
                                                perlin_min_speed;
                                            self.particle_forces.speed_noise.max_speed =
                                                perlin_max_speed;
                                            self.particle_forces.speed_noise.frequency =
                                                perlin_freq;
                                        }

                                        ui.separator();

                                        let mut lifetime_min = self.leaf_emitter_desc.lifetime_min;
                                        let mut lifetime_max = self.leaf_emitter_desc.lifetime_max;
                                        let mut lifetime_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut lifetime_min,
                                                    0.0..=600.0,
                                                )
                                                .text("Lifetime Min (s)"),
                                            )
                                            .changed();
                                        lifetime_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut lifetime_max,
                                                    0.0..=600.0,
                                                )
                                                .text("Lifetime Max (s)"),
                                            )
                                            .changed();
                                        if lifetime_changed {
                                            if lifetime_min > lifetime_max {
                                                std::mem::swap(&mut lifetime_min, &mut lifetime_max);
                                            }
                                            self.leaf_emitter_desc.lifetime_min = lifetime_min;
                                            self.leaf_emitter_desc.lifetime_max = lifetime_max;
                                            for tree_emitter in &mut self.leaf_emitters {
                                                tree_emitter.emitter.lifetime =
                                                    lifetime_min..=lifetime_max;
                                            }
                                        }

                                        let vec4_to_color32 = |color: Vec4| -> Color32 {
                                            Color32::from_rgba_unmultiplied(
                                                (color.x * 255.0) as u8,
                                                (color.y * 255.0) as u8,
                                                (color.z * 255.0) as u8,
                                                (color.w * 255.0) as u8,
                                            )
                                        };
                                        let color32_to_vec4 = |color: Color32| -> Vec4 {
                                            Vec4::new(
                                                color.r() as f32 / 255.0,
                                                color.g() as f32 / 255.0,
                                                color.b() as f32 / 255.0,
                                                color.a() as f32 / 255.0,
                                            )
                                        };

                                        let mut color_low =
                                            vec4_to_color32(self.leaf_emitter_desc.color_low);
                                        let mut color_high =
                                            vec4_to_color32(self.leaf_emitter_desc.color_high);
                                        let mut color_changed = false;

                                        ui.horizontal(|ui| {
                                            ui.label("Color Low:");
                                            color_changed |= ui.color_edit_button_srgba(&mut color_low).changed();
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Color High:");
                                            color_changed |= ui.color_edit_button_srgba(&mut color_high).changed();
                                        });

                                        if color_changed {
                                            self.leaf_emitter_desc.color_low =
                                                color32_to_vec4(color_low);
                                            self.leaf_emitter_desc.color_high =
                                                color32_to_vec4(color_high);
                                            for tree_emitter in &mut self.leaf_emitters {
                                                tree_emitter.emitter.color_low =
                                                    self.leaf_emitter_desc.color_low;
                                                tree_emitter.emitter.color_high =
                                                    self.leaf_emitter_desc.color_high;
                                            }
                                        }

                                        ui.separator();
                                        ui.label("Butterflies");

                                        let mut butterflies_changed = false;

                                        let mut butterflies_enabled =
                                            self.butterfly_emitter_desc.enabled;
                                        butterflies_changed |= ui
                                            .checkbox(&mut butterflies_enabled, "Enable Butterflies")
                                            .changed();
                                        self.butterfly_emitter_desc.enabled = butterflies_enabled;

                                        let mut butterfly_spawn_rate =
                                            self.butterfly_emitter_desc.spawn_rate;
                                        let spawn_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut butterfly_spawn_rate,
                                                    0.0..=40.0,
                                                )
                                                .text("Spawn Rate (per s)"),
                                            )
                                            .changed();
                                        if spawn_changed {
                                            self.butterfly_emitter_desc.spawn_rate = butterfly_spawn_rate;
                                        }
                                        butterflies_changed |= spawn_changed;

                                        let mut max_butterflies =
                                            self.butterfly_emitter_desc.max_butterflies;
                                        let max_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut max_butterflies,
                                                    0..=128,
                                                )
                                                .text("Max Butterflies (map)"),
                                            )
                                            .changed();
                                        if max_changed {
                                            self.butterfly_emitter_desc.max_butterflies =
                                                max_butterflies;
                                        }
                                        butterflies_changed |= max_changed;

                                        let mut wander_radius =
                                            self.butterfly_emitter_desc.wander_radius;
                                        let wander_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut wander_radius,
                                                    0.5..=8.0,
                                                )
                                                .text("Wander Radius"),
                                            )
                                            .changed();
                                        if wander_changed {
                                            self.butterfly_emitter_desc.wander_radius = wander_radius;
                                        }
                                        butterflies_changed |= wander_changed;

                                        let mut height_min =
                                            self.butterfly_emitter_desc.height_offset_min;
                                        let mut height_max =
                                            self.butterfly_emitter_desc.height_offset_max;
                                        let mut height_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut height_min,
                                                    -1.0..=4.0,
                                                )
                                                .text("Height Offset Min"),
                                            )
                                            .changed();
                                        height_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut height_max,
                                                    -1.0..=4.0,
                                                )
                                                .text("Height Offset Max"),
                                            )
                                            .changed();
                                        if height_changed {
                                            if height_min > height_max {
                                                std::mem::swap(&mut height_min, &mut height_max);
                                            }
                                            self.butterfly_emitter_desc.height_offset_min =
                                                height_min;
                                            self.butterfly_emitter_desc.height_offset_max =
                                                height_max;
                                        }
                                        butterflies_changed |= height_changed;

                                        let mut butterfly_lifetime_min =
                                            self.butterfly_emitter_desc.lifetime_min;
                                        let mut butterfly_lifetime_max =
                                            self.butterfly_emitter_desc.lifetime_max;
                                        let mut lifetime_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut butterfly_lifetime_min,
                                                    1.0..=60.0,
                                                )
                                                .text("Lifetime Min (s)"),
                                            )
                                            .changed();
                                        lifetime_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut butterfly_lifetime_max,
                                                    1.0..=60.0,
                                                )
                                                .text("Lifetime Max (s)"),
                                            )
                                            .changed();
                                        if lifetime_changed {
                                            if butterfly_lifetime_min > butterfly_lifetime_max {
                                                std::mem::swap(
                                                    &mut butterfly_lifetime_min,
                                                    &mut butterfly_lifetime_max,
                                                );
                                            }
                                            self.butterfly_emitter_desc.lifetime_min =
                                                butterfly_lifetime_min;
                                            self.butterfly_emitter_desc.lifetime_max =
                                                butterfly_lifetime_max;
                                        }
                                        butterflies_changed |= lifetime_changed;

                                        let mut size_min = self.butterfly_emitter_desc.size_min;
                                        let mut size_max = self.butterfly_emitter_desc.size_max;
                                        let mut size_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut size_min,
                                                    0.001..=0.02,
                                                )
                                                .text("Size Min"),
                                            )
                                            .changed();
                                        size_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut size_max,
                                                    0.001..=0.02,
                                                )
                                                .text("Size Max"),
                                            )
                                            .changed();
                                        if size_changed {
                                            if size_min > size_max {
                                                std::mem::swap(&mut size_min, &mut size_max);
                                            }
                                            self.butterfly_emitter_desc.size_min = size_min;
                                            self.butterfly_emitter_desc.size_max = size_max;
                                        }
                                        butterflies_changed |= size_changed;

                                        let mut drift_strength_min =
                                            self.butterfly_emitter_desc.drift_strength_min;
                                        let mut drift_strength_max =
                                            self.butterfly_emitter_desc.drift_strength_max;
                                        let mut drift_strength_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut drift_strength_min,
                                                    0.0..=3.0,
                                                )
                                                .text("Flutter Strength Min"),
                                            )
                                            .changed();
                                        drift_strength_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut drift_strength_max,
                                                    0.0..=3.0,
                                                )
                                                .text("Flutter Strength Max"),
                                            )
                                            .changed();
                                        if drift_strength_changed {
                                            if drift_strength_min > drift_strength_max {
                                                std::mem::swap(
                                                    &mut drift_strength_min,
                                                    &mut drift_strength_max,
                                                );
                                            }
                                            self.butterfly_emitter_desc.drift_strength_min =
                                                drift_strength_min;
                                            self.butterfly_emitter_desc.drift_strength_max =
                                                drift_strength_max;
                                        }
                                        butterflies_changed |= drift_strength_changed;

                                        let mut drift_freq_min =
                                            self.butterfly_emitter_desc.drift_frequency_min;
                                        let mut drift_freq_max =
                                            self.butterfly_emitter_desc.drift_frequency_max;
                                        let mut drift_freq_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut drift_freq_min,
                                                    0.5..=6.0,
                                                )
                                                .text("Flutter Speed Min"),
                                            )
                                            .changed();
                                        drift_freq_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut drift_freq_max,
                                                    0.5..=6.0,
                                                )
                                                .text("Flutter Speed Max"),
                                            )
                                            .changed();
                                        if drift_freq_changed {
                                            if drift_freq_min > drift_freq_max {
                                                std::mem::swap(&mut drift_freq_min, &mut drift_freq_max);
                                            }
                                            self.butterfly_emitter_desc.drift_frequency_min =
                                                drift_freq_min;
                                            self.butterfly_emitter_desc.drift_frequency_max =
                                                drift_freq_max;
                                        }
                                        butterflies_changed |= drift_freq_changed;

                                        let mut steering_strength =
                                            self.butterfly_emitter_desc.steering_strength;
                                        let steering_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut steering_strength,
                                                    0.0..=3.0,
                                                )
                                                .text("Home Pull Strength"),
                                            )
                                            .changed();
                                        if steering_changed {
                                            self.butterfly_emitter_desc.steering_strength =
                                                steering_strength;
                                        }
                                        butterflies_changed |= steering_changed;

                                        let mut bob_frequency_hz =
                                            self.butterfly_emitter_desc.bob_frequency_hz;
                                        let bob_frequency_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut bob_frequency_hz,
                                                    0.0..=8.0,
                                                )
                                                .text("Vertical Bob Frequency (Hz)"),
                                            )
                                            .changed();
                                        if bob_frequency_changed {
                                            self.butterfly_emitter_desc.bob_frequency_hz =
                                                bob_frequency_hz;
                                        }
                                        butterflies_changed |= bob_frequency_changed;

                                        let mut bob_strength =
                                            self.butterfly_emitter_desc.bob_strength;
                                        let bob_strength_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut bob_strength,
                                                    0.0..=6.0,
                                                )
                                                .text("Vertical Bob Strength"),
                                            )
                                            .changed();
                                        if bob_strength_changed {
                                            self.butterfly_emitter_desc.bob_strength = bob_strength;
                                        }
                                        butterflies_changed |= bob_strength_changed;

                                        let mut butterfly_color_low =
                                            vec4_to_color32(self.butterfly_emitter_desc.color_low);
                                        let mut butterfly_color_high =
                                            vec4_to_color32(self.butterfly_emitter_desc.color_high);
                                        let mut butterfly_color_changed = false;

                                        ui.horizontal(|ui| {
                                            ui.label("Wing Color Low:");
                                            butterfly_color_changed |= ui
                                                .color_edit_button_srgba(&mut butterfly_color_low)
                                                .changed();
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Wing Color High:");
                                            butterfly_color_changed |= ui
                                                .color_edit_button_srgba(&mut butterfly_color_high)
                                                .changed();
                                        });

                                        if butterfly_color_changed {
                                            self.butterfly_emitter_desc.color_low =
                                                color32_to_vec4(butterfly_color_low);
                                            self.butterfly_emitter_desc.color_high =
                                                color32_to_vec4(butterfly_color_high);
                                        }
                                        butterflies_changed |= butterfly_color_changed;

                                        if butterflies_changed {
                                            for emitter in &mut self.butterfly_emitters {
                                                emitter.apply_desc(&self.butterfly_emitter_desc);
                                            }
                                        }

                                        if self.leaf_emitters.is_empty()
                                            && self.butterfly_emitters.is_empty()
                                        {
                                            ui.label("No active emitters");
                                        }
                                    });

                                    ui.collapsing("Voxel Colors", |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label("Dirt Color:");
                                                ui.color_edit_button_srgba(
                                                    &mut self.gui_adjustables.voxel_dirt_color.value,
                                                );
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("Trunk Color:");
                                                ui.color_edit_button_srgba(
                                                    &mut self.gui_adjustables.voxel_trunk_color.value,
                                                );
                                            });
                                        });

                                    });
                                });
                        }
                        self.config_panel_visible = config_panel_open;

                        // FPS counter in bottom right
                        egui::Area::new("fps_counter".into())
                            .anchor(egui::Align2::RIGHT_BOTTOM, egui::Vec2::new(-16.0, -16.0))
                            .show(ctx, |ui| {
                                let fps_frame = egui::containers::Frame {
                                    fill: PANEL_DARK,
                                    inner_margin: egui::Margin::symmetric(10, 6),
                                    corner_radius: egui::CornerRadius::same(0),
                                    shadow: egui::epaint::Shadow {
                                        offset: [4, 4],
                                        blur: 0,
                                        spread: 0,
                                        color: SHADOW_COLOR,
                                    },
                                    stroke: egui::Stroke::new(2.0, FLOWER_ACCENT),
                                    ..Default::default()
                                };

                                fps_frame.show(ui, |ui| {
                                    ui.allocate_ui_with_layout(
                                        egui::Vec2::new(110.0, 24.0),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                RichText::new("FPS")
                                                    .color(GOLD_ACCENT)
                                                    .monospace()
                                                    .size(12.0),
                                            );
                                            ui.add_space(6.0);
                                            ui.label(
                                                RichText::new(format!(
                                                    "{:.1}",
                                                    self.time_info.display_fps()
                                                ))
                                                .color(SAGE_ACCENT)
                                                .monospace()
                                                .strong(),
                                            );
                                        },
                                    );
                                });
                            });
                    });

                if tree_desc_changed {
                    self.add_tree(
                        self.debug_tree_desc.clone(),
                        TreePlacement::Terrain(Vec2::new(
                            self.debug_tree_pos.x,
                            self.debug_tree_pos.z,
                        )),
                        TreeAddOptions::default().with_cleanup(),
                    )
                    .unwrap();
                }

                if self.regenerate_trees_requested {
                    self.regenerate_trees_requested = false;
                    match self.generate_procedural_trees() {
                        Ok(_) => {
                            log::info!("Procedural trees regenerated successfully");
                        }
                        Err(e) => {
                            log::error!("Failed to regenerate procedural trees: {}", e);
                        }
                    }
                }

                // update sun position if auto day/night cycle is enabled
                if self.gui_adjustables.auto_daynight_cycle.value {
                    // update time of day based on delta time and day cycle speed
                    // day_cycle_minutes is the real-world minutes for a full day cycle
                    // convert to time progression per second: 1.0 / (day_cycle_minutes * 60.0)
                    let time_speed = 1.0 / (self.gui_adjustables.day_cycle_minutes.value * 60.0);
                    self.gui_adjustables.time_of_day.value += frame_delta_time * time_speed;

                    // keep time_of_day in 0.0 to 1.0 range (wrap around)
                    self.gui_adjustables.time_of_day.value %= 1.0;

                    self.calculate_sun_position(
                        self.gui_adjustables.time_of_day.value,
                        self.gui_adjustables.latitude.value,
                        self.gui_adjustables.season.value,
                    );
                }

                self.update_particle_simulation(frame_delta_time);

                let device = self.vulkan_ctx.device();
                let sync = &self.frames_in_flight[self.current_frame];
                let cmdbuf = &sync.command_buffer;

                self.vulkan_ctx
                    .wait_for_fences(&[sync.fence.as_raw()])
                    .unwrap();

                let image_idx = match self.swapchain.acquire_next(&sync.image_available) {
                    Ok((image_index, _)) => image_index,
                    Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                        self.is_resize_pending = true;
                        return;
                    }
                    Err(error) => panic!("Error while acquiring next image. Cause: {}", error),
                };

                let image_index_usize = image_idx as usize;
                let image_in_flight_fence = self.images_in_flight[image_index_usize];
                if image_in_flight_fence != vk::Fence::null() {
                    self.vulkan_ctx
                        .wait_for_fences(&[image_in_flight_fence])
                        .unwrap();
                }
                self.images_in_flight[image_index_usize] = sync.fence.as_raw();

                unsafe {
                    device
                        .as_raw()
                        .reset_fences(&[sync.fence.as_raw()])
                        .expect("Failed to reset fences")
                };

                cmdbuf.begin(false);

                self.tracer
                    .update_buffers(
                        &self.time_info,
                        self.gui_adjustables.debug_float.value,
                        self.gui_adjustables.debug_bool.value,
                        self.gui_adjustables.debug_uint.value,
                        Vec3::new(
                            self.gui_adjustables.flora_instance_hue_offset.value,
                            self.gui_adjustables.flora_instance_saturation_offset.value,
                            self.gui_adjustables.flora_instance_value_offset.value,
                        ),
                        Vec3::new(
                            self.gui_adjustables.flora_voxel_hue_offset.value,
                            self.gui_adjustables.flora_voxel_saturation_offset.value,
                            self.gui_adjustables.flora_voxel_value_offset.value,
                        ),
                        get_sun_dir(
                            self.gui_adjustables.sun_altitude.value.asin().to_degrees(),
                            self.gui_adjustables.sun_azimuth.value * 360.0,
                        ),
                        self.gui_adjustables.sun_size.value,
                        Vec3::new(
                            self.gui_adjustables.sun_color.value.r() as f32 / 255.0,
                            self.gui_adjustables.sun_color.value.g() as f32 / 255.0,
                            self.gui_adjustables.sun_color.value.b() as f32 / 255.0,
                        ),
                        self.gui_adjustables.sun_luminance.value,
                        self.gui_adjustables.sun_altitude.value,
                        self.gui_adjustables.sun_azimuth.value,
                        Vec3::new(
                            self.gui_adjustables.ambient_light.value.r() as f32 / 255.0,
                            self.gui_adjustables.ambient_light.value.g() as f32 / 255.0,
                            self.gui_adjustables.ambient_light.value.b() as f32 / 255.0,
                        ),
                        self.gui_adjustables.temporal_position_phi.value,
                        self.gui_adjustables.temporal_alpha.value,
                        self.gui_adjustables.phi_c.value,
                        self.gui_adjustables.phi_n.value,
                        self.gui_adjustables.phi_p.value,
                        self.gui_adjustables.min_phi_z.value,
                        self.gui_adjustables.max_phi_z.value,
                        self.gui_adjustables.phi_z_stable_sample_count.value,
                        self.gui_adjustables.is_changing_lum_phi.value,
                        self.gui_adjustables.is_spatial_denoising_enabled.value,
                        self.gui_adjustables.a_trous_iteration_count.value,
                        self.gui_adjustables.god_ray_max_depth.value,
                        self.gui_adjustables.god_ray_max_checks.value,
                        self.gui_adjustables.god_ray_weight.value,
                        Vec3::new(
                            self.gui_adjustables.sun_color.value.r() as f32 / 255.0,
                            self.gui_adjustables.sun_color.value.g() as f32 / 255.0,
                            self.gui_adjustables.sun_color.value.b() as f32 / 255.0,
                        ),
                        self.gui_adjustables.starlight_iterations.value,
                        self.gui_adjustables.starlight_formuparam.value,
                        self.gui_adjustables.starlight_volsteps.value,
                        self.gui_adjustables.starlight_stepsize.value,
                        self.gui_adjustables.starlight_zoom.value,
                        self.gui_adjustables.starlight_tile.value,
                        self.gui_adjustables.starlight_speed.value,
                        self.gui_adjustables.starlight_brightness.value,
                        self.gui_adjustables.starlight_darkmatter.value,
                        self.gui_adjustables.starlight_distfading.value,
                        self.gui_adjustables.starlight_saturation.value,
                        Vec3::new(
                            self.gui_adjustables.voxel_dirt_color.value.r() as f32 / 255.0,
                            self.gui_adjustables.voxel_dirt_color.value.g() as f32 / 255.0,
                            self.gui_adjustables.voxel_dirt_color.value.b() as f32 / 255.0,
                        ),
                        Vec3::new(
                            self.gui_adjustables.voxel_trunk_color.value.r() as f32 / 255.0,
                            self.gui_adjustables.voxel_trunk_color.value.g() as f32 / 255.0,
                            self.gui_adjustables.voxel_trunk_color.value.b() as f32 / 255.0,
                        ),
                    )
                    .unwrap();

                let color_to_vec3 = |color: Color32| -> Vec3 {
                    Vec3::new(
                        color.r() as f32 / 255.0,
                        color.g() as f32 / 255.0,
                        color.b() as f32 / 255.0,
                    )
                };

                let flora_colors: Vec<(Vec3, Vec3)> = species::species()
                    .iter()
                    .map(|desc| match desc.key {
                        "grass" => (
                            color_to_vec3(self.gui_adjustables.grass_bottom_color.value),
                            color_to_vec3(self.gui_adjustables.grass_tip_color.value),
                        ),
                        "ember_bloom" => (
                            color_to_vec3(self.gui_adjustables.ember_bloom_bottom_color.value),
                            color_to_vec3(self.gui_adjustables.ember_bloom_tip_color.value),
                        ),
                        _ => {
                            let bottom = Color32::from_rgb(
                                desc.default_bottom_color[0],
                                desc.default_bottom_color[1],
                                desc.default_bottom_color[2],
                            );
                            let tip = Color32::from_rgb(
                                desc.default_tip_color[0],
                                desc.default_tip_color[1],
                                desc.default_tip_color[2],
                            );
                            (color_to_vec3(bottom), color_to_vec3(tip))
                        }
                    })
                    .collect();

                let leaf_bottom = color_to_vec3(self.gui_adjustables.leaves_bottom_color.value);
                let leaf_tip = color_to_vec3(self.gui_adjustables.leaves_tip_color.value);

                self.tracer
                    .record_trace(
                        cmdbuf,
                        self.surface_builder.get_resources(),
                        self.gui_adjustables.lod_distance.value,
                        self.time_info.time_since_start(),
                        &flora_colors,
                        leaf_bottom,
                        leaf_tip,
                    )
                    .unwrap();

                self.swapchain.record_blit(
                    self.tracer.get_screen_output_tex().get_image(),
                    cmdbuf,
                    image_idx,
                );

                let render_area = self.window_state.window_extent();

                self.swapchain
                    .record_begin_render_pass_cmdbuf(cmdbuf, image_idx, render_area);

                self.egui_renderer
                    .record_command_buffer(device, cmdbuf, render_area);

                unsafe {
                    device.cmd_end_render_pass(cmdbuf.as_raw());
                };

                cmdbuf.end();

                let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
                let wait_semaphores = [sync.image_available.as_raw()];
                let render_finished = &self.image_render_finished_semaphores[image_index_usize];
                let signal_semaphores = [render_finished.as_raw()];
                let command_buffers = [cmdbuf.as_raw()];
                let submit_info = [vk::SubmitInfo::default()
                    .wait_semaphores(&wait_semaphores)
                    .wait_dst_stage_mask(&wait_stages)
                    .command_buffers(&command_buffers)
                    .signal_semaphores(&signal_semaphores)];

                unsafe {
                    self.vulkan_ctx
                        .device()
                        .as_raw()
                        .queue_submit(
                            self.vulkan_ctx.get_general_queue().as_raw(),
                            &submit_info,
                            sync.fence.as_raw(),
                        )
                        .expect("Failed to submit work to gpu.")
                };

                let present_result = self.swapchain.present(&signal_semaphores, image_idx);

                match present_result {
                    Ok(is_suboptimal) if is_suboptimal => {
                        self.is_resize_pending = true;
                    }
                    Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                        self.is_resize_pending = true;
                    }
                    Err(error) => panic!("Failed to present queue. Cause: {}", error),
                    _ => {}
                }

                self.current_frame = (self.current_frame + 1) % self.frames_in_flight.len();

                self.tracer
                    .update_camera(frame_delta_time, self.is_fly_mode);
            }
            _ => (),
        }
    }

    pub fn on_device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if !self.window_state.is_cursor_visible() {
                self.accumulated_mouse_delta += Vec2::new(delta.0 as f32, delta.1 as f32);
            }
        }
    }

    pub fn on_about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if !self.window_state.is_minimized() {
            self.window_state.window().request_redraw();
        }
    }

    fn on_resize(&mut self) {
        self.vulkan_ctx.device().wait_idle();

        let window_extent = self.window_state.window_extent();

        self.swapchain.on_resize(window_extent);
        self.rebuild_swapchain_image_syncs();
        self.tracer.on_resize(
            window_extent,
            self.contree_builder.get_resources(),
            self.scene_accel_builder.get_resources(),
        );

        // the render pass should be rebuilt when the swapchain is recreated
        self.egui_renderer
            .set_render_pass(self.swapchain.get_render_pass());

        self.is_resize_pending = false;
    }

    fn rebuild_swapchain_image_syncs(&mut self) {
        let device = self.vulkan_ctx.device();
        let image_count = self.swapchain.image_count();
        let (present_semaphores, images_in_flight) =
            Self::create_swapchain_image_syncs(device, image_count);
        self.image_render_finished_semaphores = present_semaphores;
        self.images_in_flight = images_in_flight;
    }

    fn create_swapchain_image_syncs(
        device: &Device,
        image_count: usize,
    ) -> (Vec<Semaphore>, Vec<vk::Fence>) {
        let semaphores = (0..image_count).map(|_| Semaphore::new(device)).collect();
        let images_in_flight = vec![vk::Fence::null(); image_count];
        (semaphores, images_in_flight)
    }
}

#[derive(Clone, Copy)]
struct ButterflyQueryTarget {
    emitter_index: usize,
    handle: crate::particles::ParticleHandle,
}
