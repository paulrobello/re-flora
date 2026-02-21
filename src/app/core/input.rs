use super::ui_style::SHOVEL_SLOT_INDEX;
use super::App;
use crate::app::world_edits::TerrainRemovalEdit;
use crate::tracer::TerrainRayQuery;
use glam::{Vec2, Vec3};
use std::time::Instant;
use winit::event::DeviceEvent;
use winit::event_loop::ActiveEventLoop;

impl App {
    pub(super) fn sync_cursor_with_panels(&mut self) {
        let any_panel_open = self.config_panel_visible || self.settings_panel_visible;
        self.window_state.set_cursor_visibility(any_panel_open);
        self.window_state.set_cursor_grab(!any_panel_open);
        if any_panel_open {
            self.shovel_dig_held = false;
        }
    }

    pub(super) fn is_shovel_selected(&self) -> bool {
        self.selected_item_panel_slot == SHOVEL_SLOT_INDEX
    }

    fn query_camera_ray_terrain_intersection(
        &mut self,
        max_distance: f32,
    ) -> anyhow::Result<Option<Vec3>> {
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

    pub(super) fn try_shovel_dig(&mut self, now: Instant) {
        if self.window_state.is_cursor_visible() || !self.is_shovel_selected() {
            return;
        }

        if let Some(last_dig) = self.last_shovel_dig_time {
            if now.duration_since(last_dig) < super::SHOVEL_DIG_INTERVAL {
                return;
            }
        }

        match self.query_camera_ray_terrain_intersection(super::SHOVEL_RAY_QUERY_DISTANCE) {
            Ok(Some(center)) => {
                if let Err(err) = self.apply_surface_terrain_removal(TerrainRemovalEdit {
                    center,
                    radius: super::SHOVEL_REMOVE_RADIUS,
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
}
