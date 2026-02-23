mod resources;
use super::PlainBuilderResources;
use crate::{
    flora::species,
    geom::UAabb3,
    util::ShaderCompiler,
    vkn::{
        Allocator, Buffer, ClearValue, ColorClearValue, CommandBuffer, ComputePipeline,
        DescriptorPool, Extent3D, MemoryBarrier, PipelineBarrier, PlainMemberTypeWithData,
        ShaderModule, StructMemberDataBuilder, VulkanContext, WriteDescriptorSet,
    },
};
use anyhow::Result;
use ash::vk;
use glam::{UVec3, Vec3};
pub use resources::*;

pub struct FloraRegenStats {
    pub candidate_total: u32,
    pub appended_total: u32,
    pub before_total: u32,
    pub after_total: u32,
    pub dispatch_dim: UVec3,
}
use std::collections::HashSet;

pub struct SurfaceBuilder {
    vulkan_ctx: VulkanContext,
    pub resources: SurfaceResources,

    #[allow(dead_code)]
    pool: DescriptorPool,

    make_surface_ppl: ComputePipeline,
    place_flora_ppl: ComputePipeline,
    edit_flora_ppl: ComputePipeline,

    chunk_bound: UAabb3,
    voxel_dim_per_chunk: UVec3,
    flora_species_count: usize,
}

impl SurfaceBuilder {
    pub fn new(
        vulkan_ctx: VulkanContext,
        allocator: Allocator,
        shader_compiler: &ShaderCompiler,
        plain_builder_resources: &PlainBuilderResources,
        voxel_dim_per_chunk: UVec3,
        chunk_bound: UAabb3,
    ) -> Self {
        let device = vulkan_ctx.device();
        species::assert_species_limit();
        let flora_species_count = species::species_count();

        let make_surface_sm = ShaderModule::from_glsl(
            device,
            shader_compiler,
            "shader/builder/surface/make_surface.comp",
            "main",
        )
        .unwrap();

        let place_flora_sm = ShaderModule::from_glsl(
            device,
            shader_compiler,
            "shader/builder/surface/place_flora.comp",
            "main",
        )
        .unwrap();

        let edit_flora_sm = ShaderModule::from_glsl(
            device,
            shader_compiler,
            "shader/builder/surface/edit_flora_instances.comp",
            "main",
        )
        .unwrap();

        let resources = SurfaceResources::new(
            device.clone(),
            allocator,
            voxel_dim_per_chunk,
            &make_surface_sm,
            &place_flora_sm,
            &edit_flora_sm,
            chunk_bound,
        );

        let pool = DescriptorPool::new(device).unwrap();

        let make_surface_ppl = ComputePipeline::new(
            device,
            &make_surface_sm,
            &pool,
            &[&resources, plain_builder_resources],
        );

        let place_flora_ppl = ComputePipeline::new(
            device,
            &place_flora_sm,
            &pool,
            &[&resources, plain_builder_resources],
        );

        let edit_flora_ppl = ComputePipeline::new(
            device,
            &edit_flora_sm,
            &pool,
            &[&resources, plain_builder_resources],
        );

        Self {
            vulkan_ctx,
            resources,
            pool,
            make_surface_ppl,
            place_flora_ppl,
            edit_flora_ppl,
            chunk_bound,
            voxel_dim_per_chunk,
            flora_species_count,
        }
    }

    fn update_flora_instance_set(&self, chunk_id: UVec3) {
        let chunk_resources = &self
            .resources
            .instances
            .chunk_flora_instances
            .iter()
            .find(|(_, resources)| resources.chunk_id == chunk_id)
            .unwrap()
            .1;

        self.update_flora_instance_set_with_resources(&chunk_resources.resources);
    }

    fn update_flora_instance_set_with_resources(&self, resources: &[InstanceResource]) {
        for (species_index, instance_resource) in resources.iter().enumerate() {
            self.place_flora_ppl.write_descriptor_set(
                1,
                WriteDescriptorSet::new_buffer_write(0, &instance_resource.instances_buf)
                    .with_array_element(species_index as u32),
            );
        }
    }

    /// Returns active_voxel_len
    pub fn build_surface(&mut self, chunk_id: UVec3, place_flora: bool) -> Result<u32> {
        if !self.chunk_bound.in_bound(chunk_id) {
            return Err(anyhow::anyhow!("Chunk ID out of bounds"));
        }

        let atlas_read_offset = chunk_id * self.voxel_dim_per_chunk;
        let atlas_read_dim = self.voxel_dim_per_chunk;

        let device = self.vulkan_ctx.device();

        update_make_surface_info(
            &self.resources.make_surface_info,
            atlas_read_offset,
            atlas_read_dim,
            true,
        )?;

        cleanup_make_surface_result(&self.resources.make_surface_result)?;
        if place_flora {
            update_place_flora_info(
                &self.resources.place_flora_info,
                atlas_read_offset,
                atlas_read_dim,
                UVec3::ZERO,
            )?;
            cleanup_place_flora_result(&self.resources.place_flora_result)?;
            self.update_flora_instance_set(chunk_id);
        }

        let cmdbuf = CommandBuffer::new(device, self.vulkan_ctx.command_pool());
        cmdbuf.begin(true);

        self.resources.surface.get_image().record_clear(
            &cmdbuf,
            Some(vk::ImageLayout::GENERAL),
            0,
            ClearValue::Color(ColorClearValue::UInt([0, 0, 0, 0])),
        );

        let extent = Extent3D {
            width: self.voxel_dim_per_chunk.x,
            height: self.voxel_dim_per_chunk.y,
            depth: self.voxel_dim_per_chunk.z,
        };

        self.make_surface_ppl.record(&cmdbuf, extent, None);
        if place_flora {
            // Barrier to ensure make_surface writes complete before place_flora reads
            let barrier = PipelineBarrier::new(
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vec![MemoryBarrier::new_shader_access()],
            );
            barrier.record_insert(device, &cmdbuf);

            self.place_flora_ppl.record(&cmdbuf, extent, None);
        }

        cmdbuf.end();

        cmdbuf.submit(&self.vulkan_ctx.get_general_queue(), None);

        device.wait_queue_idle(&self.vulkan_ctx.get_general_queue());

        let active_voxel_len = get_make_surface_result(&self.resources.make_surface_result);
        if place_flora {
            let flora_instance_lengths =
                get_flora_result(&self.resources.place_flora_result, self.flora_species_count);

            let chunk_resources = self
                .resources
                .instances
                .chunk_flora_instances
                .iter_mut()
                .find(|(_, resources)| resources.chunk_id == chunk_id)
                .unwrap();
            for (species_idx, instances_len) in flora_instance_lengths.iter().enumerate() {
                chunk_resources.1.get_mut(species_idx).instances_len = *instances_len;
            }
        }

        return Ok(active_voxel_len);

        fn update_make_surface_info(
            make_surface_info: &Buffer,
            atlas_read_offset: UVec3,
            atlas_read_dim: UVec3,
            is_crossing_boundary: bool,
        ) -> Result<()> {
            let data = StructMemberDataBuilder::from_buffer(make_surface_info)
                .set_field(
                    "atlas_read_offset",
                    PlainMemberTypeWithData::UVec3(atlas_read_offset.to_array()),
                )
                .set_field(
                    "atlas_read_dim",
                    PlainMemberTypeWithData::UVec3(atlas_read_dim.to_array()),
                )
                .set_field(
                    "is_crossing_boundary",
                    PlainMemberTypeWithData::UInt(if is_crossing_boundary { 1 } else { 0 }),
                )
                .build()?;
            make_surface_info.fill_with_raw_u8(&data)?;
            Ok(())
        }

        fn cleanup_make_surface_result(make_surface_result: &Buffer) -> Result<()> {
            let layout = make_surface_result.get_layout().unwrap();
            let buffer_size = layout.root_member.get_size_bytes() as usize;
            let zeroed = vec![0u8; buffer_size];
            make_surface_result.fill_with_raw_u8(&zeroed)?;
            Ok(())
        }

        fn update_place_flora_info(
            place_flora_info: &Buffer,
            atlas_read_offset: UVec3,
            atlas_read_dim: UVec3,
            atlas_local_offset: UVec3,
        ) -> Result<()> {
            let data = StructMemberDataBuilder::from_buffer(place_flora_info)
                .set_field(
                    "atlas_read_offset",
                    PlainMemberTypeWithData::UVec3(atlas_read_offset.to_array()),
                )
                .set_field(
                    "atlas_read_dim",
                    PlainMemberTypeWithData::UVec3(atlas_read_dim.to_array()),
                )
                .set_field(
                    "atlas_local_offset",
                    PlainMemberTypeWithData::UVec3(atlas_local_offset.to_array()),
                )
                .build()?;
            place_flora_info.fill_with_raw_u8(&data)?;
            Ok(())
        }

        fn cleanup_place_flora_result(place_flora_result: &Buffer) -> Result<()> {
            let layout = place_flora_result.get_layout().unwrap();
            let buffer_size = layout.root_member.get_size_bytes() as usize;
            let zeroed = vec![0u8; buffer_size];
            place_flora_result.fill_with_raw_u8(&zeroed)?;
            Ok(())
        }

        /// Returns: active_voxel_len
        fn get_make_surface_result(make_surface_result: &Buffer) -> u32 {
            let raw_data = make_surface_result.read_back().unwrap();
            let total_u32 = raw_data.len() / std::mem::size_of::<u32>();
            let data =
                unsafe { std::slice::from_raw_parts(raw_data.as_ptr() as *const u32, total_u32) };
            assert!(
                total_u32 >= 1,
                "make_surface_result buffer too small: expected at least 1 u32, got {}",
                total_u32
            );
            data[0]
        }

        /// Returns: per-species instance lengths
        fn get_flora_result(place_flora_result: &Buffer, species_count: usize) -> Vec<u32> {
            let raw_data = place_flora_result.read_back().unwrap();
            let total_u32 = raw_data.len() / std::mem::size_of::<u32>();
            let data =
                unsafe { std::slice::from_raw_parts(raw_data.as_ptr() as *const u32, total_u32) };
            assert!(
                total_u32 >= species_count,
                "place_flora_result buffer too small: expected at least {} u32s, got {}",
                species_count,
                total_u32
            );
            let mut flora_instance_lengths = Vec::with_capacity(species_count);
            flora_instance_lengths.extend_from_slice(&data[0..species_count]);
            flora_instance_lengths
        }
    }

    pub fn edit_flora_instances(
        &mut self,
        chunk_id: UVec3,
        edit_center: Vec3,
        edit_radius: f32,
        flora_tick: u32,
    ) -> Result<()> {
        if !self.chunk_bound.in_bound(chunk_id) {
            return Err(anyhow::anyhow!("Chunk ID out of bounds"));
        }

        let chunk_resources = self
            .resources
            .instances
            .chunk_flora_instances
            .iter_mut()
            .find(|(_, resources)| resources.chunk_id == chunk_id)
            .unwrap();

        let device = self.vulkan_ctx.device();
        let edit_center_vox = edit_center * 256.0;
        let edit_radius_vox = edit_radius * 256.0;

        for species_idx in 0..self.flora_species_count {
            let instance_resource = chunk_resources.1.get_mut(species_idx);
            let input_len = instance_resource.instances_len;
            if input_len == 0 {
                continue;
            }

            update_edit_flora_info(
                &self.resources.edit_flora_info,
                edit_center_vox,
                edit_radius_vox,
                input_len,
                flora_tick,
            )?;
            cleanup_edit_flora_result(&self.resources.edit_flora_result)?;

            self.edit_flora_ppl.write_descriptor_set(
                1,
                WriteDescriptorSet::new_buffer_write(0, &instance_resource.instances_buf),
            );
            self.edit_flora_ppl.write_descriptor_set(
                1,
                WriteDescriptorSet::new_buffer_write(1, &self.resources.flora_instance_scratch),
            );

            let cmdbuf = CommandBuffer::new(device, self.vulkan_ctx.command_pool());
            cmdbuf.begin(true);

            self.edit_flora_ppl.record(
                &cmdbuf,
                Extent3D {
                    width: input_len,
                    height: 1,
                    depth: 1,
                },
                None,
            );

            let compute_to_transfer = PipelineBarrier::new(
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::TRANSFER,
                vec![MemoryBarrier::new(
                    vk::AccessFlags::SHADER_WRITE,
                    vk::AccessFlags::TRANSFER_READ | vk::AccessFlags::TRANSFER_WRITE,
                )],
            );
            compute_to_transfer.record_insert(device, &cmdbuf);

            let copy_size = instance_resource.instances_buf.get_size_bytes();
            self.resources.flora_instance_scratch.record_copy_to_buffer(
                &cmdbuf,
                &instance_resource.instances_buf,
                copy_size,
                0,
                0,
            );

            cmdbuf.end();
            cmdbuf.submit(&self.vulkan_ctx.get_general_queue(), None);
            device.wait_queue_idle(&self.vulkan_ctx.get_general_queue());

            let output_len = get_edit_flora_result(&self.resources.edit_flora_result);
            instance_resource.instances_len = output_len;
        }

        return Ok(());

        fn update_edit_flora_info(
            edit_flora_info: &Buffer,
            edit_center_vox: Vec3,
            edit_radius_vox: f32,
            input_len: u32,
            flora_tick: u32,
        ) -> Result<()> {
            let data = StructMemberDataBuilder::from_buffer(edit_flora_info)
                .set_field(
                    "edit_center_radius_vox",
                    PlainMemberTypeWithData::Vec4([
                        edit_center_vox.x,
                        edit_center_vox.y,
                        edit_center_vox.z,
                        edit_radius_vox,
                    ]),
                )
                .set_field(
                    "meta",
                    PlainMemberTypeWithData::UVec4([input_len, flora_tick, 0, 0]),
                )
                .build()?;
            edit_flora_info.fill_with_raw_u8(&data)?;
            Ok(())
        }

        fn cleanup_edit_flora_result(edit_flora_result: &Buffer) -> Result<()> {
            let layout = edit_flora_result.get_layout().unwrap();
            let buffer_size = layout.root_member.get_size_bytes() as usize;
            let zeroed = vec![0u8; buffer_size];
            edit_flora_result.fill_with_raw_u8(&zeroed)?;
            Ok(())
        }

        fn get_edit_flora_result(edit_flora_result: &Buffer) -> u32 {
            let raw_data = edit_flora_result.read_back().unwrap();
            let total_u32 = raw_data.len() / std::mem::size_of::<u32>();
            let data =
                unsafe { std::slice::from_raw_parts(raw_data.as_ptr() as *const u32, total_u32) };
            assert!(
                total_u32 >= 1,
                "edit_flora_result buffer too small: expected at least 1 u32, got {}",
                total_u32
            );
            data[0]
        }
    }

    pub fn regenerate_flora_instances(
        &mut self,
        chunk_id: UVec3,
        edit_center: Vec3,
        edit_radius: f32,
        _flora_tick: u32,
    ) -> Result<FloraRegenStats> {
        if !self.chunk_bound.in_bound(chunk_id) {
            return Err(anyhow::anyhow!("Chunk ID out of bounds"));
        }

        let edit_center_vox = edit_center * 256.0;
        let edit_radius_vox = edit_radius * 256.0;
        let chunk_min = chunk_id * self.voxel_dim_per_chunk;
        let chunk_max = chunk_min + self.voxel_dim_per_chunk - UVec3::ONE;

        let requested_min = Vec3::new(
            (edit_center_vox.x - edit_radius_vox).floor(),
            (edit_center_vox.y - edit_radius_vox).floor(),
            (edit_center_vox.z - edit_radius_vox).floor(),
        );
        let requested_max = Vec3::new(
            (edit_center_vox.x + edit_radius_vox).ceil(),
            (edit_center_vox.y + edit_radius_vox).ceil(),
            (edit_center_vox.z + edit_radius_vox).ceil(),
        );

        let atlas_read_offset = UVec3::new(
            requested_min.x.max(chunk_min.x as f32).max(0.0) as u32,
            requested_min.y.max(chunk_min.y as f32).max(0.0) as u32,
            requested_min.z.max(chunk_min.z as f32).max(0.0) as u32,
        );
        let atlas_read_max = UVec3::new(
            requested_max.x.min(chunk_max.x as f32).max(0.0) as u32,
            requested_max.y.min(chunk_max.y as f32).max(0.0) as u32,
            requested_max.z.min(chunk_max.z as f32).max(0.0) as u32,
        );

        if atlas_read_offset.cmpgt(atlas_read_max).any() {
            return Ok(FloraRegenStats {
                candidate_total: 0,
                appended_total: 0,
                before_total: 0,
                after_total: 0,
                dispatch_dim: UVec3::ZERO,
            });
        }

        let atlas_read_dim = atlas_read_max - atlas_read_offset + UVec3::ONE;

        update_place_flora_info(
            &self.resources.place_flora_info,
            atlas_read_offset,
            atlas_read_dim,
            atlas_read_offset - chunk_id * self.voxel_dim_per_chunk,
        )?;
        cleanup_place_flora_result(&self.resources.place_flora_result)?;
        self.update_flora_instance_set_with_resources(&self.resources.flora_regen_candidates);

        let device = self.vulkan_ctx.device();
        let cmdbuf = CommandBuffer::new(device, self.vulkan_ctx.command_pool());
        cmdbuf.begin(true);
        self.place_flora_ppl.record(
            &cmdbuf,
            Extent3D {
                width: atlas_read_dim.x,
                height: atlas_read_dim.y,
                depth: atlas_read_dim.z,
            },
            None,
        );
        cmdbuf.end();
        cmdbuf.submit(&self.vulkan_ctx.get_general_queue(), None);
        device.wait_queue_idle(&self.vulkan_ctx.get_general_queue());

        let candidate_lengths =
            get_place_flora_result(&self.resources.place_flora_result, self.flora_species_count);
        let candidate_total = candidate_lengths.iter().copied().sum();

        let chunk_resources = self
            .resources
            .instances
            .chunk_flora_instances
            .iter_mut()
            .find(|(_, resources)| resources.chunk_id == chunk_id)
            .unwrap();

        let edit_radius_vox_sq = edit_radius_vox * edit_radius_vox;

        let mut occupied_positions = HashSet::<u64>::new();
        let mut species_instances = Vec::with_capacity(self.flora_species_count);
        let mut before_total = 0_u32;

        for species_idx in 0..self.flora_species_count {
            let instance_resource = chunk_resources.1.get(species_idx);
            let instances = read_instances(
                &instance_resource.instances_buf,
                instance_resource.instances_len,
            );
            before_total = before_total.saturating_add(instance_resource.instances_len);
            for instance in &instances {
                occupied_positions.insert(pack_position_key(
                    instance.pos_x,
                    instance.pos_y,
                    instance.pos_z,
                ));
            }
            species_instances.push(instances);
        }

        let mut appended_total = 0_u32;
        for species_idx in 0..self.flora_species_count {
            let candidate_len = candidate_lengths[species_idx];
            if candidate_len == 0 {
                continue;
            }
            let candidate_instances = read_instances(
                &self.resources.flora_regen_candidates[species_idx].instances_buf,
                candidate_len,
            );
            if candidate_instances.is_empty() {
                continue;
            }

            let current_instances = &mut species_instances[species_idx];
            let species_capacity = (chunk_resources
                .1
                .get(species_idx)
                .instances_buf
                .get_size_bytes()
                / std::mem::size_of::<Instance>() as u64)
                as usize;

            for mut candidate in candidate_instances {
                let stem_pos = Vec3::new(
                    candidate.pos_x as f32,
                    candidate.pos_y as f32,
                    candidate.pos_z as f32,
                );
                let base_center = stem_pos + Vec3::new(0.5, -0.5, 0.5);
                let delta = base_center - edit_center_vox;
                let in_edit = delta.length_squared() <= edit_radius_vox_sq;
                if !in_edit {
                    continue;
                }

                let key = pack_position_key(candidate.pos_x, candidate.pos_y, candidate.pos_z);
                if occupied_positions.contains(&key) {
                    continue;
                }
                if current_instances.len() >= species_capacity {
                    break;
                }

                candidate.growth_start_tick = 0;
                occupied_positions.insert(key);
                current_instances.push(candidate);
                appended_total = appended_total.saturating_add(1);
            }
        }

        let mut after_total = 0_u32;
        for species_idx in 0..self.flora_species_count {
            let instance_resource = chunk_resources.1.get_mut(species_idx);
            instance_resource
                .instances_buf
                .fill(&species_instances[species_idx])?;
            instance_resource.instances_len = species_instances[species_idx].len() as u32;
            after_total = after_total.saturating_add(instance_resource.instances_len);
        }

        return Ok(FloraRegenStats {
            candidate_total,
            appended_total,
            before_total,
            after_total,
            dispatch_dim: atlas_read_dim,
        });

        fn update_place_flora_info(
            place_flora_info: &Buffer,
            atlas_read_offset: UVec3,
            atlas_read_dim: UVec3,
            atlas_local_offset: UVec3,
        ) -> Result<()> {
            let data = StructMemberDataBuilder::from_buffer(place_flora_info)
                .set_field(
                    "atlas_read_offset",
                    PlainMemberTypeWithData::UVec3(atlas_read_offset.to_array()),
                )
                .set_field(
                    "atlas_read_dim",
                    PlainMemberTypeWithData::UVec3(atlas_read_dim.to_array()),
                )
                .set_field(
                    "atlas_local_offset",
                    PlainMemberTypeWithData::UVec3(atlas_local_offset.to_array()),
                )
                .build()?;
            place_flora_info.fill_with_raw_u8(&data)?;
            Ok(())
        }

        fn cleanup_place_flora_result(place_flora_result: &Buffer) -> Result<()> {
            let layout = place_flora_result.get_layout().unwrap();
            let buffer_size = layout.root_member.get_size_bytes() as usize;
            let zeroed = vec![0u8; buffer_size];
            place_flora_result.fill_with_raw_u8(&zeroed)?;
            Ok(())
        }

        fn get_place_flora_result(place_flora_result: &Buffer, species_count: usize) -> Vec<u32> {
            let raw_data = place_flora_result.read_back().unwrap();
            let total_u32 = raw_data.len() / std::mem::size_of::<u32>();
            let data =
                unsafe { std::slice::from_raw_parts(raw_data.as_ptr() as *const u32, total_u32) };
            assert!(
                total_u32 >= species_count,
                "place_flora_result buffer too small: expected at least {} u32s, got {}",
                species_count,
                total_u32
            );
            let mut flora_instance_lengths = Vec::with_capacity(species_count);
            flora_instance_lengths.extend_from_slice(&data[0..species_count]);
            flora_instance_lengths
        }

        fn read_instances(buffer: &Buffer, len: u32) -> Vec<Instance> {
            if len == 0 {
                return Vec::new();
            }

            let raw_data = buffer.read_back().unwrap();
            let instance_size = std::mem::size_of::<Instance>();
            let max_count = (raw_data.len() / instance_size).min(len as usize);
            let mut out = Vec::with_capacity(max_count);

            for chunk in raw_data.chunks_exact(instance_size).take(max_count) {
                let pos_x = u32::from_ne_bytes(chunk[0..4].try_into().unwrap());
                let pos_y = u32::from_ne_bytes(chunk[4..8].try_into().unwrap());
                let pos_z = u32::from_ne_bytes(chunk[8..12].try_into().unwrap());
                let ty_seed = u32::from_ne_bytes(chunk[12..16].try_into().unwrap());
                let growth_start_tick = u32::from_ne_bytes(chunk[16..20].try_into().unwrap());
                out.push(Instance {
                    pos_x,
                    pos_y,
                    pos_z,
                    ty_seed,
                    growth_start_tick,
                });
            }

            out
        }

        fn pack_position_key(x: u32, y: u32, z: u32) -> u64 {
            ((x as u64) << 42) | ((y as u64) << 21) | (z as u64)
        }
    }

    pub fn get_resources(&self) -> &SurfaceResources {
        &self.resources
    }
}
