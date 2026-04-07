use super::App;
use crate::app::world_edits::{BuildEdit, ClearVoxelRegionEdit, VoxelEdit, WorldEditPlan};
use crate::app::world_ops;
use crate::builder::{ContreeBuilder, PlainBuilder, SceneAccelBuilder, SurfaceBuilder};
use crate::geom::UAabb3;
use crate::util::BENCH;
use crate::vkn::{VulkanContext, VulkanContextDesc};
use crate::window::{WindowMode, WindowState, WindowStateDesc};
use anyhow::Result;
use glam::UVec3;
use winit::event_loop::ActiveEventLoop;

impl App {
    #[allow(dead_code)]
    pub(super) fn init(
        plain_builder: &mut PlainBuilder,
        surface_builder: &mut SurfaceBuilder,
        contree_builder: &mut ContreeBuilder,
        scene_accel_builder: &mut SceneAccelBuilder,
    ) -> Result<()> {
        let world_dim = super::VOXEL_DIM_PER_CHUNK * super::CHUNK_DIM;
        let world_bound = UAabb3::new(UVec3::ZERO, world_dim - UVec3::ONE);
        world_ops::execute_edit_plan_on_builders(
            plain_builder,
            surface_builder,
            contree_builder,
            scene_accel_builder,
            super::VOXEL_DIM_PER_CHUNK,
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

    pub(super) fn create_window_state(event_loop: &ActiveEventLoop) -> WindowState {
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

    pub(super) fn create_vulkan_context(window_state: &WindowState) -> VulkanContext {
        VulkanContext::new(
            &window_state.window(),
            VulkanContextDesc {
                name: "Re: Flora".into(),
            },
        )
    }
}
