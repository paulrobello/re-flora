#[allow(unused)]
use crate::util::Timer;

mod boot;
mod input;
mod lifecycle;
mod particles;
mod ui_style;
mod vegetation;

use self::particles::TreeLeafEmitter;
use self::vegetation::{TreeRecord, TreeVariationConfig};
use crate::app::environment;
use crate::app::gui_config_loader::GuiConfigLoader;
use crate::app::gui_config_model::GuiConfigFile;
use crate::app::world_edits::{
    BuildEdit, TreeAddOptions, TreePlacement, VoxelEdit, WorldBuildBackend, WorldEditPlan,
};
use crate::app::world_ops;
use crate::app::GuiAdjustables;
use crate::audio::{SpatialSoundManager, TreeAudioManager};
use crate::builder::{ContreeBuilder, PlainBuilder, SceneAccelBuilder, SurfaceBuilder};
use crate::flora::species;
use crate::geom::UAabb3;
use crate::particles::{
    ButterflyEmitter, ButterflyEmitterDesc, LeafEmitterDesc, ParticleForces, ParticleHandle,
    ParticleSnapshot, ParticleSystem, PARTICLE_CAPACITY,
};
use crate::tracer::{Tracer, TracerDesc};
use crate::tree_gen::TreeDesc;
use crate::util::TimeInfo;
use crate::util::{get_sun_dir, ShaderCompiler, BENCH};
use crate::vkn::{Allocator, CommandBuffer, Fence, Semaphore, SwapchainDesc};
use crate::RenderFlags;
use crate::{
    egui_renderer::EguiRenderer,
    vkn::{Swapchain, VulkanContext},
    window::WindowState,
};
use anyhow::{Context, Result};
use ash::vk;
use egui::{Color32, ColorImage, FontData, FontDefinitions, FontFamily, RichText, TextureHandle};
use glam::{UVec3, Vec2, Vec3, Vec4};
use gpu_allocator::vulkan::AllocatorCreateDesc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use ui_style::{
    apply_gui_style, draw_backpack_summary, draw_item_panel, CUSTOM_GUI_FONT_NAME,
    CUSTOM_GUI_FONT_PATH, FLOWER_ACCENT, GOLD_ACCENT, ITEM_PANEL_COPPER_SHOVEL_ICON_FALLBACK_PATH,
    ITEM_PANEL_COPPER_SHOVEL_ICON_PATH, ITEM_PANEL_HOE_ICON_FALLBACK_PATH,
    ITEM_PANEL_HOE_ICON_PATH, ITEM_PANEL_SHOVEL_ICON_FALLBACK_PATH, ITEM_PANEL_SHOVEL_ICON_PATH,
    ITEM_PANEL_SLOT_COUNT, ITEM_PANEL_STAFF_ICON_FALLBACK_PATH, ITEM_PANEL_STAFF_ICON_PATH,
    PANEL_BG, PANEL_DARK, SAGE_ACCENT, SHADOW_COLOR,
};
use uuid::Uuid;
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowId,
};

const LEAF_CLUSTER_DISTANCE: f32 = 0.08;

struct FrameSync {
    image_available: Semaphore,
    fence: Fence,
    command_buffer: CommandBuffer,
}

/// Tracks incremental chunk loading during app startup.
/// Loading proceeds in two phases so that all chunk voxel data is written
/// to the atlas before any surface normals are computed.  This eliminates
/// boundary-normal seams caused by missing neighbour data.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LoadingPhase {
    /// Phase 1: run chunk_init for every chunk (fills atlas with voxel data).
    Terrain,
    /// Phase 2: build surface + contree + flora + scene_accel per chunk.
    Building,
}

struct LoadingState {
    /// Ordered list of chunk IDs to process.
    chunk_indices: Vec<UVec3>,
    /// Current index into chunk_indices.
    current: usize,
    /// Label for display.
    step_label: String,
    /// Current loading phase.
    phase: LoadingPhase,
}

impl LoadingState {
    fn total(&self) -> usize {
        self.chunk_indices.len()
    }

    fn progress_fraction(&self) -> f32 {
        if self.chunk_indices.is_empty() {
            return 1.0;
        }
        let total = self.chunk_indices.len() as f32;
        match self.phase {
            // Terrain phase is the first half of progress
            LoadingPhase::Terrain => (self.current as f32 / total) * 0.5,
            // Building phase is the second half
            LoadingPhase::Building => 0.5 + (self.current as f32 / total) * 0.5,
        }
    }

    fn is_done(&self) -> bool {
        self.phase == LoadingPhase::Building && self.current >= self.chunk_indices.len()
    }
}

pub struct App {
    egui_renderer: EguiRenderer,
    loading_state: Option<LoadingState>,
    is_resize_pending: bool,
    swapchain: Swapchain,
    window_state: WindowState,
    frames_in_flight: Vec<FrameSync>,
    current_frame: usize,
    image_render_finished_semaphores: Vec<Semaphore>,
    images_in_flight: Vec<vk::Fence>,
    time_info: TimeInfo,
    render_flags: RenderFlags,
    accumulated_mouse_delta: Vec2,
    smoothed_mouse_delta: Vec2,
    perf_logging: bool,

    tracer: Tracer,

    // builders
    plain_builder: PlainBuilder,
    surface_builder: SurfaceBuilder,
    contree_builder: ContreeBuilder,
    scene_accel_builder: SceneAccelBuilder,

    // gui config and adjustables
    gui_config: GuiConfigFile,
    gui_adjustables: GuiAdjustables,
    debug_tree_pos: Vec3,
    config_panel_visible: bool,
    settings_panel_visible: bool,
    cursor_engaged: bool,
    is_fly_mode: bool,
    item_panel_shovel_icon: Option<TextureHandle>,
    item_panel_copper_shovel_icon: Option<TextureHandle>,
    item_panel_staff_icon: Option<TextureHandle>,
    item_panel_hoe_icon: Option<TextureHandle>,
    selected_item_panel_slot: usize,
    terrain_query_debug_text: String,
    prev_leaves_params: [f32; 4],
    left_mouse_held: bool,
    shovel_dig_held: bool,
    last_shovel_dig_time: Option<Instant>,
    last_copper_shovel_place_time: Option<Instant>,
    last_staff_regen_time: Option<Instant>,
    last_hoe_trim_time: Option<Instant>,
    backpack_dirt_count: u32,
    backpack_sand_count: u32,
    backpack_cherry_wood_count: u32,
    backpack_oak_wood_count: u32,
    backpack_rock_count: u32,
    terrain_edit_loop_sound: Option<Uuid>,
    terrain_edit_loop_sound_muted: bool,

    flora_tick: u32,
    flora_tick_accumulator: f32,

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
    /// Deferred placement retries: (handle, emitter_idx, attempts_remaining)
    pending_placement_retries: Vec<(ParticleHandle, usize, u8)>,
    /// Deferred movement retries: (handle, emitter_idx, origin, direction)
    pending_movement_retries: Vec<(ParticleHandle, usize, Vec3, Vec3)>,
    particle_animation_time_sec: f32,
    particle_snapshots: Vec<ParticleSnapshot>,
    particle_forces: ParticleForces,

    // screenshot / auto-exit automation
    /// When real rendering started (after loading completes).
    render_start_time: Option<Instant>,
    screenshot_path: Option<String>,
    screenshot_delay: f32,
    screenshot_taken: bool,
    auto_exit_delay: Option<f32>,

    // note: always keep the context to end, as it has to be destroyed last
    vulkan_ctx: VulkanContext,

    // Keep ownership so the shared PetalSonic engine outlives every subsystem.
    #[allow(dead_code)]
    spatial_sound_manager: SpatialSoundManager,
    tree_audio_manager: TreeAudioManager,
}

impl App {
    /// Save a screenshot by reading back the GPU render target and writing a PNG.
    fn save_screenshot(&self, path: &str) {
        // Ensure all GPU work is complete before reading back
        self.vulkan_ctx.device().wait_idle();

        let image = self.tracer.get_screen_output_tex().get_image();
        let extent = image.get_desc().extent;
        let width = extent.width;
        let height = extent.height;

        match image.fetch_data(
            &self.vulkan_ctx.get_general_queue(),
            self.vulkan_ctx.command_pool(),
        ) {
            Ok(rgba_data) => match image::RgbaImage::from_raw(width, height, rgba_data) {
                Some(img) => match img.save(path) {
                    Ok(()) => log::info!("[SCREENSHOT] Saved {}x{} to {}", width, height, path),
                    Err(e) => log::error!("[SCREENSHOT] Failed to write {}: {}", path, e),
                },
                None => log::error!(
                    "[SCREENSHOT] Pixel data size mismatch ({}x{}, {} bytes expected)",
                    width,
                    height,
                    width as usize * height as usize * 4,
                ),
            },
            Err(e) => log::error!("[SCREENSHOT] GPU readback failed: {}", e),
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // Ensure GPU work is done before resources begin destructing
        self.vulkan_ctx.device().wait_idle();
    }
}

impl WorldBuildBackend for App {
    fn apply_voxel_edit(&mut self, edit: VoxelEdit) -> Result<()> {
        world_ops::apply_voxel_edit(&mut self.plain_builder, edit)
    }

    fn apply_build_edit(&mut self, edit: BuildEdit) -> Result<()> {
        world_ops::apply_build_edit(
            &mut self.surface_builder,
            &mut self.contree_builder,
            &mut self.scene_accel_builder,
            VOXEL_DIM_PER_CHUNK,
            edit,
        )
    }
}

const VOXEL_DIM_PER_CHUNK: UVec3 = UVec3::new(256, 256, 256);
const CHUNK_DIM: UVec3 = UVec3::new(5, 2, 5);
const FREE_ATLAS_DIM: UVec3 = UVec3::new(512, 512, 512);
const MAX_FRAMES_IN_FLIGHT: usize = 2;
const SHOVEL_REMOVE_RADIUS: f32 = 0.08;
const SHOVEL_DIG_INTERVAL: Duration = Duration::from_millis(80);
const SHOVEL_RAY_QUERY_DISTANCE: f32 = 2.0;
const TERRAIN_EDIT_LOOP_PATH: &str =
    "assets/sfx/ROCKMisc_Designed Rock Movement Loop A_SARM_RkBrck_Stereo-Loop.wav";
const TERRAIN_EDIT_LOOP_VOLUME_DB: f32 = 20.0;
const TERRAIN_EDIT_LOOP_MUTED_VOLUME_DB: f32 = -80.0;
const ITEM_PANEL_SCROLL_SFX_PATH: &str =
    "assets/sfx/MECHSwtch_Game Boy Advance SP, B Button, On 05_SARM_BTNS.wav";
const ITEM_PANEL_SCROLL_SFX_VOLUME_DB: f32 = -6.0;
const FLORA_TICK_RATE_HZ: f32 = 1.0;
const FLORA_SPROUT_DELAY_TICKS: u32 = 2;
const FLORA_FULL_GROWTH_TICKS: u32 = 30;

impl App {
    fn linear_to_db(linear: f32) -> f32 {
        const MIN_DB: f32 = -80.0;
        const MAX_DB: f32 = 24.0;

        let linear = linear.clamp(0.0, 1.0);
        if linear <= 0.0 {
            return MIN_DB;
        }

        let max_gain = 10.0_f32.powf(MAX_DB / 20.0);

        let gain = if linear <= 0.5 {
            let normalized = linear / 0.5;
            // Give the lower half finer control so the slider feels closer to
            // the "audio taper" used by game settings rather than a relabeled
            // decibel control.
            normalized.powi(3)
        } else {
            let normalized = (linear - 0.5) / 0.5;
            1.0 + (max_gain - 1.0) * normalized.powi(2)
        };

        (20.0 * gain.log10()).clamp(MIN_DB, MAX_DB)
    }

    pub fn new(_event_loop: &ActiveEventLoop, options: &crate::AppOptions) -> Result<Self> {
        let chunk_bound = UAabb3::new(UVec3::ZERO, CHUNK_DIM);
        let window_state = Self::create_window_state(_event_loop, options);
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
                present_mode: vk::PresentModeKHR::IMMEDIATE,
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

        log::info!("[INIT] Creating EguiRenderer...");
        let renderer = EguiRenderer::new(
            vulkan_ctx.clone(),
            &window_state.window(),
            allocator.clone(),
            &shader_compiler,
            swapchain.get_render_pass(),
        );

        log::info!("[INIT] Creating PlainBuilder...");
        let plain_builder = PlainBuilder::new(
            vulkan_ctx.clone(),
            &shader_compiler,
            allocator.clone(),
            CHUNK_DIM * VOXEL_DIM_PER_CHUNK,
            FREE_ATLAS_DIM,
        );

        log::info!("[INIT] Creating SurfaceBuilder...");
        let surface_builder = SurfaceBuilder::new(
            vulkan_ctx.clone(),
            allocator.clone(),
            &shader_compiler,
            plain_builder.get_resources(),
            VOXEL_DIM_PER_CHUNK,
            chunk_bound,
        );

        log::info!("[INIT] Creating ContreeBuilder...");
        let contree_builder = ContreeBuilder::new(
            vulkan_ctx.clone(),
            allocator.clone(),
            &shader_compiler,
            surface_builder.get_resources(),
            VOXEL_DIM_PER_CHUNK,
            512 * 1024 * 1024, // node buffer pool size
            512 * 1024 * 1024, // leaf buffer pool size
        );

        log::info!("[INIT] Creating SceneAccelBuilder...");
        let scene_accel_builder = SceneAccelBuilder::new(
            vulkan_ctx.clone(),
            allocator.clone(),
            &shader_compiler,
            chunk_bound,
        )?;

        log::info!("[INIT] Preparing chunk loading (deferred to render loop)...");

        // Build chunk index list for incremental loading
        let world_dim = VOXEL_DIM_PER_CHUNK * CHUNK_DIM;
        let world_bound = UAabb3::new(UVec3::ZERO, world_dim - UVec3::ONE);
        let chunk_indices = world_ops::get_affected_chunk_indices(
            world_bound.min(),
            world_bound.max(),
            VOXEL_DIM_PER_CHUNK,
        );
        log::info!(
            "[INIT] Will load {} chunks incrementally during render loop.",
            chunk_indices.len()
        );

        // Shared spatial audio engine (PetalSonic) used by both the tracer (camera)
        // and the app-level tree ambience sources.
        let spatial_sound_manager = SpatialSoundManager::new(1024)?;
        let tree_audio_manager = TreeAudioManager::new(spatial_sound_manager.clone());

        log::info!("[INIT] Creating Tracer...");
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
        log::info!("[INIT] Tracer created.");

        let debug_tree_pos = Vec3::new(2.0, 0.2, 2.0);
        let gui_config = GuiConfigLoader::load();
        let gui_adjustables = GuiAdjustables::from_config(&gui_config);
        let init_leaves_params = [
            gui_adjustables.leaves_inner_density.value,
            gui_adjustables.leaves_outer_density.value,
            gui_adjustables.leaves_inner_radius.value,
            gui_adjustables.leaves_outer_radius.value,
        ];

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
        let butterfly_emitter_desc = Self::butterfly_desc_from_gui_adjustables(&gui_adjustables);
        let particle_snapshots = Vec::with_capacity(PARTICLE_CAPACITY);
        let particle_forces = ParticleForces {
            linear_damping: 0.08,
            ..ParticleForces::default()
        };

        let mut app = Self {
            vulkan_ctx,
            egui_renderer: renderer,
            window_state,
            loading_state: Some(LoadingState {
                chunk_indices,
                current: 0,
                step_label: "Initializing...".to_owned(),
                phase: LoadingPhase::Terrain,
            }),

            accumulated_mouse_delta: Vec2::ZERO,
            smoothed_mouse_delta: Vec2::ZERO,
            perf_logging: options.perf,

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
            render_flags: RenderFlags::from(options),

            gui_config,
            gui_adjustables,
            debug_tree_pos,
            debug_tree_desc: TreeDesc::default(),
            tree_variation_config: TreeVariationConfig::default(),
            regenerate_trees_requested: false,
            prev_bound: Default::default(),
            tree_records: HashMap::new(),
            config_panel_visible: false,
            settings_panel_visible: false,
            cursor_engaged: !options.windowed,
            is_fly_mode: true,
            item_panel_shovel_icon: None,
            item_panel_copper_shovel_icon: None,
            item_panel_staff_icon: None,
            item_panel_hoe_icon: None,
            selected_item_panel_slot: 0,
            terrain_query_debug_text: "not hit".to_owned(),
            prev_leaves_params: init_leaves_params,
            left_mouse_held: false,
            shovel_dig_held: false,
            last_shovel_dig_time: None,
            last_copper_shovel_place_time: None,
            last_staff_regen_time: None,
            last_hoe_trim_time: None,
            backpack_dirt_count: 0,
            backpack_sand_count: 0,
            backpack_cherry_wood_count: 0,
            backpack_oak_wood_count: 0,
            backpack_rock_count: 0,
            terrain_edit_loop_sound: None,
            terrain_edit_loop_sound_muted: true,
            flora_tick: FLORA_FULL_GROWTH_TICKS,
            flora_tick_accumulator: 0.0,

            // multi-tree management
            next_tree_id: 1, // Start from 1, use 0 for GUI single tree
            single_tree_id: 0,

            particle_system,
            leaf_emitters,
            tree_leaf_emitter_indices,
            leaf_emitter_desc,
            butterfly_emitters,
            butterfly_emitter_desc,
            pending_placement_retries: Vec::new(),
            pending_movement_retries: Vec::new(),
            particle_animation_time_sec: 0.0,
            particle_snapshots,
            particle_forces,

            render_start_time: None,
            screenshot_path: options.screenshot_path.clone(),
            screenshot_delay: options.screenshot_delay,
            screenshot_taken: false,
            auto_exit_delay: options.auto_exit_delay,

            spatial_sound_manager,
            tree_audio_manager,
        };

        if let Err(err) = app
            .spatial_sound_manager
            .set_global_volume_gain_db(Self::linear_to_db(app.gui_adjustables.master_volume.value))
        {
            log::error!("Failed to apply initial master volume: {}", err);
        }

        app.configure_gui_font()?;
        app.load_item_panel_icons()?;

        log::info!("[INIT] App struct ready. Chunk loading will proceed in render loop.");
        Ok(app)
    }

    /// Two-phase loading:
    ///   Phase 1 (Terrain): chunk_init for ALL chunks — fills the atlas so every
    ///           chunk's ±2 halo normal calculation can see its neighbours.
    ///   Phase 2 (Building): surface + contree + flora + scene_accel per chunk.
    fn process_loading_step(&mut self) {
        let loading = match &mut self.loading_state {
            Some(l) => l,
            None => return,
        };

        if loading.is_done() {
            return;
        }

        let total = loading.total();
        let current = loading.current + 1;

        match loading.phase {
            LoadingPhase::Terrain => {
                let chunk_id = loading.chunk_indices[loading.current];
                let atlas_offset = chunk_id * VOXEL_DIM_PER_CHUNK;
                loading.step_label = format!("Terrain {}/{}", current, total);

                if let Err(e) = self
                    .plain_builder
                    .chunk_init(atlas_offset, VOXEL_DIM_PER_CHUNK)
                {
                    log::error!("chunk_init failed for {chunk_id:?}: {e}");
                }

                loading.current += 1;
                if loading.current >= total {
                    // All terrain written — move to building phase
                    loading.current = 0;
                    loading.phase = LoadingPhase::Building;
                }
            }
            LoadingPhase::Building => {
                let chunk_id = loading.chunk_indices[loading.current];
                let atlas_offset = chunk_id * VOXEL_DIM_PER_CHUNK;
                loading.step_label = format!("Building {}/{}", current, total);

                // Surface build + flora seeding — submit without waiting.
                if let Err(e) = self
                    .surface_builder
                    .build_surface_and_flora_submit_only(chunk_id)
                {
                    log::error!("build_surface failed for {chunk_id:?}: {e}");
                    loading.current += 1;
                    return;
                }

                // Contree build — wait covers all prior submissions
                let res = self.contree_builder.build_and_alloc(atlas_offset).unwrap();

                // Finalize flora (readback instance lengths — GPU already done)
                if let Err(e) = self.surface_builder.finalize_flora_seeding(chunk_id) {
                    log::error!("finalize_flora_seeding failed for {chunk_id:?}: {e}");
                }

                // Update scene accel texture
                if let Some((node_buffer_offset, leaf_buffer_offset)) = res {
                    if let Err(e) = self.scene_accel_builder.update_scene_tex(
                        chunk_id,
                        node_buffer_offset,
                        leaf_buffer_offset,
                    ) {
                        log::error!("update_scene_tex failed for {chunk_id:?}: {e}");
                    }
                }
                loading.current += 1;
            }
        }
    }

    /// Render the loading screen (progress bar + status text).
    /// Handles the full frame: acquire -> egui render -> present.
    fn render_loading_frame(&mut self) {
        let loading = match &self.loading_state {
            Some(l) => l,
            None => return,
        };

        let progress = loading.progress_fraction();
        let step_label = loading.step_label.clone();
        let is_done = loading.is_done();
        let total = loading.total();
        let current = loading.current;

        // Update egui with loading UI
        self.egui_renderer
            .update(&self.window_state.window(), |ctx| {
                // Dark background panel centered on screen
                #[allow(deprecated)]
                egui::CentralPanel::default()
                    .frame(egui::containers::Frame {
                        fill: Color32::from_rgb(20, 20, 25),
                        ..Default::default()
                    })
                    .show(ctx, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(ui.available_height() * 0.3);

                            ui.label(
                                RichText::new("Re: Flora")
                                    .size(36.0)
                                    .color(Color32::from_rgb(200, 180, 140)),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("Loading world...")
                                    .size(18.0)
                                    .color(Color32::from_rgb(160, 160, 170)),
                            );
                            ui.add_space(24.0);

                            // Progress bar with centered bold percentage
                            let bar_width = ui.available_width().min(400.0);
                            let progress = if is_done { 1.0 } else { progress };
                            let bar_height = 24.0;
                            let (rect, _bar_response) = ui.allocate_at_least(
                                egui::vec2(bar_width, bar_height),
                                egui::Sense::hover(),
                            );

                            let painter = ui.painter();
                            // Background
                            painter.rect_filled(rect, 2.0, Color32::from_rgb(40, 40, 50));
                            // Filled portion
                            let fill_width = rect.width() * progress;
                            let fill_rect = egui::Rect::from_min_max(
                                rect.min,
                                egui::pos2(rect.min.x + fill_width, rect.max.y),
                            );
                            painter.rect_filled(fill_rect, 2.0, Color32::from_rgb(100, 140, 80));

                            // Centered bold percentage text with shadow
                            let pct_text = format!("{}%", (progress * 100.0) as u32);
                            let font = egui::FontId::proportional(14.0);
                            let shadow_galley = painter.layout_no_wrap(
                                pct_text.clone(),
                                font.clone(),
                                Color32::from_black_alpha(120),
                            );
                            let galley = painter.layout_no_wrap(pct_text, font, Color32::WHITE);
                            let text_pos = egui::pos2(
                                rect.center().x - galley.size().x / 2.0,
                                rect.center().y - galley.size().y / 2.0,
                            );
                            painter.galley(
                                egui::pos2(text_pos.x + 1.0, text_pos.y + 1.0),
                                shadow_galley,
                                Color32::from_black_alpha(120),
                            );
                            painter.galley(text_pos, galley, Color32::WHITE);

                            ui.add_space(12.0);

                            // Status text
                            let status = if is_done {
                                "Finalizing...".to_owned()
                            } else {
                                format!("{} — chunk {}/{}", step_label, current + 1, total)
                            };
                            ui.label(
                                RichText::new(status)
                                    .size(14.0)
                                    .color(Color32::from_rgb(130, 130, 140)),
                            );
                        });
                    });
            });

        // Present the frame
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

        // Check if loading is complete
        if is_done {
            log::info!("[INIT] All chunks loaded. Running post-init tasks...");
            self.loading_state = None;
            self.finalize_loading();
        }
    }

    /// Called once after all chunks are loaded to finish initialization.
    fn finalize_loading(&mut self) {
        // Ensure all pending GPU work (from the last loading frame) is complete
        // before submitting new work during post-init.
        self.vulkan_ctx.device().wait_idle();

        BENCH.lock().unwrap().summary();

        self.ensure_map_butterfly_emitter();

        log::info!("[INIT] Adding debug tree...");
        if let Err(e) = self.add_tree(
            self.debug_tree_desc.clone(),
            TreePlacement::Terrain(Vec2::new(self.debug_tree_pos.x, self.debug_tree_pos.z)),
            TreeAddOptions::default(),
        ) {
            log::error!("Failed to add debug tree: {e}");
        }
        log::info!("[INIT] Debug tree added.");

        log::info!("[INIT] Regenerating leaves...");
        if let Err(e) = self.tracer.regenerate_leaves(
            self.gui_adjustables.leaves_inner_density.value,
            self.gui_adjustables.leaves_outer_density.value,
            self.gui_adjustables.leaves_inner_radius.value,
            self.gui_adjustables.leaves_outer_radius.value,
        ) {
            log::error!("Failed to regenerate leaves: {e}");
        }
        // Ensure all post-init GPU work (tree stamping, leaf regeneration) is fully
        // complete before the first real render frame begins.
        self.vulkan_ctx.device().wait_idle();

        // Mark the start of real rendering for screenshot/auto-exit timers.
        self.render_start_time = Some(Instant::now());
        log::info!("[INIT] App initialization complete. Render timers started.");
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

    fn load_item_panel_icons(&mut self) -> Result<()> {
        let shovel_path = if std::path::Path::new(ITEM_PANEL_SHOVEL_ICON_PATH).exists() {
            ITEM_PANEL_SHOVEL_ICON_PATH
        } else {
            log::warn!(
                "Item panel icon not found at {}. Falling back to {}",
                ITEM_PANEL_SHOVEL_ICON_PATH,
                ITEM_PANEL_SHOVEL_ICON_FALLBACK_PATH
            );
            ITEM_PANEL_SHOVEL_ICON_FALLBACK_PATH
        };

        let staff_path = if std::path::Path::new(ITEM_PANEL_STAFF_ICON_PATH).exists() {
            ITEM_PANEL_STAFF_ICON_PATH
        } else {
            log::warn!(
                "Item panel icon not found at {}. Falling back to {}",
                ITEM_PANEL_STAFF_ICON_PATH,
                ITEM_PANEL_STAFF_ICON_FALLBACK_PATH
            );
            ITEM_PANEL_STAFF_ICON_FALLBACK_PATH
        };

        let shovel_bytes = std::fs::read(shovel_path)
            .with_context(|| format!("Failed to read item panel icon from {shovel_path}"))?;
        let shovel_rgba = image::load_from_memory(&shovel_bytes)
            .with_context(|| format!("Failed to decode item panel icon from {shovel_path}"))?
            .to_rgba8();
        let shovel_size = [shovel_rgba.width() as usize, shovel_rgba.height() as usize];
        let shovel_pixels = shovel_rgba.into_raw();
        let shovel_image = ColorImage::from_rgba_unmultiplied(shovel_size, &shovel_pixels);

        let shovel_texture = self.egui_renderer.context().load_texture(
            "item_panel_wooden_shovel",
            shovel_image,
            egui::TextureOptions::NEAREST,
        );
        self.item_panel_shovel_icon = Some(shovel_texture);

        let copper_shovel_path =
            if std::path::Path::new(ITEM_PANEL_COPPER_SHOVEL_ICON_PATH).exists() {
                ITEM_PANEL_COPPER_SHOVEL_ICON_PATH
            } else {
                log::warn!(
                    "Item panel icon not found at {}. Falling back to {}",
                    ITEM_PANEL_COPPER_SHOVEL_ICON_PATH,
                    ITEM_PANEL_COPPER_SHOVEL_ICON_FALLBACK_PATH
                );
                ITEM_PANEL_COPPER_SHOVEL_ICON_FALLBACK_PATH
            };

        let copper_shovel_bytes = std::fs::read(copper_shovel_path)
            .with_context(|| format!("Failed to read item panel icon from {copper_shovel_path}"))?;
        let copper_shovel_rgba = image::load_from_memory(&copper_shovel_bytes)
            .with_context(|| format!("Failed to decode item panel icon from {copper_shovel_path}"))?
            .to_rgba8();
        let copper_shovel_size = [
            copper_shovel_rgba.width() as usize,
            copper_shovel_rgba.height() as usize,
        ];
        let copper_shovel_pixels = copper_shovel_rgba.into_raw();
        let copper_shovel_image =
            ColorImage::from_rgba_unmultiplied(copper_shovel_size, &copper_shovel_pixels);

        let copper_shovel_texture = self.egui_renderer.context().load_texture(
            "item_panel_copper_shovel",
            copper_shovel_image,
            egui::TextureOptions::NEAREST,
        );
        self.item_panel_copper_shovel_icon = Some(copper_shovel_texture);

        let staff_bytes = std::fs::read(staff_path)
            .with_context(|| format!("Failed to read item panel icon from {staff_path}"))?;
        let staff_rgba = image::load_from_memory(&staff_bytes)
            .with_context(|| format!("Failed to decode item panel icon from {staff_path}"))?
            .to_rgba8();
        let staff_size = [staff_rgba.width() as usize, staff_rgba.height() as usize];
        let staff_pixels = staff_rgba.into_raw();
        let staff_image = ColorImage::from_rgba_unmultiplied(staff_size, &staff_pixels);

        let staff_texture = self.egui_renderer.context().load_texture(
            "item_panel_wooden_staff",
            staff_image,
            egui::TextureOptions::NEAREST,
        );
        self.item_panel_staff_icon = Some(staff_texture);

        let hoe_path = if std::path::Path::new(ITEM_PANEL_HOE_ICON_PATH).exists() {
            ITEM_PANEL_HOE_ICON_PATH
        } else {
            log::warn!(
                "Item panel icon not found at {}. Falling back to {}",
                ITEM_PANEL_HOE_ICON_PATH,
                ITEM_PANEL_HOE_ICON_FALLBACK_PATH
            );
            ITEM_PANEL_HOE_ICON_FALLBACK_PATH
        };

        let hoe_bytes = std::fs::read(hoe_path)
            .with_context(|| format!("Failed to read item panel icon from {hoe_path}"))?;
        let hoe_rgba = image::load_from_memory(&hoe_bytes)
            .with_context(|| format!("Failed to decode item panel icon from {hoe_path}"))?
            .to_rgba8();
        let hoe_size = [hoe_rgba.width() as usize, hoe_rgba.height() as usize];
        let hoe_pixels = hoe_rgba.into_raw();
        let hoe_image = ColorImage::from_rgba_unmultiplied(hoe_size, &hoe_pixels);

        let hoe_texture = self.egui_renderer.context().load_texture(
            "item_panel_wooden_hoe",
            hoe_image,
            egui::TextureOptions::NEAREST,
        );
        self.item_panel_hoe_icon = Some(hoe_texture);
        Ok(())
    }

    fn calculate_sun_position(time_of_day: f32, latitude: f32, season: f32) -> (f32, f32) {
        environment::calculate_sun_position(time_of_day, latitude, season)
    }

    fn execute_edit_plan(&mut self, plan: WorldEditPlan) -> Result<()> {
        world_ops::execute_edit_plan_on_backend(self, plan)
    }

    pub fn on_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        if let WindowEvent::KeyboardInput { event, .. } = &event {
            if event.state == ElementState::Pressed && event.physical_key == KeyCode::KeyQ {
                self.on_terminate(event_loop);
                return;
            }

            if event.state == ElementState::Pressed && event.physical_key == KeyCode::Escape {
                self.settings_panel_visible = !self.settings_panel_visible;
                self.sync_cursor_with_panels();
                return;
            }
        }

        // if cursor is visible, feed the event to gui first, if the event is being consumed by gui, no need to handle it again later
        if self.window_state.is_cursor_visible() {
            let consumed = self
                .egui_renderer
                .on_window_event(&self.window_state.window(), &event)
                .consumed;

            if consumed {
                return;
            }

            // Click in viewport (not consumed by GUI) → engage cursor and close panels.
            // But don't recapture if the pointer is over an egui panel area.
            if let WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } = &event
            {
                if !self.egui_renderer.context().is_pointer_over_egui() {
                    self.cursor_engaged = true;
                    self.config_panel_visible = false;
                    self.settings_panel_visible = false;
                    self.sync_cursor_with_panels();
                }
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
                if event.state == ElementState::Pressed && event.physical_key == KeyCode::KeyE {
                    self.config_panel_visible = !self.config_panel_visible;
                    self.sync_cursor_with_panels();
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
                    if event.state == ElementState::Pressed {
                        let target_slot = match event.physical_key {
                            PhysicalKey::Code(KeyCode::Digit1) => Some(0),
                            PhysicalKey::Code(KeyCode::Digit2) => Some(1),
                            PhysicalKey::Code(KeyCode::Digit3) => Some(2),
                            PhysicalKey::Code(KeyCode::Digit4) => Some(3),
                            PhysicalKey::Code(KeyCode::Digit5) => Some(4),
                            _ => None,
                        };

                        if let Some(slot_idx) = target_slot {
                            if slot_idx < ITEM_PANEL_SLOT_COUNT
                                && slot_idx != self.selected_item_panel_slot
                            {
                                self.selected_item_panel_slot = slot_idx;
                                self.play_item_panel_scroll_sound();
                            }
                        }
                    }

                    self.tracer.handle_keyboard(&event);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    self.left_mouse_held = state == ElementState::Pressed;
                }

                if !self.window_state.is_cursor_visible() && button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            self.update_terrain_query_debug_text();
                            self.shovel_dig_held = true;
                            let now = Instant::now();
                            if self.is_shovel_selected() {
                                self.try_shovel_dig(now);
                            } else if self.is_copper_shovel_selected() {
                                self.try_copper_shovel_place(now);
                            } else if self.is_staff_selected() {
                                self.try_staff_regenerate(now);
                            } else if self.is_hoe_selected() {
                                self.try_hoe_trim(now);
                            }
                        }
                        ElementState::Released => {
                            self.shovel_dig_held = false;
                            self.stop_terrain_edit_loop_sound();
                        }
                    }
                }
            }

            // redraw the window
            WindowEvent::RedrawRequested => {
                // when the windiw is resized, redraw is called afterwards, so when the window is minimized, return
                if self.window_state.is_minimized() {
                    return;
                }

                let redraw_start = Instant::now();

                // resize the window if needed
                if self.is_resize_pending {
                    self.on_resize();
                }

                self.window_state.maintain_cursor_grab();

                self.time_info.update(self.perf_logging);

                // During loading, process one chunk step per frame and show loading screen
                if self.loading_state.is_some() {
                    self.process_loading_step();
                    self.render_loading_frame();
                    return;
                }

                if self.shovel_dig_held {
                    self.update_terrain_query_debug_text();
                    let now = Instant::now();
                    if self.is_shovel_selected() {
                        self.try_shovel_dig(now);
                    } else if self.is_copper_shovel_selected() {
                        self.try_copper_shovel_place(now);
                    } else if self.is_staff_selected() {
                        self.try_staff_regenerate(now);
                    } else if self.is_hoe_selected() {
                        self.try_hoe_trim(now);
                    } else {
                        self.stop_terrain_edit_loop_sound();
                    }
                }
                let frame_delta_time = self.time_info.delta_time();
                let time_since_start = self.time_info.time_since_start();
                self.flora_tick_accumulator += frame_delta_time * FLORA_TICK_RATE_HZ;
                while self.flora_tick_accumulator >= 1.0 {
                    self.flora_tick = self.flora_tick.wrapping_add(1);
                    self.flora_tick_accumulator -= 1.0;
                }
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

                let tree_desc_changed = false;
                let item_panel_shovel_icon = self.item_panel_shovel_icon.clone();
                let item_panel_copper_shovel_icon = self.item_panel_copper_shovel_icon.clone();
                let item_panel_staff_icon = self.item_panel_staff_icon.clone();
                let item_panel_hoe_icon = self.item_panel_hoe_icon.clone();
                let selected_item_panel_slot = self.selected_item_panel_slot;
                let backpack_dirt_count = self.backpack_dirt_count;
                let backpack_sand_count = self.backpack_sand_count;
                let backpack_cherry_wood_count = self.backpack_cherry_wood_count;
                let backpack_oak_wood_count = self.backpack_oak_wood_count;
                let backpack_rock_count = self.backpack_rock_count;
                let terrain_query_debug_text = self.terrain_query_debug_text.clone();
                let egui_start = Instant::now();
                self.egui_renderer
                    .update(&self.window_state.window(), |ctx| {
                        let mut style = (*ctx.global_style()).clone();
                        apply_gui_style(&mut style);
                        ctx.set_global_style(style);

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
                                stroke: egui::Stroke::new(3.0, SAGE_ACCENT),
                                ..Default::default()
                            };

                            let content_rect = ctx.content_rect();
                            let panel_pos = egui::pos2(content_rect.left(), content_rect.top());
                            let panel_size = egui::Vec2::new(
                                content_rect.width() * 0.24,
                                content_rect.height() * 0.6,
                            );

                            egui::Window::new("Debug Panel")
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
                                            RichText::new("Debug Panel")
                                                .size(18.0)
                                                .color(GOLD_ACCENT),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .add(egui::Button::new("Save").small())
                                                    .clicked()
                                                {
                                                    match self.gui_adjustables.save_to_config() {
                                                        Ok(_) => {
                                                            log::info!("Config saved successfully");
                                                        }
                                                        Err(e) => {
                                                            log::error!(
                                                                "Failed to save config: {}",
                                                                e
                                                            );
                                                        }
                                                    }
                                                }
                                                if ui
                                                    .add(egui::Button::new("Reset").small())
                                                    .clicked()
                                                {
                                                    self.gui_adjustables.reload_from_config();
                                                }
                                            },
                                        );
                                    });

                                    ui.add_space(4.0);
                                    ui.separator();
                                    ui.add_space(4.0);

                                    egui::ScrollArea::vertical().auto_shrink([false; 2]).show(
                                        ui,
                                        |ui| {
                                            crate::app::render_gui_from_config(
                                                ui,
                                                &self.gui_config,
                                                &mut self.gui_adjustables,
                                            );
                                        },
                                    );
                                });
                        }
                        self.config_panel_visible = config_panel_open;

                        if self.settings_panel_visible {
                            let settings_size = egui::Vec2::new(
                                ctx.content_rect().width() * 0.5,
                                ctx.content_rect().height() * 0.5,
                            );

                            egui::Window::new("Settings Panel")
                                .id(egui::Id::new("settings_panel"))
                                .resizable(false)
                                .movable(false)
                                .collapsible(false)
                                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                                .default_size(settings_size)
                                .show(ctx, |ui| {
                                    ui.heading(
                                        RichText::new("Audio").size(18.0).color(GOLD_ACCENT),
                                    );
                                    ui.add_space(8.0);
                                    ui.add(
                                        egui::Slider::new(
                                            &mut self.gui_adjustables.master_volume.value,
                                            0.0..=1.0,
                                        )
                                        .text("Master Volume"),
                                    );
                                });
                        }

                        draw_item_panel(
                            ctx,
                            item_panel_shovel_icon.as_ref(),
                            item_panel_copper_shovel_icon.as_ref(),
                            item_panel_staff_icon.as_ref(),
                            item_panel_hoe_icon.as_ref(),
                            selected_item_panel_slot,
                        );

                        draw_backpack_summary(
                            ctx,
                            backpack_dirt_count,
                            backpack_sand_count,
                            backpack_cherry_wood_count,
                            backpack_oak_wood_count,
                            backpack_rock_count,
                            terrain_query_debug_text.as_str(),
                        );

                        if self.left_mouse_held {
                            let center = ctx.content_rect().center();
                            let painter = ctx.layer_painter(egui::LayerId::new(
                                egui::Order::Foreground,
                                egui::Id::new("debug_center_dot"),
                            ));
                            painter.circle_filled(center, 4.0, Color32::RED);
                        }

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
                self.sync_cursor_with_panels();

                if let Err(err) =
                    self.spatial_sound_manager
                        .set_global_volume_gain_db(Self::linear_to_db(
                            self.gui_adjustables.master_volume.value,
                        ))
                {
                    log::error!("Failed to apply master volume: {}", err);
                }

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

                    Self::calculate_sun_position(
                        self.gui_adjustables.time_of_day.value,
                        self.gui_adjustables.latitude.value,
                        self.gui_adjustables.season.value,
                    );
                }

                let egui_ms = egui_start.elapsed().as_secs_f32() * 1000.0;

                // CPU-only particle work: emitter updates, physics.
                // Runs while the GPU is still rendering the previous frame.
                if self.render_flags.enable_particles {
                    self.update_particle_cpu(frame_delta_time);
                }

                let fence_wait_start = Instant::now();
                // Wait for frame N-2 (this slot was last used 2 frames ago)
                {
                    let fence = self.frames_in_flight[self.current_frame].fence.as_raw();
                    self.vulkan_ctx.wait_for_fences(&[fence]).unwrap();
                }
                let fence_wait_ms = fence_wait_start.elapsed().as_secs_f32() * 1000.0;

                let acquire_start = Instant::now();
                let image_idx = {
                    let sem = &self.frames_in_flight[self.current_frame].image_available;
                    match self.swapchain.acquire_next(sem) {
                        Ok((image_index, _)) => image_index,
                        Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                            self.is_resize_pending = true;
                            return;
                        }
                        Err(error) => {
                            panic!("Error while acquiring next image. Cause: {}", error)
                        }
                    }
                };
                let acquire_ms = acquire_start.elapsed().as_secs_f32() * 1000.0;

                let img_fence_start = Instant::now();
                let image_index_usize = image_idx as usize;
                let image_in_flight_fence = self.images_in_flight[image_index_usize];
                if image_in_flight_fence != vk::Fence::null() {
                    self.vulkan_ctx
                        .wait_for_fences(&[image_in_flight_fence])
                        .unwrap();
                }
                self.images_in_flight[image_index_usize] =
                    self.frames_in_flight[self.current_frame].fence.as_raw();

                // Wait for frame N-1's render commands to complete before submitting
                // terrain query commands to the same queue. The "other" slot was used
                // last frame; waiting on its fence ensures no race on MoltenVK.
                {
                    let prev_frame = (self.current_frame + 1) % self.frames_in_flight.len();
                    let prev_fence = self.frames_in_flight[prev_frame].fence.as_raw();
                    self.vulkan_ctx.wait_for_fences(&[prev_fence]).unwrap();
                }

                // GPU terrain queries + particle upload. All prior render fences
                // are now satisfied, so execute_one_time_command won't race.
                let particle_start = Instant::now();
                if self.render_flags.enable_particles {
                    self.update_particle_gpu();
                } else if let Err(err) = self.tracer.upload_particles(&self.particle_snapshots) {
                    log::error!("Failed to upload particles: {}", err);
                }
                let particle_ms = particle_start.elapsed().as_secs_f32() * 1000.0;

                let device = self.vulkan_ctx.device();
                let sync = &self.frames_in_flight[self.current_frame];
                let cmdbuf = &sync.command_buffer;

                unsafe {
                    device
                        .as_raw()
                        .reset_fences(&[sync.fence.as_raw()])
                        .expect("Failed to reset fences")
                };

                cmdbuf.begin(false);
                let img_fence_ms = img_fence_start.elapsed().as_secs_f32() * 1000.0;

                let record_start = Instant::now();

                let (sun_altitude, sun_azimuth) = Self::calculate_sun_position(
                    self.gui_adjustables.time_of_day.value,
                    self.gui_adjustables.latitude.value,
                    self.gui_adjustables.season.value,
                );

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
                        Vec3::new(
                            self.gui_adjustables.grass_bottom_dark_color.value.r() as f32 / 255.0,
                            self.gui_adjustables.grass_bottom_dark_color.value.g() as f32 / 255.0,
                            self.gui_adjustables.grass_bottom_dark_color.value.b() as f32 / 255.0,
                        ),
                        Vec3::new(
                            self.gui_adjustables.grass_bottom_light_color.value.r() as f32 / 255.0,
                            self.gui_adjustables.grass_bottom_light_color.value.g() as f32 / 255.0,
                            self.gui_adjustables.grass_bottom_light_color.value.b() as f32 / 255.0,
                        ),
                        Vec3::new(
                            self.gui_adjustables.grass_tip_dark_color.value.r() as f32 / 255.0,
                            self.gui_adjustables.grass_tip_dark_color.value.g() as f32 / 255.0,
                            self.gui_adjustables.grass_tip_dark_color.value.b() as f32 / 255.0,
                        ),
                        Vec3::new(
                            self.gui_adjustables.grass_tip_light_color.value.r() as f32 / 255.0,
                            self.gui_adjustables.grass_tip_light_color.value.g() as f32 / 255.0,
                            self.gui_adjustables.grass_tip_light_color.value.b() as f32 / 255.0,
                        ),
                        Vec3::new(
                            self.gui_adjustables.ocean_deep_color.value.r() as f32 / 255.0,
                            self.gui_adjustables.ocean_deep_color.value.g() as f32 / 255.0,
                            self.gui_adjustables.ocean_deep_color.value.b() as f32 / 255.0,
                        ),
                        Vec3::new(
                            self.gui_adjustables.ocean_shallow_color.value.r() as f32 / 255.0,
                            self.gui_adjustables.ocean_shallow_color.value.g() as f32 / 255.0,
                            self.gui_adjustables.ocean_shallow_color.value.b() as f32 / 255.0,
                        ),
                        self.gui_adjustables.ocean_normal_amplitude.value,
                        self.gui_adjustables.ocean_noise_frequency.value,
                        self.gui_adjustables.ocean_time_multiplier.value,
                        self.gui_adjustables.ocean_sea_level_shift.value,
                        self.gui_adjustables.flora_update_bucket_count.value,
                        self.gui_adjustables.flora_full_update_seconds.value,
                        self.gui_adjustables.lens_flare_intensity.value,
                        self.gui_adjustables.lens_flare_sun_pixel_scale.value,
                        self.flora_tick,
                        FLORA_SPROUT_DELAY_TICKS,
                        FLORA_FULL_GROWTH_TICKS,
                        get_sun_dir(sun_altitude.asin().to_degrees(), sun_azimuth * 360.0),
                        self.gui_adjustables.sun_size.value,
                        Vec3::new(
                            self.gui_adjustables.sun_color.value.r() as f32 / 255.0,
                            self.gui_adjustables.sun_color.value.g() as f32 / 255.0,
                            self.gui_adjustables.sun_color.value.b() as f32 / 255.0,
                        ),
                        self.gui_adjustables.sun_luminance.value,
                        self.gui_adjustables.sun_display_luminance.value,
                        sun_altitude,
                        sun_azimuth,
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
                            self.gui_adjustables.voxel_cherry_wood_color.value.r() as f32 / 255.0,
                            self.gui_adjustables.voxel_cherry_wood_color.value.g() as f32 / 255.0,
                            self.gui_adjustables.voxel_cherry_wood_color.value.b() as f32 / 255.0,
                        ),
                        Vec3::new(
                            self.gui_adjustables.voxel_oak_wood_color.value.r() as f32 / 255.0,
                            self.gui_adjustables.voxel_oak_wood_color.value.g() as f32 / 255.0,
                            self.gui_adjustables.voxel_oak_wood_color.value.b() as f32 / 255.0,
                        ),
                        self.gui_adjustables.voxel_color_variance.value,
                    )
                    .unwrap();

                // Regenerate leaves geometry if density/radius params changed
                let cur_leaves_params = [
                    self.gui_adjustables.leaves_inner_density.value,
                    self.gui_adjustables.leaves_outer_density.value,
                    self.gui_adjustables.leaves_inner_radius.value,
                    self.gui_adjustables.leaves_outer_radius.value,
                ];
                if cur_leaves_params != self.prev_leaves_params {
                    self.prev_leaves_params = cur_leaves_params;
                    if let Err(e) = self.tracer.regenerate_leaves(
                        cur_leaves_params[0],
                        cur_leaves_params[1],
                        cur_leaves_params[2],
                        cur_leaves_params[3],
                    ) {
                        log::error!("Failed to regenerate leaves: {e}");
                    }
                }

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
                            color_to_vec3(self.gui_adjustables.grass_bottom_dark_color.value),
                            color_to_vec3(self.gui_adjustables.grass_tip_light_color.value),
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

                // Always call record_trace — individual passes are gated by
                // render_flags inside. Skipping entirely leaves screen_output_tex
                // in UNDEFINED layout, causing SIGBUS on MoltenVK when record_blit
                // reads from it.
                self.tracer
                    .record_trace(
                        cmdbuf,
                        self.surface_builder.get_resources(),
                        self.gui_adjustables.lod_distance.value,
                        self.gui_adjustables.flora_draw_distance.value,
                        self.time_info.time_since_start(),
                        &flora_colors,
                        leaf_bottom,
                        leaf_tip,
                        &self.render_flags,
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

                let record_ms = record_start.elapsed().as_secs_f32() * 1000.0;

                let present_start = Instant::now();
                let present_result = self.swapchain.present(&signal_semaphores, image_idx);
                let present_ms = present_start.elapsed().as_secs_f32() * 1000.0;

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

                self.tracer.set_head_bob_params(
                    self.gui_adjustables.headbob_vertical_amp.value,
                    self.gui_adjustables.headbob_horizontal_amp.value,
                    self.gui_adjustables.headbob_roll_amp.value,
                    self.gui_adjustables.headbob_sprint_amp_mul.value,
                );

                self.tracer
                    .update_camera(frame_delta_time, self.is_fly_mode);

                let camera_ms = present_start.elapsed().as_secs_f32() * 1000.0 - present_ms;
                let total_handler_ms = redraw_start.elapsed().as_secs_f32() * 1000.0;
                if self.perf_logging {
                    log::info!(
                        "[PERF] egui: {:.1}ms | particle: {:.1}ms | fence: {:.1}ms | acquire: {:.1}ms | img_fence: {:.1}ms | record: {:.1}ms | present: {:.1}ms | camera: {:.1}ms | total: {:.1}ms",
                        egui_ms, particle_ms, fence_wait_ms, acquire_ms, img_fence_ms, record_ms, present_ms, camera_ms, total_handler_ms,
                    );
                }

                // Screenshot and auto-exit automation
                if let Some(render_start) = self.render_start_time {
                    let elapsed = render_start.elapsed().as_secs_f32();

                    // Screenshot: capture window contents after delay
                    if !self.screenshot_taken {
                        if let Some(ref path) = self.screenshot_path {
                            if elapsed >= self.screenshot_delay {
                                self.screenshot_taken = true;
                                log::info!(
                                    "[SCREENSHOT] Capturing after {:.1}s to {}",
                                    elapsed,
                                    path
                                );
                                // Use swapchain readback to save the screenshot
                                self.save_screenshot(path);
                            }
                        }
                    }

                    // Auto-exit after delay
                    if let Some(exit_delay) = self.auto_exit_delay {
                        if elapsed >= exit_delay {
                            log::info!("[AUTO-EXIT] Exiting after {:.1}s of rendering", elapsed);
                            self.on_terminate(event_loop);
                            return;
                        }
                    }
                }

                // Request next redraw immediately (bypass display link latency)
                self.window_state.window().request_redraw();
            }
            _ => (),
        }
    }
}
