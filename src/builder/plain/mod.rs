mod resources;
use crate::geom::{BvhNode, Cuboid, RoundCone, Sphere};
use crate::util::ShaderCompiler;
use crate::vkn::execute_one_time_command;
use crate::vkn::Allocator;
use crate::vkn::Buffer;
use crate::vkn::ClearValue;
use crate::vkn::ColorClearValue;
use crate::vkn::CommandBuffer;
use crate::vkn::ComputePipeline;
use crate::vkn::DescriptorPool;
use crate::vkn::Extent3D;
use crate::vkn::MemoryBarrier;
use crate::vkn::PipelineBarrier;
use crate::vkn::PlainMemberTypeWithData;
use crate::vkn::ShaderModule;
use crate::vkn::StructMemberDataBuilder;
use crate::vkn::Texture;
use crate::vkn::VulkanContext;
use anyhow::Result;
use ash::vk;
use glam::UVec3;
pub use resources::*;
use std::convert::TryInto;

pub const VOXEL_TYPE_CHERRY_WOOD: u32 = 5;
pub const VOXEL_TYPE_OAK_WOOD: u32 = 6;
pub const VOXEL_TYPE_ROCK: u32 = 7;
pub const VOXEL_TYPE_EMPTY: u32 = 0;
pub const VOXEL_TYPE_DIRT: u32 = 2;
const PRIMITIVE_KIND_ROUND_CONE: u32 = 0;
const PRIMITIVE_KIND_CUBOID: u32 = 1;
const PRIMITIVE_KIND_SPHERE: u32 = 2;
const EDIT_STATS_VOXEL_TYPE_COUNT: usize = 8;

#[derive(Clone, Copy, Debug, Default)]
pub struct ChunkModifyStats {
    pub removed_counts: [u32; EDIT_STATS_VOXEL_TYPE_COUNT],
    pub added_counts: [u32; EDIT_STATS_VOXEL_TYPE_COUNT],
}

impl ChunkModifyStats {
    pub fn count_removed(&self, voxel_type: u32) -> u32 {
        self.removed_counts
            .get(voxel_type as usize)
            .copied()
            .unwrap_or(0)
    }

    pub fn count_added(&self, voxel_type: u32) -> u32 {
        self.added_counts
            .get(voxel_type as usize)
            .copied()
            .unwrap_or(0)
    }
}

pub struct PlainBuilder {
    vulkan_ctx: VulkanContext,
    resources: PlainBuilderResources,

    #[allow(dead_code)]
    buffer_setup_ppl: ComputePipeline,
    #[allow(dead_code)]
    chunk_init_ppl: ComputePipeline,
    chunk_modify_ppl: ComputePipeline,

    #[allow(dead_code)]
    pool: DescriptorPool,

    build_cmdbuf: CommandBuffer,
}

impl PlainBuilder {
    pub fn new(
        vulkan_ctx: VulkanContext,
        shader_compiler: &ShaderCompiler,
        allocator: Allocator,
        plain_atlas_dim: UVec3,
        free_atlas_dim: UVec3,
    ) -> Self {
        let device = vulkan_ctx.device();

        let buffer_setup_sm = ShaderModule::from_glsl(
            device,
            shader_compiler,
            "shader/builder/chunk_writer/buffer_setup.comp",
            "main",
        )
        .unwrap();
        let chunk_init_sm = ShaderModule::from_glsl(
            device,
            shader_compiler,
            "shader/builder/chunk_writer/chunk_init.comp",
            "main",
        )
        .unwrap();
        let chunk_modify_sm = ShaderModule::from_glsl(
            device,
            shader_compiler,
            "shader/builder/chunk_writer/chunk_modify.comp",
            "main",
        )
        .unwrap();

        let resources = PlainBuilderResources::new(
            device,
            allocator.clone(),
            plain_atlas_dim,
            free_atlas_dim,
            &buffer_setup_sm,
            &chunk_modify_sm,
        );

        let pool = DescriptorPool::new(device).unwrap();

        let buffer_setup_ppl = ComputePipeline::new(device, &buffer_setup_sm, &pool, &[&resources]);
        let chunk_init_ppl = ComputePipeline::new(device, &chunk_init_sm, &pool, &[&resources]);
        let chunk_modify_ppl = ComputePipeline::new(device, &chunk_modify_sm, &pool, &[&resources]);

        init_atlas_images(&vulkan_ctx, &resources);

        let build_cmdbuf = Self::record_build_cmdbuf(
            &vulkan_ctx,
            &resources.chunk_atlas,
            &resources.region_indirect,
            &buffer_setup_ppl,
            &chunk_init_ppl,
        );

        return Self {
            vulkan_ctx,
            resources,
            buffer_setup_ppl,
            chunk_init_ppl,
            chunk_modify_ppl,
            pool,
            build_cmdbuf,
        };

        fn init_atlas_images(vulkan_context: &VulkanContext, resources: &PlainBuilderResources) {
            execute_one_time_command(
                vulkan_context.device(),
                vulkan_context.command_pool(),
                &vulkan_context.get_general_queue(),
                |cmdbuf| {
                    resources.chunk_atlas.get_image().record_clear(
                        cmdbuf,
                        Some(vk::ImageLayout::GENERAL),
                        0,
                        ClearValue::Color(ColorClearValue::UInt([0, 0, 0, 0])),
                    );
                    resources.free_atlas.get_image().record_clear(
                        cmdbuf,
                        Some(vk::ImageLayout::GENERAL),
                        0,
                        ClearValue::Color(ColorClearValue::UInt([0, 0, 0, 0])),
                    );
                },
            );
        }
    }

    fn record_build_cmdbuf(
        vulkan_ctx: &VulkanContext,
        chunk_atlas: &Texture,
        region_indirect: &Buffer,
        buffer_setup_ppl: &ComputePipeline,
        chunk_init_ppl: &ComputePipeline,
    ) -> CommandBuffer {
        let shader_access_memory_barrier = MemoryBarrier::new_shader_access();
        let indirect_access_memory_barrier = MemoryBarrier::new_indirect_access();

        let shader_access_pipeline_barrier = PipelineBarrier::new(
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vec![shader_access_memory_barrier],
        );
        let indirect_access_pipeline_barrier = PipelineBarrier::new(
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::DRAW_INDIRECT | vk::PipelineStageFlags::COMPUTE_SHADER,
            vec![indirect_access_memory_barrier],
        );

        let cmdbuf = CommandBuffer::new(vulkan_ctx.device(), vulkan_ctx.command_pool());
        cmdbuf.begin(false);

        chunk_atlas
            .get_image()
            .record_transition_barrier(&cmdbuf, 0, vk::ImageLayout::GENERAL);

        buffer_setup_ppl.record(
            &cmdbuf,
            Extent3D {
                width: 1,
                height: 1,
                depth: 1,
            },
            None,
        );

        shader_access_pipeline_barrier.record_insert(vulkan_ctx.device(), &cmdbuf);
        indirect_access_pipeline_barrier.record_insert(vulkan_ctx.device(), &cmdbuf);

        chunk_init_ppl.record_indirect(&cmdbuf, region_indirect, None);

        cmdbuf.end();
        cmdbuf
    }

    pub fn get_resources(&self) -> &PlainBuilderResources {
        &self.resources
    }

    pub fn chunk_init(&mut self, atlas_offset: UVec3, atlas_dim: UVec3) -> Result<()> {
        if atlas_dim.x == 0 || atlas_dim.y == 0 || atlas_dim.z == 0 {
            return Ok(());
        }
        update_buffers(&self.resources, atlas_offset, atlas_dim)?;

        // re-record the command buffer with updated descriptor sets
        self.build_cmdbuf = Self::record_build_cmdbuf(
            &self.vulkan_ctx,
            &self.resources.chunk_atlas,
            &self.resources.region_indirect,
            &self.buffer_setup_ppl,
            &self.chunk_init_ppl,
        );

        self.build_cmdbuf
            .submit(&self.vulkan_ctx.get_general_queue(), None);
        self.vulkan_ctx
            .device()
            .wait_queue_idle(&self.vulkan_ctx.get_general_queue());
        return Ok(());

        fn update_buffers(
            resources: &PlainBuilderResources,
            offset: UVec3,
            dim: UVec3,
        ) -> Result<()> {
            let data = StructMemberDataBuilder::from_buffer(&resources.region_info)
                .set_field("offset", PlainMemberTypeWithData::UVec3(offset.to_array()))
                .set_field("dim", PlainMemberTypeWithData::UVec3(dim.to_array()))
                .build()?;
            resources.region_info.fill_with_raw_u8(&data)?;
            Ok(())
        }
    }

    pub fn chunk_modify(&mut self, bvh_nodes: &[BvhNode], round_cones: &[RoundCone]) -> Result<()> {
        self.chunk_modify_with_voxel_type(bvh_nodes, round_cones, VOXEL_TYPE_CHERRY_WOOD)
    }

    pub fn chunk_modify_with_voxel_type(
        &mut self,
        bvh_nodes: &[BvhNode],
        round_cones: &[RoundCone],
        fill_voxel_type: u32,
    ) -> Result<()> {
        self.chunk_modify_round_cones_with_voxel_type(bvh_nodes, round_cones, fill_voxel_type)
    }

    pub fn chunk_modify_cuboids(
        &mut self,
        bvh_nodes: &[BvhNode],
        cuboids: &[Cuboid],
    ) -> Result<()> {
        self.chunk_modify_cuboids_with_voxel_type(bvh_nodes, cuboids, VOXEL_TYPE_CHERRY_WOOD)
    }

    pub fn chunk_modify_cuboids_with_voxel_type(
        &mut self,
        bvh_nodes: &[BvhNode],
        cuboids: &[Cuboid],
        fill_voxel_type: u32,
    ) -> Result<()> {
        self.chunk_modify_cuboids_with_voxel_type_impl(bvh_nodes, cuboids, fill_voxel_type)
    }

    pub fn chunk_modify_surface_spheres_with_voxel_type(
        &mut self,
        bvh_nodes: &[BvhNode],
        spheres: &[Sphere],
        fill_voxel_type: u32,
        max_write_count: Option<u32>,
    ) -> Result<ChunkModifyStats> {
        let (offset, dim) = calculate_offset_and_dim(bvh_nodes);
        clear_edit_stats(&self.resources)?;
        update_chunk_modify_info(
            &self.resources,
            offset,
            dim,
            fill_voxel_type,
            PRIMITIVE_KIND_SPHERE,
            true,
            max_write_count,
        )?;
        update_spheres(&self.resources, spheres)?;
        update_trunk_bvh_nodes(&self.resources, bvh_nodes)?;

        execute_one_time_command(
            self.vulkan_ctx.device(),
            self.vulkan_ctx.command_pool(),
            &self.vulkan_ctx.get_general_queue(),
            |cmdbuf| {
                self.chunk_modify_ppl.record(
                    cmdbuf,
                    Extent3D {
                        width: dim.x,
                        height: dim.y,
                        depth: dim.z,
                    },
                    None,
                );
            },
        );
        read_edit_stats(&self.resources)
    }

    fn chunk_modify_round_cones_with_voxel_type(
        &mut self,
        bvh_nodes: &[BvhNode],
        round_cones: &[RoundCone],
        fill_voxel_type: u32,
    ) -> Result<()> {
        let (offset, dim) = calculate_offset_and_dim(bvh_nodes);
        update_chunk_modify_info(
            &self.resources,
            offset,
            dim,
            fill_voxel_type,
            PRIMITIVE_KIND_ROUND_CONE,
            false,
            None,
        )?;
        update_round_cones(&self.resources, round_cones)?;
        update_trunk_bvh_nodes(&self.resources, bvh_nodes)?;

        execute_one_time_command(
            self.vulkan_ctx.device(),
            self.vulkan_ctx.command_pool(),
            &self.vulkan_ctx.get_general_queue(),
            |cmdbuf| {
                self.chunk_modify_ppl.record(
                    cmdbuf,
                    Extent3D {
                        width: dim.x,
                        height: dim.y,
                        depth: dim.z,
                    },
                    None,
                );
            },
        );
        Ok(())
    }

    fn chunk_modify_cuboids_with_voxel_type_impl(
        &mut self,
        bvh_nodes: &[BvhNode],
        cuboids: &[Cuboid],
        fill_voxel_type: u32,
    ) -> Result<()> {
        let (offset, dim) = calculate_offset_and_dim(bvh_nodes);
        update_chunk_modify_info(
            &self.resources,
            offset,
            dim,
            fill_voxel_type,
            PRIMITIVE_KIND_CUBOID,
            false,
            None,
        )?;
        update_cuboids(&self.resources, cuboids)?;
        update_trunk_bvh_nodes(&self.resources, bvh_nodes)?;

        execute_one_time_command(
            self.vulkan_ctx.device(),
            self.vulkan_ctx.command_pool(),
            &self.vulkan_ctx.get_general_queue(),
            |cmdbuf| {
                self.chunk_modify_ppl.record(
                    cmdbuf,
                    Extent3D {
                        width: dim.x,
                        height: dim.y,
                        depth: dim.z,
                    },
                    None,
                );
            },
        );
        Ok(())
    }
}

fn calculate_offset_and_dim(bvh_nodes: &[BvhNode]) -> (UVec3, UVec3) {
    let root_node = &bvh_nodes[0];
    (
        root_node.aabb.min_uvec3(),
        root_node.aabb.max_uvec3() - root_node.aabb.min_uvec3(),
    )
}

fn update_chunk_modify_info(
    resources: &PlainBuilderResources,
    offset: UVec3,
    dim: UVec3,
    fill_voxel_type: u32,
    primitive_kind: u32,
    surface_only: bool,
    max_write_count: Option<u32>,
) -> Result<()> {
    let data = StructMemberDataBuilder::from_buffer(&resources.chunk_modify_info)
        .set_field("offset", PlainMemberTypeWithData::UVec3(offset.to_array()))
        .set_field("dim", PlainMemberTypeWithData::UVec3(dim.to_array()))
        .set_field(
            "fill_voxel_type",
            PlainMemberTypeWithData::UInt(fill_voxel_type),
        )
        .set_field(
            "primitive_kind",
            PlainMemberTypeWithData::UInt(primitive_kind),
        )
        .set_field(
            "surface_only",
            PlainMemberTypeWithData::UInt(if surface_only { 1 } else { 0 }),
        )
        .set_field(
            "max_write_count",
            PlainMemberTypeWithData::UInt(max_write_count.unwrap_or(0)),
        )
        .build()?;
    resources.chunk_modify_info.fill_with_raw_u8(&data)?;
    Ok(())
}

fn clear_edit_stats(resources: &PlainBuilderResources) -> Result<()> {
    resources
        .edit_stats
        .fill_with_raw_u32(&[0; EDIT_STATS_VOXEL_TYPE_COUNT * 2])
}

fn read_edit_stats(resources: &PlainBuilderResources) -> Result<ChunkModifyStats> {
    let raw = resources.edit_stats.read_back()?;
    let expected_len = EDIT_STATS_VOXEL_TYPE_COUNT * 2 * std::mem::size_of::<u32>();
    if raw.len() < expected_len {
        return Err(anyhow::anyhow!(
            "Edit stats buffer too small: got {}, need {}",
            raw.len(),
            expected_len
        ));
    }

    let mut values = [0u32; EDIT_STATS_VOXEL_TYPE_COUNT * 2];
    for (idx, chunk) in raw
        .chunks_exact(std::mem::size_of::<u32>())
        .take(EDIT_STATS_VOXEL_TYPE_COUNT * 2)
        .enumerate()
    {
        values[idx] = u32::from_ne_bytes(chunk.try_into().unwrap());
    }

    let mut removed_counts = [0u32; EDIT_STATS_VOXEL_TYPE_COUNT];
    removed_counts.copy_from_slice(&values[..EDIT_STATS_VOXEL_TYPE_COUNT]);
    let mut added_counts = [0u32; EDIT_STATS_VOXEL_TYPE_COUNT];
    added_counts.copy_from_slice(&values[EDIT_STATS_VOXEL_TYPE_COUNT..]);

    Ok(ChunkModifyStats {
        removed_counts,
        added_counts,
    })
}

fn update_round_cones(resources: &PlainBuilderResources, round_cones: &[RoundCone]) -> Result<()> {
    for (i, round_cone) in round_cones.iter().enumerate() {
        let data = StructMemberDataBuilder::from_buffer(&resources.round_cones)
            .set_field(
                "data.center_a",
                PlainMemberTypeWithData::Vec3(round_cone.center_a().to_array()),
            )
            .set_field(
                "data.center_b",
                PlainMemberTypeWithData::Vec3(round_cone.center_b().to_array()),
            )
            .set_field(
                "data.radius_a",
                PlainMemberTypeWithData::Float(round_cone.radius_a()),
            )
            .set_field(
                "data.radius_b",
                PlainMemberTypeWithData::Float(round_cone.radius_b()),
            )
            .build()?;
        resources
            .round_cones
            .fill_element_with_raw_u8(&data, i as u64)?;
    }
    Ok(())
}

fn update_cuboids(resources: &PlainBuilderResources, cuboids: &[Cuboid]) -> Result<()> {
    for (i, cuboid) in cuboids.iter().enumerate() {
        let data = StructMemberDataBuilder::from_buffer(&resources.cuboids)
            .set_field(
                "data.min_corner",
                PlainMemberTypeWithData::Vec3(cuboid.min().to_array()),
            )
            .set_field("data._pad0", PlainMemberTypeWithData::Float(0.0))
            .set_field(
                "data.max_corner",
                PlainMemberTypeWithData::Vec3(cuboid.max().to_array()),
            )
            .set_field("data._pad1", PlainMemberTypeWithData::Float(0.0))
            .build()?;
        resources
            .cuboids
            .fill_element_with_raw_u8(&data, i as u64)?;
    }
    Ok(())
}

fn update_spheres(resources: &PlainBuilderResources, spheres: &[Sphere]) -> Result<()> {
    for (i, sphere) in spheres.iter().enumerate() {
        let data = StructMemberDataBuilder::from_buffer(&resources.spheres)
            .set_field(
                "data.center",
                PlainMemberTypeWithData::Vec3(sphere.center().to_array()),
            )
            .set_field(
                "data.radius",
                PlainMemberTypeWithData::Float(sphere.radius()),
            )
            .build()?;
        resources
            .spheres
            .fill_element_with_raw_u8(&data, i as u64)?;
    }
    Ok(())
}

fn update_trunk_bvh_nodes(resources: &PlainBuilderResources, bvh_nodes: &[BvhNode]) -> Result<()> {
    for (i, bvh_node) in bvh_nodes.iter().enumerate() {
        let combined_offset: u32 = if bvh_node.is_leaf {
            let primitive_idx = bvh_node.data_offset;
            0x8000_0000 | primitive_idx
        } else {
            bvh_node.left
        };
        let data = StructMemberDataBuilder::from_buffer(&resources.trunk_bvh_nodes)
            .set_field(
                "data.aabb_min",
                PlainMemberTypeWithData::Vec3(bvh_node.aabb.min().to_array()),
            )
            .set_field(
                "data.aabb_max",
                PlainMemberTypeWithData::Vec3(bvh_node.aabb.max().to_array()),
            )
            .set_field(
                "data.offset",
                PlainMemberTypeWithData::UInt(combined_offset),
            )
            .build()?;
        resources
            .trunk_bvh_nodes
            .fill_element_with_raw_u8(&data, i as u64)?;
    }
    Ok(())
}
