mod resources;
use anyhow::Result;
use ash::vk;
use glam::UVec3;
pub use resources::*;

use crate::{
    generated::gpu_structs::SceneTexUpdateInfo,
    geom::UAabb3,
    util::ShaderCompiler,
    vkn::{
        execute_one_time_command, Allocator, Buffer, ClearValue, ColorClearValue, CommandBuffer,
        ComputePipeline, DescriptorPool, Extent3D, ShaderModule, VulkanContext,
    },
};
use bytemuck::Zeroable;

pub struct SceneAccelBuilder {
    pub vulkan_ctx: VulkanContext,
    pub resources: SceneAccelBuilderResources,

    #[allow(dead_code)]
    pool: DescriptorPool,

    #[allow(dead_code)]
    update_scene_tex_ppl: ComputePipeline,
    update_scene_tex_cmdbuf: CommandBuffer,
}

impl SceneAccelBuilder {
    pub fn new(
        vulkan_ctx: VulkanContext,
        allocator: Allocator,
        shader_compiler: &ShaderCompiler,
        chunk_bound: UAabb3,
    ) -> Result<Self> {
        let device = vulkan_ctx.device();
        let pool = DescriptorPool::new(device).unwrap();

        let update_scene_tex_sm = ShaderModule::from_glsl(
            device,
            shader_compiler,
            "shader/builder/scene_accel/update_scene_tex.comp",
            "main",
        )
        .unwrap();

        let resources = SceneAccelBuilderResources::new(
            device.clone(),
            allocator,
            chunk_bound,
            &update_scene_tex_sm,
        );

        let update_scene_tex_ppl =
            ComputePipeline::new(device, &update_scene_tex_sm, &pool, &[&resources]);

        let update_scene_tex_cmdbuf =
            Self::record_update_scene_tex_cmdbuf(vulkan_ctx.clone(), &update_scene_tex_ppl);

        Self::clear_tex(&vulkan_ctx, &resources);

        Ok(Self {
            vulkan_ctx,
            resources,
            pool,
            update_scene_tex_ppl,
            update_scene_tex_cmdbuf,
        })
    }

    fn record_update_scene_tex_cmdbuf(
        vulkan_ctx: VulkanContext,
        update_scene_tex_ppl: &ComputePipeline,
    ) -> CommandBuffer {
        let device = vulkan_ctx.device();
        let cmdbuf = CommandBuffer::new(device, vulkan_ctx.command_pool());
        cmdbuf.begin(false);

        let extent = Extent3D {
            width: 1,
            height: 1,
            depth: 1,
        };
        update_scene_tex_ppl.record(&cmdbuf, extent, None);

        cmdbuf.end();
        cmdbuf
    }

    /// Clears the scene offset texture to zero.
    ///
    /// Also can be used at init time since it can transfer the image layout to general.
    fn clear_tex(vulkan_context: &VulkanContext, resources: &SceneAccelBuilderResources) {
        execute_one_time_command(
            vulkan_context.device(),
            vulkan_context.command_pool(),
            &vulkan_context.get_general_queue(),
            |cmdbuf| {
                resources.scene_tex.get_image().record_clear(
                    cmdbuf,
                    Some(vk::ImageLayout::GENERAL),
                    0,
                    ClearValue::Color(ColorClearValue::UInt([0, 0, 0, 0])),
                );
            },
        );
    }

    pub fn update_scene_tex(
        &mut self,
        chunk_idx: UVec3,
        node_offset_for_chunk: u64,
        node_count_for_chunk: u64,
    ) -> Result<()> {
        log::info!(
            "[SCENE_TEX] chunk_idx={:?} node_offset={} leaf_offset={}",
            chunk_idx,
            node_offset_for_chunk,
            node_count_for_chunk,
        );
        update_buffers(
            &self.resources.scene_tex_update_info,
            chunk_idx,
            node_offset_for_chunk as u32,
            node_count_for_chunk as u32,
        )?;

        // Submit without waiting — no downstream CPU dependency. Same-queue
        // ordering guarantees this completes before the next frame's rendering.
        self.update_scene_tex_cmdbuf
            .submit(&self.vulkan_ctx.get_general_queue(), None);
        return Ok(());

        fn update_buffers(
            scene_tex_update_info: &Buffer,
            chunk_idx: UVec3,
            node_offset_for_chunk: u32,
            leaf_offset_for_chunk: u32,
        ) -> Result<()> {
            scene_tex_update_info.fill_uniform(&SceneTexUpdateInfo {
                chunk_idx: chunk_idx.to_array(),
                node_offset_for_chunk,
                leaf_offset_for_chunk,
                ..SceneTexUpdateInfo::zeroed()
            })
        }
    }

    pub fn get_resources(&self) -> &SceneAccelBuilderResources {
        &self.resources
    }

    /// DEBUG: Read back the entire scene_tex and log non-trivial entries.
    #[allow(dead_code)]
    pub fn debug_read_scene_tex(&self) {
        let image = self.resources.scene_tex.get_image();
        match image.fetch_data(
            &self.vulkan_ctx.get_general_queue(),
            self.vulkan_ctx.command_pool(),
        ) {
            Ok(data) => {
                let desc = image.get_desc();
                let w = desc.extent.width as usize;
                let h = desc.extent.height as usize;
                let d = desc.extent.depth as usize;
                // R32G32_UINT = 8 bytes per texel
                let bytes_per_texel = 8usize;
                for z in 0..d {
                    for y in 0..h {
                        for x in 0..w {
                            let idx = (z * h * w + y * w + x) * bytes_per_texel;
                            if idx + 8 > data.len() {
                                continue;
                            }
                            let r = u32::from_ne_bytes(data[idx..idx + 4].try_into().unwrap());
                            let g = u32::from_ne_bytes(data[idx + 4..idx + 8].try_into().unwrap());
                            // scene_tex stores (node_offset+1, leaf_offset+1), 0 means empty
                            if r != 0 || g != 0 {
                                log::info!(
                                    "[SCENE_TEX_READBACK] chunk=({},{},{}) raw=({},{}) decoded_node={} decoded_leaf={}",
                                    x, y, z, r, g,
                                    r.wrapping_sub(1), g.wrapping_sub(1),
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("[SCENE_TEX_READBACK] failed: {}", e);
            }
        }
    }
}
