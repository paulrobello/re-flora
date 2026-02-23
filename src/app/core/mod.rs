#[allow(unused)]
use crate::util::Timer;

mod boot;
mod input;
mod lifecycle;
mod particles;
mod ui_style;
mod vegetation;

use self::particles::{BirdAudioBinding, TreeLeafEmitter};
use self::vegetation::{TreeRecord, TreeVariationConfig};
use crate::app::environment;
use crate::app::world_edits::{
    BuildEdit, ClearVoxelRegionEdit, TreeAddOptions, TreePlacement, VoxelEdit, WorldBuildBackend,
    WorldEditPlan,
};
use crate::app::world_ops;
use crate::app::GuiAdjustables;
use crate::audio::{SpatialSoundManager, TreeAudioManager};
use crate::builder::{ContreeBuilder, PlainBuilder, SceneAccelBuilder, SurfaceBuilder};
use crate::flora::species;
use crate::geom::UAabb3;
use crate::particles::{
    BirdEmitter, BirdEmitterDesc, ButterflyEmitter, ButterflyEmitterDesc, LeafEmitterDesc,
    ParticleForces, ParticleSnapshot, ParticleSystem, PARTICLE_CAPACITY,
};
use crate::tracer::{Tracer, TracerDesc};
use crate::tree_gen::TreeDesc;
use crate::util::TimeInfo;
use crate::util::{get_sun_dir, ShaderCompiler};
use crate::vkn::{Allocator, CommandBuffer, Fence, Semaphore, SwapchainDesc};
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
    apply_gui_style, draw_item_panel, CUSTOM_GUI_FONT_NAME, CUSTOM_GUI_FONT_PATH, FLOWER_ACCENT,
    GOLD_ACCENT, ITEM_PANEL_SHOVEL_ICON_FALLBACK_PATH, ITEM_PANEL_SHOVEL_ICON_PATH,
    ITEM_PANEL_SLOT_COUNT, ITEM_PANEL_STAFF_ICON_FALLBACK_PATH, ITEM_PANEL_STAFF_ICON_PATH,
    PANEL_BG, PANEL_DARK, SAGE_ACCENT, SHADOW_COLOR,
};
use uuid::Uuid;
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::KeyCode,
    window::WindowId,
};

const LEAF_CLUSTER_DISTANCE: f32 = 0.08;

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
    settings_panel_visible: bool,
    is_fly_mode: bool,
    item_panel_shovel_icon: Option<TextureHandle>,
    item_panel_staff_icon: Option<TextureHandle>,
    selected_item_panel_slot: usize,
    shovel_dig_held: bool,
    last_shovel_dig_time: Option<Instant>,
    last_staff_regen_time: Option<Instant>,
    terrain_edit_loop_sound: Option<Uuid>,

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
    bird_emitters: Vec<BirdEmitter>,
    bird_emitter_desc: BirdEmitterDesc,
    particle_snapshots: Vec<ParticleSnapshot>,
    particle_forces: ParticleForces,
    bird_audio_binding: BirdAudioBinding,

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
const MAX_FRAMES_IN_FLIGHT: usize = 1;
const SHOVEL_REMOVE_RADIUS: f32 = 0.08;
const SHOVEL_DIG_INTERVAL: Duration = Duration::from_millis(80);
const SHOVEL_RAY_QUERY_DISTANCE: f32 = 2.0;
const TERRAIN_EDIT_LOOP_PATH: &str =
    "assets/sfx/ROCKMisc_Designed Rock Movement Loop A_SARM_RkBrck_Stereo-Loop.wav";
const TERRAIN_EDIT_LOOP_VOLUME_DB: f32 = -10.0;
const ITEM_PANEL_SCROLL_SFX_PATH: &str =
    "assets/sfx/MECHSwtch_Game Boy Advance SP, B Button, On 05_SARM_BTNS.wav";
const ITEM_PANEL_SCROLL_SFX_VOLUME_DB: f32 = -6.0;
const FLORA_TICK_RATE_HZ: f32 = 1.0;
const FLORA_SPROUT_DELAY_TICKS: u32 = 2;
const FLORA_FULL_GROWTH_TICKS: u32 = 30;

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
        let butterfly_emitter_desc = Self::butterfly_desc_from_gui_adjustables(&gui_adjustables);
        let bird_emitters = Vec::new();
        let bird_emitter_desc = butterfly_emitter_desc;
        let particle_snapshots = Vec::with_capacity(PARTICLE_CAPACITY);
        let particle_forces = ParticleForces {
            linear_damping: 0.08,
            ..ParticleForces::default()
        };
        let bird_audio_binding = BirdAudioBinding::default();

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
            settings_panel_visible: false,
            is_fly_mode: false,
            item_panel_shovel_icon: None,
            item_panel_staff_icon: None,
            selected_item_panel_slot: 0,
            shovel_dig_held: false,
            last_shovel_dig_time: None,
            last_staff_regen_time: None,
            terrain_edit_loop_sound: None,
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
            bird_emitters,
            bird_emitter_desc,
            particle_snapshots,
            particle_forces,
            bird_audio_binding,

            spatial_sound_manager,
            tree_audio_manager,
        };

        app.configure_gui_font()?;
        app.load_item_panel_icons()?;
        app.ensure_map_butterfly_emitter();
        app.ensure_map_bird_emitter();

        app.add_tree(
            app.debug_tree_desc.clone(),
            TreePlacement::Terrain(Vec2::new(app.debug_tree_pos.x, app.debug_tree_pos.z)),
            TreeAddOptions::default(),
        )?;
        app.plant_map_region_fence_posts()?;

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
        Ok(())
    }

    fn calculate_sun_position(&mut self, time_of_day: f32, latitude: f32, season: f32) {
        let (sun_altitude, sun_azimuth) =
            environment::calculate_sun_position(time_of_day, latitude, season);
        self.gui_adjustables.sun_altitude.value = sun_altitude;
        self.gui_adjustables.sun_azimuth.value = sun_azimuth;
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
                    self.tracer.handle_keyboard(&event);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if !self.window_state.is_cursor_visible() {
                    let scroll_y = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(position) => position.y as f32,
                    };

                    let step = if scroll_y > 0.0 {
                        -1
                    } else if scroll_y < 0.0 {
                        1
                    } else {
                        0
                    };

                    if step != 0 {
                        let next_slot = (self.selected_item_panel_slot as i32 + step)
                            .rem_euclid(ITEM_PANEL_SLOT_COUNT as i32)
                            as usize;
                        self.selected_item_panel_slot = next_slot;
                        self.play_item_panel_scroll_sound();
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if !self.window_state.is_cursor_visible() && button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            self.shovel_dig_held = true;
                            let now = Instant::now();
                            if self.is_shovel_selected() {
                                self.try_shovel_dig(now);
                            } else if self.is_staff_selected() {
                                self.try_staff_regenerate(now);
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

                // resize the window if needed
                if self.is_resize_pending {
                    self.on_resize();
                }

                self.window_state.maintain_cursor_grab();

                self.time_info.update();
                if self.shovel_dig_held {
                    let now = Instant::now();
                    if self.is_shovel_selected() {
                        self.try_shovel_dig(now);
                    } else if self.is_staff_selected() {
                        self.try_staff_regenerate(now);
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

                let mut tree_desc_changed = false;
                let item_panel_shovel_icon = self.item_panel_shovel_icon.clone();
                let item_panel_staff_icon = self.item_panel_staff_icon.clone();
                let selected_item_panel_slot = self.selected_item_panel_slot;
                self.egui_renderer
                    .update(&self.window_state.window(), |ctx| {
                        let mut style = (*ctx.style()).clone();
                        apply_gui_style(&mut style);
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
                                                let prev_bound = self.prev_bound;
                                                if let Err(e) = world_ops::execute_edit_plan_on_builders(
                                                    &mut self.plain_builder,
                                                    &mut self.surface_builder,
                                                    &mut self.contree_builder,
                                                    &mut self.scene_accel_builder,
                                                    VOXEL_DIM_PER_CHUNK,
                                                    WorldEditPlan {
                                                        voxel_edits: vec![VoxelEdit::ClearVoxelRegion(
                                                            ClearVoxelRegionEdit {
                                                                offset: prev_bound.min(),
                                                                dim: prev_bound.max()
                                                                    - prev_bound.min(),
                                                            },
                                                        )],
                                                        build_edits: vec![BuildEdit::RebuildMesh(
                                                            prev_bound,
                                                        )],
                                                    },
                                                ) {
                                                    log::error!("Failed to clean up chunks for terrain query: {}", e);
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

                                        butterflies_changed |= ui
                                            .checkbox(
                                                &mut self.gui_adjustables.butterflies_enabled.value,
                                                "Enable Butterflies",
                                            )
                                            .changed();
                                        butterflies_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.butterflies_per_chunk.value,
                                                    0.0..=2.0,
                                                )
                                                .text("Butterflies Per Chunk"),
                                            )
                                            .changed();
                                        butterflies_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.butterfly_wander_radius.value,
                                                    0.5..=8.0,
                                                )
                                                .text("Wander Radius"),
                                            )
                                            .changed();

                                        let mut height_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut self
                                                        .gui_adjustables
                                                        .butterfly_height_offset_min
                                                        .value,
                                                    0.0..=4.0,
                                                )
                                                .text("Height Offset Min"),
                                            )
                                            .changed();
                                        height_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut self
                                                        .gui_adjustables
                                                        .butterfly_height_offset_max
                                                        .value,
                                                    0.0..=4.0,
                                                )
                                                .text("Height Offset Max"),
                                            )
                                            .changed();
                                        if height_changed
                                            && self.gui_adjustables.butterfly_height_offset_min.value
                                                > self.gui_adjustables.butterfly_height_offset_max.value
                                        {
                                            std::mem::swap(
                                                &mut self
                                                    .gui_adjustables
                                                    .butterfly_height_offset_min
                                                    .value,
                                                &mut self
                                                    .gui_adjustables
                                                    .butterfly_height_offset_max
                                                    .value,
                                            );
                                        }
                                        butterflies_changed |= height_changed;

                                        butterflies_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.butterfly_size.value,
                                                    0.001..=0.03,
                                                )
                                                .text("Size"),
                                            )
                                            .changed();

                                        let mut drift_strength_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut self
                                                        .gui_adjustables
                                                        .butterfly_drift_strength_min
                                                        .value,
                                                    0.0..=3.0,
                                                )
                                                .text("Flutter Strength Min"),
                                            )
                                            .changed();
                                        drift_strength_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut self
                                                        .gui_adjustables
                                                        .butterfly_drift_strength_max
                                                        .value,
                                                    0.0..=3.0,
                                                )
                                                .text("Flutter Strength Max"),
                                            )
                                            .changed();
                                        if drift_strength_changed
                                            && self.gui_adjustables.butterfly_drift_strength_min.value
                                                > self.gui_adjustables.butterfly_drift_strength_max.value
                                        {
                                            std::mem::swap(
                                                &mut self
                                                    .gui_adjustables
                                                    .butterfly_drift_strength_min
                                                    .value,
                                                &mut self
                                                    .gui_adjustables
                                                    .butterfly_drift_strength_max
                                                    .value,
                                            );
                                        }
                                        butterflies_changed |= drift_strength_changed;

                                        let mut drift_freq_changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut self
                                                        .gui_adjustables
                                                        .butterfly_drift_frequency_min
                                                        .value,
                                                    0.5..=6.0,
                                                )
                                                .text("Flutter Speed Min"),
                                            )
                                            .changed();
                                        drift_freq_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut self
                                                        .gui_adjustables
                                                        .butterfly_drift_frequency_max
                                                        .value,
                                                    0.5..=6.0,
                                                )
                                                .text("Flutter Speed Max"),
                                            )
                                            .changed();
                                        if drift_freq_changed
                                            && self.gui_adjustables.butterfly_drift_frequency_min.value
                                                > self.gui_adjustables.butterfly_drift_frequency_max.value
                                        {
                                            std::mem::swap(
                                                &mut self
                                                    .gui_adjustables
                                                    .butterfly_drift_frequency_min
                                                    .value,
                                                &mut self
                                                    .gui_adjustables
                                                    .butterfly_drift_frequency_max
                                                    .value,
                                            );
                                        }
                                        butterflies_changed |= drift_freq_changed;

                                        butterflies_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut self
                                                        .gui_adjustables
                                                        .butterfly_steering_strength
                                                        .value,
                                                    0.0..=3.0,
                                                )
                                                .text("Home Pull Strength"),
                                            )
                                            .changed();
                                        butterflies_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut self
                                                        .gui_adjustables
                                                        .butterfly_bob_frequency_hz
                                                        .value,
                                                    0.0..=8.0,
                                                )
                                                .text("Vertical Bob Frequency (Hz)"),
                                            )
                                            .changed();
                                        butterflies_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut self
                                                        .gui_adjustables
                                                        .butterfly_bob_strength
                                                        .value,
                                                    0.0..=6.0,
                                                )
                                                .text("Vertical Bob Strength"),
                                            )
                                            .changed();

                                        ui.horizontal(|ui| {
                                            ui.label("Wing Color Low:");
                                            butterflies_changed |= ui
                                                .color_edit_button_srgba(
                                                    &mut self
                                                        .gui_adjustables
                                                        .butterfly_wing_color_low
                                                        .value,
                                                )
                                                .changed();
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Wing Color High:");
                                            butterflies_changed |= ui
                                                .color_edit_button_srgba(
                                                    &mut self
                                                        .gui_adjustables
                                                        .butterfly_wing_color_high
                                                        .value,
                                                )
                                                .changed();
                                        });

                                        if butterflies_changed {
                                            self.butterfly_emitter_desc =
                                                Self::butterfly_desc_from_gui_adjustables(
                                                    &self.gui_adjustables,
                                                );
                                            self.bird_emitter_desc = self.butterfly_emitter_desc;
                                            for emitter in &mut self.butterfly_emitters {
                                                emitter.apply_desc(&self.butterfly_emitter_desc);
                                            }
                                            for emitter in &mut self.bird_emitters {
                                                emitter.apply_desc(&self.bird_emitter_desc);
                                            }
                                        }

                                        if self.leaf_emitters.is_empty()
                                            && self.butterfly_emitters.is_empty()
                                            && self.bird_emitters.is_empty()
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
                                                ui.label("Cherry Wood Color:");
                                                ui.color_edit_button_srgba(
                                                    &mut self.gui_adjustables.voxel_cherry_wood_color.value,
                                                );
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("Oak Wood Color:");
                                                ui.color_edit_button_srgba(
                                                    &mut self.gui_adjustables.voxel_oak_wood_color.value,
                                                );
                                            });
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut self.gui_adjustables.voxel_color_variance.value,
                                                    0.0..=2.0,
                                                )
                                                .text("Hash Color Variance"),
                                            );
                                        });

                                    });
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
                                .show(ctx, |_ui| {});
                        }

                        draw_item_panel(
                            ctx,
                            item_panel_shovel_icon.as_ref(),
                            item_panel_staff_icon.as_ref(),
                            selected_item_panel_slot,
                        );

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
                        self.flora_tick,
                        FLORA_SPROUT_DELAY_TICKS,
                        FLORA_FULL_GROWTH_TICKS,
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
}
