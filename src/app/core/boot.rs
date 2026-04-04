use super::App;
use crate::app::world_ops;
use crate::builder::{ContreeBuilder, PlainBuilder, SceneAccelBuilder, SurfaceBuilder};
use crate::util::BENCH;
use crate::vkn::{VulkanContext, VulkanContextDesc};
use crate::window::{WindowMode, WindowState, WindowStateDesc};
use anyhow::Result;
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
        world_ops::init_chunk_by_chunk(
            plain_builder,
            surface_builder,
            contree_builder,
            scene_accel_builder,
            super::VOXEL_DIM_PER_CHUNK,
            world_dim,
        )?;

        BENCH.lock().unwrap().summary();
        Ok(())
    }

    pub(super) fn create_window_state(
        event_loop: &ActiveEventLoop,
        options: &crate::AppOptions,
    ) -> WindowState {
        const WINDOW_TITLE_DEBUG: &str = "Re: Flora - debug build";
        const WINDOW_TITLE_RELEASE: &str = "Re: Flora - release build";
        let using_mode = if cfg!(debug_assertions) {
            WINDOW_TITLE_DEBUG
        } else {
            WINDOW_TITLE_RELEASE
        };
        let (window_mode, cursor_locked, cursor_visible) = if options.windowed {
            // Windowed mode: show cursor, no grab (Confined mode is unreliable on macOS)
            (WindowMode::Windowed(false), false, true)
        } else {
            // Borderless fullscreen: hide cursor and lock it for FPS camera
            (WindowMode::BorderlessFullscreen, true, false)
        };
        let window_descriptor = WindowStateDesc {
            title: using_mode.to_owned(),
            window_mode,
            cursor_locked,
            cursor_visible,
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
