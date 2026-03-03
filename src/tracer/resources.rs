use crate::{
    flora::species,
    particles::{
        bird_spritesheet_sequence_def, BIRD_SPRITESHEET_HEIGHT, BIRD_SPRITESHEET_REL_PATH,
        BIRD_SPRITESHEET_SEQUENCE_ORDER, BIRD_SPRITESHEET_WIDTH, BIRD_TOTAL_FRAME_COUNT,
        BUTTERFLY_FRAMES_PER_VARIANT, PARTICLE_CAPACITY, PARTICLE_SPRITE_FRAME_DIM,
    },
    resource::Resource,
    tracer::{
        leaves_construct::generate_indexed_voxel_leaves, DenoiserResources,
        ExtentDependentResources, Vertex,
    },
    util::get_project_root,
    vkn::{
        Allocator, Buffer, BufferUsage, Device, Extent2D, Extent3D, ImageDesc, ShaderModule,
        Texture, TextureRegion, VulkanContext,
    },
};
use ash::vk;
use bytemuck::{Pod, Zeroable};
use glam::IVec3;
use resource_container_derive::ResourceContainer;
use std::path::Path;

type MeshGenerator = fn(bool) -> anyhow::Result<(Vec<Vertex>, Vec<u32>)>;

#[derive(ResourceContainer)]
pub struct FloraMeshResources {
    pub vertices: Resource<Buffer>,
    pub indices: Resource<Buffer>,
    pub indices_len: u32,
}

impl FloraMeshResources {
    pub fn new(
        device: Device,
        allocator: Allocator,
        is_lod_used: bool,
        generator: MeshGenerator,
    ) -> Self {
        let (vertices_data, indices_data) = generator(is_lod_used).unwrap();
        let indices_len = indices_data.len() as u32;

        let vertices = Buffer::new_sized(
            device.clone(),
            allocator.clone(),
            BufferUsage::from_flags(vk::BufferUsageFlags::VERTEX_BUFFER),
            gpu_allocator::MemoryLocation::CpuToGpu,
            (std::mem::size_of::<Vertex>() * vertices_data.len()) as u64,
        );
        vertices.fill(&vertices_data).unwrap();

        let indices = Buffer::new_sized(
            device.clone(),
            allocator.clone(),
            BufferUsage::from_flags(vk::BufferUsageFlags::INDEX_BUFFER),
            gpu_allocator::MemoryLocation::CpuToGpu,
            (std::mem::size_of::<u32>() * indices_data.len()) as u64,
        );
        indices.fill(&indices_data).unwrap();

        Self {
            vertices: Resource::new(vertices),
            indices: Resource::new(indices),
            indices_len,
        }
    }
}

#[derive(ResourceContainer)]
pub struct LeavesResources {
    pub vertices: Resource<Buffer>,
    pub indices: Resource<Buffer>,
    pub indices_len: u32,
}

impl LeavesResources {
    pub fn new(device: Device, allocator: Allocator, is_lod_used: bool) -> Self {
        // use default parameters for initial leaf generation
        Self::new_with_params(device, allocator, 0.5, 0.25, 8.0, 16.0, is_lod_used)
    }

    pub fn new_with_params(
        device: Device,
        allocator: Allocator,
        inner_density: f32,
        outer_density: f32,
        inner_radius: f32,
        outer_radius: f32,
        is_lod_used: bool,
    ) -> Self {
        // 1. Generate the indexed data for hollow sphere-shaped leaves.
        let (mut vertices_data, mut indices_data) = generate_indexed_voxel_leaves(
            inner_density,
            outer_density,
            inner_radius,
            outer_radius,
            is_lod_used,
        )
        .unwrap();

        // guard against empty data - create minimal buffers to avoid Vulkan validation errors
        if vertices_data.is_empty() {
            vertices_data.push(Vertex {
                packed_data: [0; 2],
            }); // Dummy vertex
        }
        if indices_data.is_empty() {
            indices_data.push(0); // Dummy index
        }

        let indices_len = if indices_data.len() == 1 && indices_data[0] == 0 {
            0 // Don't render anything if this was a dummy index
        } else {
            indices_data.len() as u32
        };

        // 2. Create and fill the vertex buffer.
        let vertices = Buffer::new_sized(
            device.clone(),
            allocator.clone(),
            BufferUsage::from_flags(vk::BufferUsageFlags::VERTEX_BUFFER),
            gpu_allocator::MemoryLocation::CpuToGpu,
            (std::mem::size_of::<Vertex>() * vertices_data.len()) as u64,
        );
        vertices.fill(&vertices_data).unwrap();

        // 3. Create and fill the index buffer.
        let indices = Buffer::new_sized(
            device.clone(),
            allocator.clone(),
            BufferUsage::from_flags(vk::BufferUsageFlags::INDEX_BUFFER),
            gpu_allocator::MemoryLocation::CpuToGpu,
            (std::mem::size_of::<u32>() * indices_data.len()) as u64,
        );
        indices.fill(&indices_data).unwrap();

        Self {
            vertices: Resource::new(vertices),
            indices: Resource::new(indices),
            indices_len,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct ParticleInstanceGpu {
    pub position: [u32; 3],
    pub size: f32,
    pub color: [f32; 4],
    pub tex_index: u32,
}

pub struct ParticleRendererResources {
    pub vertices: Resource<Buffer>,
    pub indices: Resource<Buffer>,
    pub indices_len: u32,
    pub instance_buffer: Resource<Buffer>,
    pub instance_count: u32,
}

impl ParticleRendererResources {
    pub fn new(device: Device, allocator: Allocator) -> Self {
        let instance_capacity = PARTICLE_CAPACITY as u32;
        let (vertices, indices, indices_len) =
            Self::create_particle_mesh(device.clone(), allocator.clone());

        let instance_buffer = Buffer::new_sized(
            device.clone(),
            allocator.clone(),
            BufferUsage::from_flags(vk::BufferUsageFlags::VERTEX_BUFFER),
            gpu_allocator::MemoryLocation::CpuToGpu,
            (std::mem::size_of::<ParticleInstanceGpu>() as u64) * instance_capacity as u64,
        );

        Self {
            vertices: Resource::new(vertices),
            indices: Resource::new(indices),
            indices_len,
            instance_buffer: Resource::new(instance_buffer),
            instance_count: 0,
        }
    }

    fn create_particle_mesh(device: Device, allocator: Allocator) -> (Buffer, Buffer, u32) {
        use crate::tracer::voxel_encoding::append_indexed_cube_data;

        let mut vertices_data = Vec::new();
        let mut indices_data = Vec::new();
        append_indexed_cube_data(
            &mut vertices_data,
            &mut indices_data,
            IVec3::ZERO,
            0,
            IVec3::ZERO,
            1,
            true,
        )
        .unwrap();

        let vertices = Buffer::new_sized(
            device.clone(),
            allocator.clone(),
            BufferUsage::from_flags(vk::BufferUsageFlags::VERTEX_BUFFER),
            gpu_allocator::MemoryLocation::CpuToGpu,
            (std::mem::size_of::<Vertex>() * vertices_data.len()) as u64,
        );
        vertices.fill(&vertices_data).unwrap();

        let indices = Buffer::new_sized(
            device.clone(),
            allocator.clone(),
            BufferUsage::from_flags(vk::BufferUsageFlags::INDEX_BUFFER),
            gpu_allocator::MemoryLocation::CpuToGpu,
            (std::mem::size_of::<u32>() * indices_data.len()) as u64,
        );
        indices.fill(&indices_data).unwrap();

        (vertices, indices, indices_data.len() as u32)
    }
}

#[derive(ResourceContainer)]
pub struct TracerResources {
    pub gui_input: Resource<Buffer>,
    pub sun_info: Resource<Buffer>,
    pub shading_info: Resource<Buffer>,
    pub camera_info: Resource<Buffer>,
    pub camera_info_prev_frame: Resource<Buffer>,
    pub shadow_camera_info: Resource<Buffer>,
    pub flora_growth_info: Resource<Buffer>,
    pub env_info: Resource<Buffer>,
    pub starlight_info: Resource<Buffer>,
    pub voxel_colors: Resource<Buffer>,
    pub god_ray_info: Resource<Buffer>,
    pub post_processing_info: Resource<Buffer>,
    pub player_collider_info: Resource<Buffer>,
    pub player_collision_result: Resource<Buffer>,
    pub terrain_query_count: Resource<Buffer>,
    pub terrain_query_info: Resource<Buffer>,
    pub terrain_query_result: Resource<Buffer>,

    pub flora_meshes: Vec<FloraMeshResources>,
    pub leaves_resources: LeavesResources,

    pub flora_meshes_lod: Vec<FloraMeshResources>,
    pub leaves_resources_lod: LeavesResources,

    pub shadow_map_tex: Resource<Texture>,
    pub shadow_map_tex_for_vsm_ping: Resource<Texture>,
    pub shadow_map_tex_for_vsm_pong: Resource<Texture>,

    pub sun_sprite_tex: Resource<Texture>,
    pub particle_lod_tex_lut: Resource<Texture>,

    pub scalar_bn: Resource<Texture>,
    pub unit_vec2_bn: Resource<Texture>,
    pub unit_vec3_bn: Resource<Texture>,
    pub weighted_cosine_bn: Resource<Texture>,
    pub fast_unit_vec3_bn: Resource<Texture>,
    pub fast_weighted_cosine_bn: Resource<Texture>,

    pub extent_dependent_resources: ExtentDependentResources,
    pub denoiser_resources: DenoiserResources,
}

impl TracerResources {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        vulkan_ctx: &VulkanContext,
        allocator: Allocator,
        tracer_sm: &ShaderModule,
        tracer_shadow_sm: &ShaderModule,
        composition_sm: &ShaderModule,
        temporal_sm: &ShaderModule,
        spatial_sm: &ShaderModule,
        god_ray_sm: &ShaderModule,
        post_processing_sm: &ShaderModule,
        player_collider_sm: &ShaderModule,
        terrain_query_sm: &ShaderModule,
        flora_vert_sm: &ShaderModule,
        rendering_extent: Extent2D,
        screen_extent: Extent2D,
        shadow_map_extent: Extent2D,
        max_terrain_queries: u32,
    ) -> Self {
        let device = vulkan_ctx.device();

        let gui_input_layout = tracer_sm.get_buffer_layout("U_GuiInput").unwrap();
        let gui_input = Buffer::from_buffer_layout(
            device.clone(),
            allocator.clone(),
            gui_input_layout.clone(),
            BufferUsage::empty(),
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let sun_info_layout = tracer_sm.get_buffer_layout("U_SunInfo").unwrap();
        let sun_info = Buffer::from_buffer_layout(
            device.clone(),
            allocator.clone(),
            sun_info_layout.clone(),
            BufferUsage::empty(),
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let shading_info_layout = tracer_sm.get_buffer_layout("U_ShadingInfo").unwrap();
        let shading_info = Buffer::from_buffer_layout(
            device.clone(),
            allocator.clone(),
            shading_info_layout.clone(),
            BufferUsage::empty(),
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let camera_info_layout = tracer_sm.get_buffer_layout("U_CameraInfo").unwrap();
        let camera_info = Buffer::from_buffer_layout(
            device.clone(),
            allocator.clone(),
            camera_info_layout.clone(),
            BufferUsage::empty(),
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let camera_info_prev_frame_layout = tracer_sm
            .get_buffer_layout("U_CameraInfoPrevFrame")
            .unwrap();
        let camera_info_prev_frame = Buffer::from_buffer_layout(
            device.clone(),
            allocator.clone(),
            camera_info_prev_frame_layout.clone(),
            BufferUsage::empty(),
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let shadow_camera_info_layout = tracer_shadow_sm
            .get_buffer_layout("U_ShadowCameraInfo")
            .unwrap();
        let shadow_camera_info = Buffer::from_buffer_layout(
            device.clone(),
            allocator.clone(),
            shadow_camera_info_layout.clone(),
            BufferUsage::empty(),
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let flora_growth_info_layout = flora_vert_sm
            .get_buffer_layout("U_FloraGrowthInfo")
            .unwrap();
        let flora_growth_info = Buffer::from_buffer_layout(
            device.clone(),
            allocator.clone(),
            flora_growth_info_layout.clone(),
            BufferUsage::empty(),
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let env_info_layout = tracer_sm.get_buffer_layout("U_EnvInfo").unwrap();
        let env_info = Buffer::from_buffer_layout(
            device.clone(),
            allocator.clone(),
            env_info_layout.clone(),
            BufferUsage::empty(),
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let starlight_info_layout = composition_sm.get_buffer_layout("U_StarlightInfo").unwrap();
        let starlight_info = Buffer::from_buffer_layout(
            device.clone(),
            allocator.clone(),
            starlight_info_layout.clone(),
            BufferUsage::empty(),
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let voxel_colors_layout = tracer_sm.get_buffer_layout("U_VoxelColors").unwrap();
        let voxel_colors = Buffer::from_buffer_layout(
            device.clone(),
            allocator.clone(),
            voxel_colors_layout.clone(),
            BufferUsage::empty(),
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let god_ray_info_layout = god_ray_sm.get_buffer_layout("U_GodRayInfo").unwrap();
        let god_ray_info = Buffer::from_buffer_layout(
            device.clone(),
            allocator.clone(),
            god_ray_info_layout.clone(),
            BufferUsage::empty(),
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let post_processing_info_layout = post_processing_sm
            .get_buffer_layout("U_PostProcessingInfo")
            .unwrap();
        let post_processing_info = Buffer::from_buffer_layout(
            device.clone(),
            allocator.clone(),
            post_processing_info_layout.clone(),
            BufferUsage::empty(),
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let player_collider_info_layout = player_collider_sm
            .get_buffer_layout("U_PlayerColliderInfo")
            .unwrap();
        let player_collider_info = Buffer::from_buffer_layout(
            device.clone(),
            allocator.clone(),
            player_collider_info_layout.clone(),
            BufferUsage::empty(),
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let player_collision_result_layout = player_collider_sm
            .get_buffer_layout("B_PlayerCollisionResult")
            .unwrap();

        let player_collision_result = Buffer::from_buffer_layout(
            device.clone(),
            allocator.clone(),
            player_collision_result_layout.clone(),
            BufferUsage::empty(),
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let terrain_query_count_layout = terrain_query_sm
            .get_buffer_layout("U_TerrainQueryCount")
            .unwrap();
        let terrain_query_count = Buffer::from_buffer_layout(
            device.clone(),
            allocator.clone(),
            terrain_query_count_layout.clone(),
            BufferUsage::empty(),
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let terrain_query_info = Buffer::new_sized(
            device.clone(),
            allocator.clone(),
            BufferUsage::from_flags(vk::BufferUsageFlags::STORAGE_BUFFER),
            gpu_allocator::MemoryLocation::CpuToGpu,
            (max_terrain_queries * 8 * std::mem::size_of::<f32>() as u32) as u64,
        );

        let terrain_query_result = Buffer::new_sized(
            device.clone(),
            allocator.clone(),
            BufferUsage::from_flags(vk::BufferUsageFlags::STORAGE_BUFFER),
            gpu_allocator::MemoryLocation::CpuToGpu,
            (max_terrain_queries * 4 * std::mem::size_of::<f32>() as u32) as u64,
        );

        let shadow_map_tex = Self::create_shadow_map_tex(
            device.clone(),
            allocator.clone(),
            shadow_map_extent.into(),
        );
        let shadow_map_tex_for_vsm_ping = Self::create_shadow_map_tex_for_vsm_pingpong(
            device.clone(),
            allocator.clone(),
            shadow_map_extent.into(),
        );
        let shadow_map_tex_for_vsm_pong = Self::create_shadow_map_tex_for_vsm_pingpong(
            device.clone(),
            allocator.clone(),
            shadow_map_extent.into(),
        );

        let sun_sprite_tex = Self::create_sun_sprite_tex(vulkan_ctx, allocator.clone());
        let particle_lod_tex_lut = Self::create_particle_lod_tex_lut(vulkan_ctx, allocator.clone());

        let extent_dependent_resources = ExtentDependentResources::new(
            device.clone(),
            allocator.clone(),
            rendering_extent,
            screen_extent,
        );

        let scalar_bn = create_bn(
            vulkan_ctx,
            allocator.clone(),
            vk::Format::R8_UNORM,
            "stbn/scalar_2d_1d_1d/stbn_scalar_2Dx1Dx1D_128x128x64x1_",
        );
        let unit_vec2_bn = create_bn(
            vulkan_ctx,
            allocator.clone(),
            vk::Format::R8G8_UNORM,
            "stbn/unitvec2_2d_1d/stbn_unitvec2_2Dx1D_128x128x64_",
        );
        let unit_vec3_bn = create_bn(
            vulkan_ctx,
            allocator.clone(),
            vk::Format::R8G8B8A8_UNORM,
            "stbn/unitvec3_2d_1d/stbn_unitvec3_2Dx1D_128x128x64_",
        );
        let weighted_cosine_bn = create_bn(
            vulkan_ctx,
            allocator.clone(),
            vk::Format::R8G8B8A8_UNORM,
            "stbn/unitvec3_cosine_2d_1d/stbn_unitvec3_cosine_2Dx1D_128x128x64_",
        );
        let fast_unit_vec3_bn = create_bn(
            vulkan_ctx,
            allocator.clone(),
            vk::Format::R8G8B8A8_UNORM,
            "fast/unit_vec3/out_",
        );
        let fast_weighted_cosine_bn = create_bn(
            vulkan_ctx,
            allocator.clone(),
            vk::Format::R8G8B8A8_UNORM,
            "fast/weighted_cosine/out_",
        );

        species::assert_species_limit();
        let flora_meshes = species::species()
            .iter()
            .map(|desc| {
                FloraMeshResources::new(
                    device.clone(),
                    allocator.clone(),
                    false,
                    desc.mesh_generator,
                )
            })
            .collect::<Vec<_>>();
        let leaves_resources = LeavesResources::new(device.clone(), allocator.clone(), false);
        let flora_meshes_lod = species::species()
            .iter()
            .map(|desc| {
                FloraMeshResources::new(
                    device.clone(),
                    allocator.clone(),
                    true,
                    desc.mesh_generator,
                )
            })
            .collect::<Vec<_>>();
        let leaves_resources_lod = LeavesResources::new(device.clone(), allocator.clone(), true);

        return Self {
            gui_input: Resource::new(gui_input),
            sun_info: Resource::new(sun_info),
            shading_info: Resource::new(shading_info),
            camera_info: Resource::new(camera_info),
            camera_info_prev_frame: Resource::new(camera_info_prev_frame),
            shadow_camera_info: Resource::new(shadow_camera_info),
            flora_growth_info: Resource::new(flora_growth_info),
            env_info: Resource::new(env_info),
            starlight_info: Resource::new(starlight_info),
            voxel_colors: Resource::new(voxel_colors),
            god_ray_info: Resource::new(god_ray_info),
            post_processing_info: Resource::new(post_processing_info),
            player_collider_info: Resource::new(player_collider_info),
            player_collision_result: Resource::new(player_collision_result),
            terrain_query_count: Resource::new(terrain_query_count),
            terrain_query_info: Resource::new(terrain_query_info),
            terrain_query_result: Resource::new(terrain_query_result),
            flora_meshes,
            leaves_resources,
            flora_meshes_lod,
            leaves_resources_lod,
            extent_dependent_resources,
            shadow_map_tex: Resource::new(shadow_map_tex),
            shadow_map_tex_for_vsm_ping: Resource::new(shadow_map_tex_for_vsm_ping),
            shadow_map_tex_for_vsm_pong: Resource::new(shadow_map_tex_for_vsm_pong),
            sun_sprite_tex: Resource::new(sun_sprite_tex),
            particle_lod_tex_lut: Resource::new(particle_lod_tex_lut),
            scalar_bn: Resource::new(scalar_bn),
            unit_vec2_bn: Resource::new(unit_vec2_bn),
            unit_vec3_bn: Resource::new(unit_vec3_bn),
            weighted_cosine_bn: Resource::new(weighted_cosine_bn),
            fast_unit_vec3_bn: Resource::new(fast_unit_vec3_bn),
            fast_weighted_cosine_bn: Resource::new(fast_weighted_cosine_bn),
            denoiser_resources: DenoiserResources::new(
                device.clone(),
                allocator.clone(),
                rendering_extent,
                temporal_sm,
                spatial_sm,
            ),
        };

        fn create_bn(
            vulkan_ctx: &VulkanContext,
            allocator: Allocator,
            format: vk::Format,
            relative_path: &str,
        ) -> Texture {
            const BLUE_NOISE_LEN: u32 = 64;

            let img_desc = ImageDesc {
                extent: Extent3D::new(128, 128, 1),
                array_len: BLUE_NOISE_LEN,
                format,
                usage: vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_DST,
                initial_layout: vk::ImageLayout::UNDEFINED,
                aspect: vk::ImageAspectFlags::COLOR,
                ..Default::default()
            };
            let sam_desc = Default::default();
            let tex = Texture::new(vulkan_ctx.device().clone(), allocator, &img_desc, &sam_desc);

            let base_path = get_project_root() + "/texture/";
            for i in 0..BLUE_NOISE_LEN {
                let path = format!("{}{}{}.png", base_path, relative_path, i);
                tex.get_image()
                    .load_and_fill(
                        &vulkan_ctx.get_general_queue(),
                        vulkan_ctx.command_pool(),
                        &path,
                        i,
                        Some(vk::ImageLayout::GENERAL),
                    )
                    .unwrap();
            }
            tex
        }
    }

    pub fn on_resize(
        &mut self,
        device: Device,
        allocator: Allocator,
        rendering_extent: Extent2D,
        screen_extent: Extent2D,
    ) {
        self.extent_dependent_resources.on_resize(
            device,
            allocator,
            rendering_extent,
            screen_extent,
        );
        self.denoiser_resources.on_resize(rendering_extent);
    }

    fn create_sun_sprite_tex(vulkan_ctx: &VulkanContext, allocator: Allocator) -> Texture {
        const SUN_SPRITE_REL_PATH: &str = "assets/texture/Planets_16x16/Sun.png";

        let path = get_project_root() + "/" + SUN_SPRITE_REL_PATH;
        if !Path::new(&path).exists() {
            panic!("Sun sprite texture missing at '{}'", path);
        }
        let image = image::open(&path).unwrap_or_else(|e| {
            panic!("Failed to open sun sprite texture '{}': {}", path, e);
        });
        let rgba = image.to_rgba8();
        let (w, h) = rgba.dimensions();
        let extent = Extent2D::new(w, h);
        let texels_rgba = rgba.into_raw();

        let img_desc = ImageDesc {
            extent: extent.into(),
            array_len: 1,
            format: vk::Format::R8G8B8A8_SRGB,
            usage: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            initial_layout: vk::ImageLayout::UNDEFINED,
            aspect: vk::ImageAspectFlags::COLOR,
            ..Default::default()
        };
        let sam_desc = Default::default();
        let tex = Texture::new(vulkan_ctx.device().clone(), allocator, &img_desc, &sam_desc);

        tex.get_image()
            .fill_with_raw_u8(
                &vulkan_ctx.get_general_queue(),
                vulkan_ctx.command_pool(),
                TextureRegion::from_image(tex.get_image()),
                &texels_rgba,
                0,
                Some(vk::ImageLayout::GENERAL),
            )
            .unwrap();
        tex
    }

    fn create_particle_lod_tex_lut(vulkan_ctx: &VulkanContext, allocator: Allocator) -> Texture {
        const PARTICLE_LOD_TEXTURE_DIR_REL_PATH: &str = "assets/texture/butterfly_16px";
        const LUT_LAYER_LEAF: u32 = 0;
        let frame_dim = PARTICLE_SPRITE_FRAME_DIM;

        let white = [255u8, 255u8, 255u8, 255u8];
        let white_layer = white
            .repeat((frame_dim * frame_dim) as usize)
            .into_iter()
            .collect::<Vec<u8>>();
        let dir_path = get_project_root() + "/" + PARTICLE_LOD_TEXTURE_DIR_REL_PATH;
        let mut atlas_paths = std::fs::read_dir(&dir_path)
            .ok()
            .into_iter()
            .flat_map(|entries| entries.filter_map(Result::ok))
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
            })
            .collect::<Vec<_>>();
        atlas_paths.sort();

        assert!(
            !atlas_paths.is_empty(),
            "Butterfly atlas not found in '{}'",
            dir_path
        );
        assert!(
            atlas_paths.len() == 1,
            "Expected exactly one butterfly atlas in '{}', found {}",
            dir_path,
            atlas_paths.len()
        );

        let butterfly_atlas_path = &atlas_paths[0];
        let atlas_path_str = butterfly_atlas_path.to_string_lossy().to_string();
        let atlas = image::open(butterfly_atlas_path)
            .unwrap_or_else(|_| panic!("Failed to open butterfly atlas '{}'", atlas_path_str));
        let rgba = atlas.to_rgba8();
        let (width, height) = rgba.dimensions();
        let expected_size = frame_dim * 5;
        assert!(
            width == expected_size && height == expected_size,
            "Butterfly atlas must be {}x{}, got {}x{}",
            expected_size,
            expected_size,
            width,
            height
        );

        let mut butterfly_layers = Vec::new();
        for row in 0..5 {
            if let Some(frames) = Self::extract_row_sequence_layers(
                &rgba,
                row,
                BUTTERFLY_FRAMES_PER_VARIANT,
                &atlas_path_str,
            ) {
                butterfly_layers.extend(frames);
            } else {
                panic!(
                    "Failed to extract butterfly frames from row {} of '{}'",
                    row, atlas_path_str
                );
            }
        }
        let bird_path = get_project_root() + "/" + BIRD_SPRITESHEET_REL_PATH;
        let mut bird_layers = Vec::with_capacity(BIRD_TOTAL_FRAME_COUNT as usize);
        match image::open(&bird_path) {
            Ok(image) => {
                let rgba = image.to_rgba8();
                let (sheet_width, sheet_height) = rgba.dimensions();
                if sheet_width != BIRD_SPRITESHEET_WIDTH || sheet_height != BIRD_SPRITESHEET_HEIGHT
                {
                    log::warn!(
                        "Bird spritesheet '{}' is {}x{}; expected {}x{}",
                        bird_path,
                        sheet_width,
                        sheet_height,
                        BIRD_SPRITESHEET_WIDTH,
                        BIRD_SPRITESHEET_HEIGHT
                    );
                }
                for sequence in BIRD_SPRITESHEET_SEQUENCE_ORDER {
                    let def = bird_spritesheet_sequence_def(sequence);
                    let source_label = format!("{} ({:?})", bird_path, sequence);
                    if let Some(frames) = Self::extract_row_sequence_layers(
                        &rgba,
                        def.row,
                        def.frame_count,
                        &source_label,
                    ) {
                        bird_layers.extend(frames);
                    } else {
                        log::warn!(
                            "Bird sprite sequence {:?} could not be extracted; using fallback frames",
                            sequence
                        );
                        for _ in 0..def.frame_count {
                            bird_layers.push(white_layer.clone());
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to open bird spritesheet '{}': {}; using fallback texture",
                    bird_path,
                    e
                );
                for _ in 0..BIRD_TOTAL_FRAME_COUNT {
                    bird_layers.push(white_layer.clone());
                }
            }
        }
        if bird_layers.len() as u32 != BIRD_TOTAL_FRAME_COUNT {
            log::warn!(
                "Bird animation frame count mismatch (got {}, expected {}); padding with fallback",
                bird_layers.len(),
                BIRD_TOTAL_FRAME_COUNT
            );
        }
        while (bird_layers.len() as u32) < BIRD_TOTAL_FRAME_COUNT {
            bird_layers.push(white_layer.clone());
        }
        if (bird_layers.len() as u32) > BIRD_TOTAL_FRAME_COUNT {
            bird_layers.truncate(BIRD_TOTAL_FRAME_COUNT as usize);
        }
        if bird_layers.is_empty() {
            for _ in 0..BIRD_TOTAL_FRAME_COUNT {
                bird_layers.push(white_layer.clone());
            }
        }
        let lut_layer_count = 1 + butterfly_layers.len() as u32 + bird_layers.len() as u32;

        let sam_desc = Default::default();
        let img_desc = ImageDesc {
            extent: Extent3D::new(frame_dim, frame_dim, 1),
            array_len: lut_layer_count,
            format: vk::Format::R8G8B8A8_SRGB,
            usage: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            initial_layout: vk::ImageLayout::UNDEFINED,
            aspect: vk::ImageAspectFlags::COLOR,
            ..Default::default()
        };
        let tex = Texture::new(vulkan_ctx.device().clone(), allocator, &img_desc, &sam_desc);

        Self::fill_particle_lut_layer(vulkan_ctx, &tex, LUT_LAYER_LEAF, &white_layer);
        for (frame_idx, frame_data) in butterfly_layers.iter().enumerate() {
            Self::fill_particle_lut_layer(
                vulkan_ctx,
                &tex,
                (frame_idx as u32) + 1,
                frame_data.as_slice(),
            );
        }
        let bird_start_layer = 1 + butterfly_layers.len() as u32;
        for (frame_idx, frame_data) in bird_layers.iter().enumerate() {
            Self::fill_particle_lut_layer(
                vulkan_ctx,
                &tex,
                bird_start_layer + frame_idx as u32,
                frame_data.as_slice(),
            );
        }

        tex
    }

    fn extract_row_sequence_layers(
        atlas: &image::RgbaImage,
        row: u32,
        target_frame_count: u32,
        source_label: &str,
    ) -> Option<Vec<Vec<u8>>> {
        if target_frame_count == 0 {
            return Some(Vec::new());
        }

        let frame_dim = PARTICLE_SPRITE_FRAME_DIM;
        let (width, height) = atlas.dimensions();
        let row_y = row.saturating_mul(frame_dim);
        if width < frame_dim || height < row_y.saturating_add(frame_dim) {
            log::warn!(
                "Animated texture '{}' is {}x{}; row {} with {}x{} frames is unavailable",
                source_label,
                width,
                height,
                row,
                frame_dim,
                frame_dim
            );
            return None;
        }

        if width % frame_dim != 0 {
            log::warn!(
                "Animated texture '{}' width {} is not divisible by frame size {}; ignoring trailing pixels",
                source_label,
                width,
                frame_dim
            );
        }

        let available_frames = (width / frame_dim).max(1);
        let mut frames = Vec::with_capacity(target_frame_count as usize);
        for target_frame_idx in 0..target_frame_count {
            let src_frame_idx = target_frame_idx.min(available_frames - 1);
            let frame = image::imageops::crop_imm(
                atlas,
                src_frame_idx * frame_dim,
                row_y,
                frame_dim,
                frame_dim,
            )
            .to_image();
            frames.push(Self::to_particle_frame_bytes(frame));
        }
        Some(frames)
    }

    fn to_particle_frame_bytes(mut frame: image::RgbaImage) -> Vec<u8> {
        for pixel in frame.pixels_mut() {
            if pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 0 {
                pixel[3] = 0;
            }
        }
        frame.into_raw()
    }

    fn fill_particle_lut_layer(vulkan_ctx: &VulkanContext, tex: &Texture, layer: u32, data: &[u8]) {
        tex.get_image()
            .fill_with_raw_u8(
                &vulkan_ctx.get_general_queue(),
                vulkan_ctx.command_pool(),
                TextureRegion::from_image(tex.get_image()),
                data,
                layer,
                Some(vk::ImageLayout::GENERAL),
            )
            .unwrap();
    }

    fn create_shadow_map_tex(
        device: Device,
        allocator: Allocator,
        shadow_map_extent: Extent3D,
    ) -> Texture {
        let tex_desc = ImageDesc {
            extent: shadow_map_extent,
            format: vk::Format::D32_SFLOAT,
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
                | vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::TRANSFER_DST,
            initial_layout: vk::ImageLayout::UNDEFINED,
            aspect: vk::ImageAspectFlags::DEPTH,
            ..Default::default()
        };
        let sam_desc = Default::default();
        Texture::new(device, allocator, &tex_desc, &sam_desc)
    }

    fn create_shadow_map_tex_for_vsm_pingpong(
        device: Device,
        allocator: Allocator,
        shadow_map_extent: Extent3D,
    ) -> Texture {
        let tex_desc = ImageDesc {
            extent: shadow_map_extent,
            format: vk::Format::R32G32B32A32_SFLOAT,
            usage: vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            initial_layout: vk::ImageLayout::UNDEFINED,
            aspect: vk::ImageAspectFlags::COLOR,
            ..Default::default()
        };
        let sam_desc = Default::default();
        Texture::new(device, allocator, &tex_desc, &sam_desc)
    }
}
