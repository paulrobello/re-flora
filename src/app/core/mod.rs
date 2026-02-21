#[allow(unused)]
use crate::util::Timer;

mod particles;
mod ui_style;

use self::particles::TreeLeafEmitter;
use crate::app::environment;
use crate::app::world_edits::{
    BuildEdit, ClearVoxelRegionEdit, CubePlacementEdit, FencePostPlacementEdit, TerrainRemovalEdit,
    TreeAddOptions, TreePlacement, TreePlacementEdit, VoxelEdit, WorldBuildBackend, WorldEditPlan,
};
use crate::app::world_ops;
use crate::app::GuiAdjustables;
use crate::audio::{SpatialSoundManager, TreeAudioManager};
use crate::builder::{
    ContreeBuilder, PlainBuilder, SceneAccelBuilder, SurfaceBuilder, VOXEL_TYPE_CHERRY_WOOD,
    VOXEL_TYPE_EMPTY, VOXEL_TYPE_OAK_WOOD,
};
use crate::flora::species;
use crate::geom::{build_bvh, Cuboid, RoundCone, Sphere, UAabb3};
use crate::particles::{
    BirdEmitter, BirdEmitterDesc, ButterflyEmitter, ButterflyEmitterDesc, LeafEmitterDesc,
    ParticleForces, ParticleSnapshot, ParticleSystem, PARTICLE_CAPACITY,
};
use crate::procedual_placer::{generate_positions, PlacerDesc};
use crate::tracer::{TerrainRayQuery, Tracer, TracerDesc};
use crate::tree_gen::{Tree, TreeDesc};
use crate::util::{cluster_positions, TimeInfo, BENCH};
use crate::util::{get_sun_dir, ShaderCompiler};
use crate::vkn::{Allocator, CommandBuffer, Device, Fence, Semaphore, SwapchainDesc};
use crate::{
    egui_renderer::EguiRenderer,
    vkn::{Swapchain, VulkanContext, VulkanContextDesc},
    window::{WindowMode, WindowState, WindowStateDesc},
};
use anyhow::{Context, Result};
use ash::vk;
use egui::{Color32, ColorImage, FontData, FontDefinitions, FontFamily, RichText, TextureHandle};
use glam::{UVec3, Vec2, Vec3, Vec4};
use gpu_allocator::vulkan::AllocatorCreateDesc;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use ui_style::{
    apply_gui_style, draw_item_panel, CUSTOM_GUI_FONT_NAME, CUSTOM_GUI_FONT_PATH, FLOWER_ACCENT,
    GOLD_ACCENT, ITEM_PANEL_SHOVEL_ICON_FALLBACK_PATH, ITEM_PANEL_SHOVEL_ICON_PATH,
    ITEM_PANEL_SLOT_COUNT, PANEL_BG, PANEL_DARK, SAGE_ACCENT, SHADOW_COLOR, SHOVEL_SLOT_INDEX,
};
use winit::event::DeviceEvent;
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::KeyCode,
    window::WindowId,
};

const LEAF_CLUSTER_DISTANCE: f32 = 0.08;

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

struct CompiledFencePlacement {
    voxel_edit: VoxelEdit,
    rebuild_bound: UAabb3,
}

struct CompiledTerrainSurfaceRemoval {
    voxel_edit: VoxelEdit,
    rebuild_bound: UAabb3,
}

struct FencePostPlacementService;

impl FencePostPlacementService {
    fn compile(edit: FencePostPlacementEdit, terrain_height: f32) -> CompiledFencePlacement {
        let downward_offset = edit.height * 0.4;
        let mut base = Vec3::new(edit.horizontal.x, terrain_height, edit.horizontal.y) * 256.0;
        base.y -= downward_offset;

        let cuboid = Cuboid::new(
            base + Vec3::Y * (edit.height * 0.5),
            Vec3::new(edit.half_width, edit.height * 0.5, edit.half_depth),
        );
        let cuboids = vec![cuboid];
        let leaves_data_sequential = vec![0_u32];
        let aabbs = vec![cuboids[0].aabb()];
        let bvh_nodes = build_bvh(&aabbs, &leaves_data_sequential).unwrap();
        let rebuild_bound =
            UAabb3::new(bvh_nodes[0].aabb.min_uvec3(), bvh_nodes[0].aabb.max_uvec3());

        CompiledFencePlacement {
            voxel_edit: VoxelEdit::StampCuboids {
                bvh_nodes,
                cuboids,
                voxel_type: VOXEL_TYPE_OAK_WOOD,
            },
            rebuild_bound,
        }
    }
}

#[allow(dead_code)]
struct CubePlacementService;

impl CubePlacementService {
    #[allow(dead_code)]
    fn compile(edit: CubePlacementEdit) -> CompiledFencePlacement {
        let half_extent = edit.size * 0.5;
        let cuboid = Cuboid::new(
            edit.center * 256.0,
            Vec3::new(half_extent, half_extent, half_extent),
        );
        let cuboids = vec![cuboid];
        let leaves_data_sequential = vec![0_u32];
        let aabbs = vec![cuboids[0].aabb()];
        let bvh_nodes = build_bvh(&aabbs, &leaves_data_sequential).unwrap();
        let rebuild_bound =
            UAabb3::new(bvh_nodes[0].aabb.min_uvec3(), bvh_nodes[0].aabb.max_uvec3());

        CompiledFencePlacement {
            voxel_edit: VoxelEdit::StampCuboids {
                bvh_nodes,
                cuboids,
                voxel_type: edit.voxel_type,
            },
            rebuild_bound,
        }
    }
}

struct TerrainSurfaceRemovalService;

impl TerrainSurfaceRemovalService {
    fn compile(edit: TerrainRemovalEdit) -> Option<CompiledTerrainSurfaceRemoval> {
        if edit.radius <= 0.0 {
            return None;
        }

        let center_voxel = edit.center * 256.0;
        let radius_voxel = edit.radius * 256.0;
        let sphere = Sphere::new(center_voxel, radius_voxel);
        let world_dim = VOXEL_DIM_PER_CHUNK * CHUNK_DIM;
        let max_inclusive = world_dim - UVec3::ONE;
        let sphere_aabb = sphere.aabb();
        let min_f = sphere_aabb.min();
        let max_f = sphere_aabb.max();
        if max_f.x < 0.0
            || max_f.y < 0.0
            || max_f.z < 0.0
            || min_f.x > max_inclusive.x as f32
            || min_f.y > max_inclusive.y as f32
            || min_f.z > max_inclusive.z as f32
        {
            return None;
        }

        let min = UVec3::new(
            min_f.x.max(0.0).floor() as u32,
            min_f.y.max(0.0).floor() as u32,
            min_f.z.max(0.0).floor() as u32,
        )
        .min(max_inclusive);
        let max = UVec3::new(
            max_f.x.max(0.0).ceil() as u32,
            max_f.y.max(0.0).ceil() as u32,
            max_f.z.max(0.0).ceil() as u32,
        )
        .min(max_inclusive);
        if min.cmpgt(max).any() {
            return None;
        }

        let clipped_aabb = crate::geom::Aabb3::new(min.as_vec3(), max.as_vec3());
        let bvh_nodes = build_bvh(&[clipped_aabb], &[0_u32]).ok()?;
        let rebuild_bound = UAabb3::new(
            bvh_nodes[0].aabb.min_uvec3().min(max_inclusive),
            bvh_nodes[0].aabb.max_uvec3().min(max_inclusive),
        );

        Some(CompiledTerrainSurfaceRemoval {
            voxel_edit: VoxelEdit::StampSurfaceSpheres {
                bvh_nodes,
                spheres: vec![sphere],
                voxel_type: VOXEL_TYPE_EMPTY,
            },
            rebuild_bound,
        })
    }
}

struct CompiledTreePlacement {
    trunk_voxel_edit: VoxelEdit,
    rebuild_bound: UAabb3,
    tree_pos: Vec3,
    this_bound: UAabb3,
    quantized_leaf_positions: Vec<UVec3>,
    world_leaf_positions: Vec<Vec3>,
}

struct TreePlacementService;

impl TreePlacementService {
    fn compile(tree_desc: TreeDesc, tree_pos: Vec3, prev_bound: UAabb3) -> CompiledTreePlacement {
        let tree = Tree::new(tree_desc);
        let mut round_cones = Vec::with_capacity(tree.trunks().len());
        for tree_trunk in tree.trunks() {
            let mut round_cone = tree_trunk.clone();
            round_cone.transform(tree_pos * 256.0);
            round_cones.push(round_cone);
        }

        let leaves_data_sequential = (0..round_cones.len()).map(|i| i as u32).collect::<Vec<_>>();
        let aabbs = round_cones.iter().map(RoundCone::aabb).collect::<Vec<_>>();

        let bvh_nodes = build_bvh(&aabbs, &leaves_data_sequential).unwrap();
        let this_bound = UAabb3::new(bvh_nodes[0].aabb.min_uvec3(), bvh_nodes[0].aabb.max_uvec3());

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

        CompiledTreePlacement {
            trunk_voxel_edit: VoxelEdit::StampRoundCones {
                bvh_nodes,
                round_cones,
                voxel_type: VOXEL_TYPE_CHERRY_WOOD,
            },
            rebuild_bound: this_bound.union_with(&prev_bound),
            tree_pos,
            this_bound,
            quantized_leaf_positions,
            world_leaf_positions,
        }
    }
}

#[derive(Clone, Debug)]
struct TreeRecord {
    position: Vec3,
    bound: UAabb3,
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
    settings_panel_visible: bool,
    is_fly_mode: bool,
    item_panel_shovel_icon: Option<TextureHandle>,
    selected_item_panel_slot: usize,
    shovel_dig_held: bool,
    last_shovel_dig_time: Option<Instant>,

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

impl App {
    fn sync_cursor_with_panels(&mut self) {
        let any_panel_open = self.config_panel_visible || self.settings_panel_visible;
        self.window_state.set_cursor_visibility(any_panel_open);
        self.window_state.set_cursor_grab(!any_panel_open);
        if any_panel_open {
            self.shovel_dig_held = false;
        }
    }

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
            selected_item_panel_slot: 0,
            shovel_dig_held: false,
            last_shovel_dig_time: None,

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

            spatial_sound_manager,
            tree_audio_manager,
        };

        app.configure_gui_font()?;
        app.load_item_panel_icon()?;
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

    fn load_item_panel_icon(&mut self) -> Result<()> {
        let icon_path = if std::path::Path::new(ITEM_PANEL_SHOVEL_ICON_PATH).exists() {
            ITEM_PANEL_SHOVEL_ICON_PATH
        } else {
            log::warn!(
                "Item panel icon not found at {}. Falling back to {}",
                ITEM_PANEL_SHOVEL_ICON_PATH,
                ITEM_PANEL_SHOVEL_ICON_FALLBACK_PATH
            );
            ITEM_PANEL_SHOVEL_ICON_FALLBACK_PATH
        };

        let icon_bytes = std::fs::read(icon_path)
            .with_context(|| format!("Failed to read item panel icon from {icon_path}"))?;
        let icon_rgba = image::load_from_memory(&icon_bytes)
            .with_context(|| format!("Failed to decode item panel icon from {icon_path}"))?
            .to_rgba8();
        let icon_size = [icon_rgba.width() as usize, icon_rgba.height() as usize];
        let icon_pixels = icon_rgba.into_raw();
        let icon_image = ColorImage::from_rgba_unmultiplied(icon_size, &icon_pixels);

        let texture = self.egui_renderer.context().load_texture(
            "item_panel_wooden_shovel",
            icon_image,
            egui::TextureOptions::NEAREST,
        );
        self.item_panel_shovel_icon = Some(texture);
        Ok(())
    }

    fn generate_procedural_trees(&mut self) -> Result<()> {
        // clear all procedural trees (keep single tree with ID 0)
        self.clear_procedural_trees()?;
        // remove the standalone debug tree so only procedural forest remains
        self.remove_tree(self.single_tree_id)?;

        let prev_bound = self.prev_bound;
        self.execute_edit_plan(WorldEditPlan::with_voxel(VoxelEdit::ClearVoxelRegion(
            ClearVoxelRegionEdit {
                offset: prev_bound.min(),
                dim: prev_bound.max() - prev_bound.min(),
            },
        )))?;

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

        for tree_pos in tree_positions_3d.iter() {
            let mut tree_desc = self.debug_tree_desc.clone();
            tree_desc.seed = rng.random_range(1..10000);

            self.apply_tree_variations(&mut tree_desc, &mut rng);
            self.apply_tree_placement(TreePlacementEdit {
                tree_desc,
                placement: TreePlacement::World(*tree_pos),
                options: TreeAddOptions::default().with_new_id(),
            })?;
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
        let (sun_altitude, sun_azimuth) =
            environment::calculate_sun_position(time_of_day, latitude, season);
        self.gui_adjustables.sun_altitude.value = sun_altitude;
        self.gui_adjustables.sun_azimuth.value = sun_azimuth;
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
        let world_dim = VOXEL_DIM_PER_CHUNK * CHUNK_DIM;
        let world_bound = UAabb3::new(UVec3::ZERO, world_dim - UVec3::ONE);
        world_ops::execute_edit_plan_on_builders(
            plain_builder,
            surface_builder,
            contree_builder,
            scene_accel_builder,
            VOXEL_DIM_PER_CHUNK,
            WorldEditPlan {
                voxel_edits: vec![VoxelEdit::ClearVoxelRegion(ClearVoxelRegionEdit {
                    offset: UVec3::ZERO,
                    dim: world_dim,
                })],
                build_edits: vec![BuildEdit::RebuildMesh(world_bound)],
            },
        )?;

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

        // Clear prior tree voxels and rebuild affected data through the shared world-edit pipeline.
        let prev_bound = self.prev_bound;
        self.execute_edit_plan(WorldEditPlan {
            voxel_edits: vec![VoxelEdit::ClearVoxelRegion(ClearVoxelRegionEdit {
                offset: prev_bound.min(),
                dim: prev_bound.max() - prev_bound.min(),
            })],
            build_edits: vec![BuildEdit::RebuildMesh(prev_bound)],
        })?;

        Ok(())
    }

    fn add_tree(
        &mut self,
        tree_desc: TreeDesc,
        placement: TreePlacement,
        options: TreeAddOptions,
    ) -> Result<()> {
        self.apply_tree_placement(TreePlacementEdit {
            tree_desc,
            placement,
            options,
        })
    }

    fn plant_map_region_fence_posts(&mut self) -> Result<()> {
        const BASE_FENCE_HEIGHT: f32 = 60.0;
        const FENCE_HEIGHT_SCALE: f32 = 0.45;
        const FENCE_HEIGHT: f32 = BASE_FENCE_HEIGHT * FENCE_HEIGHT_SCALE;
        const BASE_FENCE_SIZE: f32 = 10.0;
        const FENCE_SIZE_SCALE: f32 = 0.3;
        const POST_HALF_WIDTH: f32 = BASE_FENCE_SIZE * FENCE_SIZE_SCALE * 0.5;
        const POST_HALF_DEPTH: f32 = POST_HALF_WIDTH;
        const BORDER_PADDING: f32 = 0.2;
        const EDGE_INTERIOR_COLUMNS: u32 = 30;

        let map_size = CHUNK_DIM.as_vec3();
        let min_x = BORDER_PADDING;
        let max_x = map_size.x - BORDER_PADDING;
        let min_z = BORDER_PADDING;
        let max_z = map_size.z - BORDER_PADDING;

        let edge_segments = EDGE_INTERIOR_COLUMNS + 1;
        let mut post_positions = Vec::with_capacity((4 * EDGE_INTERIOR_COLUMNS + 4) as usize);

        // Bottom edge: left to right, include both corners.
        for i in 0..=edge_segments {
            let t = i as f32 / edge_segments as f32;
            let x = min_x + (max_x - min_x) * t;
            post_positions.push(Vec2::new(x, min_z));
        }

        // Right edge: bottom to top, exclude bottom-right corner.
        for i in 1..=edge_segments {
            let t = i as f32 / edge_segments as f32;
            let z = min_z + (max_z - min_z) * t;
            post_positions.push(Vec2::new(max_x, z));
        }

        // Top edge: right to left, exclude top-right corner.
        for i in 1..=edge_segments {
            let t = i as f32 / edge_segments as f32;
            let x = max_x - (max_x - min_x) * t;
            post_positions.push(Vec2::new(x, max_z));
        }

        // Left edge: top to bottom, exclude both corners.
        for i in 1..edge_segments {
            let t = i as f32 / edge_segments as f32;
            let z = max_z - (max_z - min_z) * t;
            post_positions.push(Vec2::new(min_x, z));
        }

        // Place all fence posts.
        for horizontal in post_positions {
            self.apply_fence_post_placement(FencePostPlacementEdit {
                horizontal,
                height: FENCE_HEIGHT,
                half_width: POST_HALF_WIDTH,
                half_depth: POST_HALF_DEPTH,
            })?;
        }

        Ok(())
    }

    fn execute_edit_plan(&mut self, plan: WorldEditPlan) -> Result<()> {
        world_ops::execute_edit_plan_on_backend(self, plan)
    }

    fn apply_fence_post_placement(&mut self, edit: FencePostPlacementEdit) -> Result<()> {
        let terrain_height = self.tracer.query_terrain_height(edit.horizontal)?;
        let compiled = FencePostPlacementService::compile(edit, terrain_height);

        // Fence geometry is trunk voxels only; no leaf instance registration.
        self.execute_edit_plan(WorldEditPlan::with_voxel_and_build(
            compiled.voxel_edit,
            BuildEdit::RebuildMesh(compiled.rebuild_bound),
        ))
    }

    #[allow(dead_code)]
    fn apply_cube_placement(&mut self, edit: CubePlacementEdit) -> Result<()> {
        let compiled = CubePlacementService::compile(edit);
        self.execute_edit_plan(WorldEditPlan::with_voxel_and_build(
            compiled.voxel_edit,
            BuildEdit::RebuildMesh(compiled.rebuild_bound),
        ))
    }

    fn is_shovel_selected(&self) -> bool {
        self.selected_item_panel_slot == SHOVEL_SLOT_INDEX
    }

    fn apply_surface_terrain_removal(&mut self, edit: TerrainRemovalEdit) -> Result<()> {
        if let Some(compiled) = TerrainSurfaceRemovalService::compile(edit) {
            self.execute_edit_plan(WorldEditPlan::with_voxel_and_build(
                compiled.voxel_edit,
                BuildEdit::RebuildMesh(compiled.rebuild_bound),
            ))?;
        }
        Ok(())
    }

    fn query_camera_ray_terrain_intersection(&mut self, max_distance: f32) -> Result<Option<Vec3>> {
        if max_distance <= 0.0 {
            return Ok(None);
        }

        let origin = self.tracer.camera_position();
        let direction = self.tracer.camera_front();
        if direction.length_squared() <= f32::EPSILON {
            return Ok(None);
        }

        let sample = self
            .tracer
            .query_terrain_ray_with_validity(TerrainRayQuery { origin, direction })?;

        let distance = if sample.is_valid {
            (sample.position - origin).length()
        } else {
            f32::INFINITY
        };

        if sample.is_valid && distance <= max_distance {
            return Ok(Some(sample.position));
        }

        Ok(None)
    }

    fn try_shovel_dig(&mut self, now: Instant) {
        if self.window_state.is_cursor_visible() || !self.is_shovel_selected() {
            return;
        }

        if let Some(last_dig) = self.last_shovel_dig_time {
            if now.duration_since(last_dig) < SHOVEL_DIG_INTERVAL {
                return;
            }
        }

        match self.query_camera_ray_terrain_intersection(SHOVEL_RAY_QUERY_DISTANCE) {
            Ok(Some(center)) => {
                if let Err(err) = self.apply_surface_terrain_removal(TerrainRemovalEdit {
                    center,
                    radius: SHOVEL_REMOVE_RADIUS,
                }) {
                    log::error!("Failed to apply terrain removal: {}", err);
                    return;
                }
                self.last_shovel_dig_time = Some(now);
            }
            Ok(None) => {
                self.last_shovel_dig_time = Some(now);
            }
            Err(err) => {
                log::error!("Shovel carve attempt failed during terrain query: {}", err);
            }
        }
    }

    fn apply_tree_placement(&mut self, edit: TreePlacementEdit) -> Result<()> {
        let TreePlacementEdit {
            tree_desc,
            placement,
            options,
        } = edit;

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

        let compiled = TreePlacementService::compile(tree_desc, tree_pos, self.prev_bound);

        self.execute_edit_plan(WorldEditPlan::with_voxel(compiled.trunk_voxel_edit))?;
        self.tracer.add_tree_leaves(
            &mut self.surface_builder.resources,
            tree_id,
            &compiled.quantized_leaf_positions,
        )?;
        self.execute_edit_plan(WorldEditPlan::with_build(BuildEdit::RebuildMesh(
            compiled.rebuild_bound,
        )))?;

        self.prev_bound = compiled.rebuild_bound;

        // Cluster leaf positions once for both audio and particle systems
        let leaf_clusters =
            cluster_positions(&compiled.world_leaf_positions, LEAF_CLUSTER_DISTANCE);

        self.tree_audio_manager.add_tree_sources_from_clusters(
            tree_id,
            compiled.tree_pos,
            &leaf_clusters,
            false,
            true,
        )?;

        self.tree_records.insert(
            tree_id,
            TreeRecord {
                position: compiled.tree_pos,
                bound: compiled.this_bound,
            },
        );

        self.upsert_tree_leaf_emitter(
            tree_id,
            compiled.tree_pos,
            &compiled.this_bound,
            &leaf_clusters,
        );

        Ok(())
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
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if !self.window_state.is_cursor_visible() && button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            self.shovel_dig_held = true;
                            self.try_shovel_dig(Instant::now());
                        }
                        ElementState::Released => {
                            self.shovel_dig_held = false;
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
                    self.try_shovel_dig(Instant::now());
                }
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
                let item_panel_shovel_icon = self.item_panel_shovel_icon.clone();
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
                                                    0..=10,
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
