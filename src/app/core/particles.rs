use super::App;
use crate::geom::UAabb3;
use crate::particles::{
    ButterflyEmitter, ButterflyEmitterDesc, FallenLeafEmitter, ParticleEmitter, ParticleHandle,
    ParticleSystem,
};
use crate::util::ClusterResult;
use egui::Color32;
use glam::{Vec2, Vec3, Vec4};

// bird-specific audio and control logic has been removed

pub(super) struct TreeLeafEmitter {
    tree_id: u32,
    pub(super) emitter: FallenLeafEmitter,
}

impl TreeLeafEmitter {
    fn new(tree_id: u32, emitter: FallenLeafEmitter) -> Self {
        Self { tree_id, emitter }
    }

    fn tree_id(&self) -> u32 {
        self.tree_id
    }
}

impl ParticleEmitter for TreeLeafEmitter {
    fn update(&mut self, system: &mut ParticleSystem, dt: f32, time: f32) {
        self.emitter.update(system, dt, time);
    }
}

impl App {
    pub(super) fn butterfly_count_from_per_chunk(butterflies_per_chunk: f32) -> u32 {
        let total_chunks = super::CHUNK_DIM.x.saturating_mul(super::CHUNK_DIM.z) as f32;
        (total_chunks * butterflies_per_chunk)
            .round()
            .clamp(0.0, u32::MAX as f32) as u32
    }

    pub(super) fn color32_to_vec4(color: Color32) -> Vec4 {
        Vec4::new(
            color.r() as f32 / 255.0,
            color.g() as f32 / 255.0,
            color.b() as f32 / 255.0,
            color.a() as f32 / 255.0,
        )
    }

    pub(super) fn butterfly_desc_from_gui_adjustables(
        gui_adjustables: &crate::app::GuiAdjustables,
    ) -> ButterflyEmitterDesc {
        let (height_offset_min, height_offset_max) = {
            let min = gui_adjustables.butterfly_height_offset_min.value;
            let max = gui_adjustables.butterfly_height_offset_max.value;
            (min.min(max), min.max(max))
        };
        let (drift_strength_min, drift_strength_max) = {
            let min = gui_adjustables.butterfly_drift_strength_min.value;
            let max = gui_adjustables.butterfly_drift_strength_max.value;
            (min.min(max), min.max(max))
        };
        let (drift_frequency_min, drift_frequency_max) = {
            let min = gui_adjustables.butterfly_drift_frequency_min.value;
            let max = gui_adjustables.butterfly_drift_frequency_max.value;
            (min.min(max), min.max(max))
        };

        ButterflyEmitterDesc {
            enabled: gui_adjustables.butterflies_enabled.value,
            butterfly_count: Self::butterfly_count_from_per_chunk(
                gui_adjustables.butterflies_per_chunk.value,
            ),
            wander_radius: gui_adjustables.butterfly_wander_radius.value,
            height_offset_min,
            height_offset_max,
            size: gui_adjustables.butterfly_size.value,
            drift_strength_min,
            drift_strength_max,
            drift_frequency_min,
            drift_frequency_max,
            steering_strength: gui_adjustables.butterfly_steering_strength.value,
            bob_frequency_hz: gui_adjustables.butterfly_bob_frequency_hz.value,
            bob_strength: gui_adjustables.butterfly_bob_strength.value,
            color_low: Vec4::ONE,
            color_high: Vec4::ONE,
        }
    }

    pub(super) fn upsert_tree_leaf_emitter(
        &mut self,
        tree_id: u32,
        tree_pos: Vec3,
        bound: &UAabb3,
        clusters: &[ClusterResult],
    ) {
        self.remove_leaf_emitter(tree_id);

        if clusters.is_empty() {
            return;
        }

        let mut emitter_indices = Vec::with_capacity(clusters.len());

        for cluster in clusters {
            let (_center, _extent) = Self::compute_leaf_emitter_region(tree_pos, bound);
            let cluster_center = cluster.pos;

            let mut emitter = FallenLeafEmitter::new(
                cluster_center,
                Vec::new(),
                tree_id as u64 + cluster.pos.x as u64 + cluster.pos.y as u64 + cluster.pos.z as u64,
                &self.leaf_emitter_desc,
            );

            let cluster_size_multiplier = (cluster.items_count as f32).sqrt();
            emitter.spawn_rate = self.leaf_emitter_desc.spawn_rate * cluster_size_multiplier;

            let idx = self.leaf_emitters.len();
            self.leaf_emitters
                .push(TreeLeafEmitter::new(tree_id, emitter));
            emitter_indices.push(idx);
        }

        self.tree_leaf_emitter_indices
            .insert(tree_id, emitter_indices);
    }

    pub(super) fn compute_leaf_emitter_region(tree_pos: Vec3, bound: &UAabb3) -> (Vec3, Vec3) {
        if bound.min() == bound.max() {
            return (
                tree_pos + Vec3::new(0.5, 1.3, 0.5),
                Vec3::new(2.0, 0.5, 2.0),
            );
        }

        let min = bound.min().as_vec3() / 256.0;
        let max = bound.max().as_vec3() / 256.0;
        let size = max - min;
        let center = min + size * 0.5;
        let extent = Vec3::new(
            (size.x * 0.5).max(0.75),
            (size.y * 0.25).max(0.5),
            (size.z * 0.5).max(0.75),
        );

        (center, extent)
    }

    pub(super) fn remove_leaf_emitter(&mut self, tree_id: u32) {
        if let Some(indices) = self.tree_leaf_emitter_indices.remove(&tree_id) {
            let mut sorted_indices = indices;
            sorted_indices.sort_unstable_by(|a, b| b.cmp(a));

            for index in sorted_indices {
                self.leaf_emitters.swap_remove(index);
                if let Some(swapped) = self.leaf_emitters.get(index) {
                    if let Some(tree_indices) =
                        self.tree_leaf_emitter_indices.get_mut(&swapped.tree_id())
                    {
                        if let Some(pos) = tree_indices
                            .iter()
                            .position(|&i| i == self.leaf_emitters.len())
                        {
                            tree_indices[pos] = index;
                        }
                    }
                }
            }
        }
    }

    pub(super) fn ensure_map_butterfly_emitter(&mut self) {
        if !self.butterfly_emitters.is_empty() {
            return;
        }

        let (center, extent) = Self::map_butterfly_region();
        self.butterfly_emitters.push(ButterflyEmitter::new(
            center,
            extent,
            9_173,
            &self.butterfly_emitter_desc,
        ));
    }

    pub(super) fn map_butterfly_region() -> (Vec3, Vec3) {
        let map_size = super::CHUNK_DIM.as_vec3();
        let center = Vec3::new(map_size.x * 0.5, 0.5, map_size.z * 0.5);
        let extent = Vec3::new(
            (map_size.x * 0.5).max(1.0),
            0.6,
            (map_size.z * 0.5).max(1.0),
        );
        (center, extent)
    }

    pub(super) fn update_particle_simulation(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }

        self.ensure_map_butterfly_emitter();
        let wind_time = self.time_info.time_since_start();
        self.particle_system
            .set_full_update_seconds(self.gui_adjustables.particle_full_update_seconds.value);

        Self::drive_emitters(
            &mut self.butterfly_emitters,
            &mut self.particle_system,
            dt,
            wind_time,
        );
        Self::drive_emitters(
            &mut self.leaf_emitters,
            &mut self.particle_system,
            dt,
            wind_time,
        );

        self.particle_system.update(dt, self.particle_forces);
        let tick_step = self.particle_system.last_tick_step();
        if tick_step.did_step {
            self.particle_animation_time_sec += tick_step.step_seconds;
            let active_bucket = if tick_step.bucket_count > 1 {
                Some(tick_step.active_bucket)
            } else {
                None
            };
            self.constrain_butterflies_to_terrain(
                tick_step.step_seconds,
                self.particle_animation_time_sec,
                active_bucket,
            );
        }
        self.particle_system
            .write_snapshots(&mut self.particle_snapshots);

        if let Err(err) = self.tracer.upload_particles(&self.particle_snapshots) {
            log::error!("Failed to upload particles: {}", err);
        }
    }

    pub(super) fn constrain_butterflies_to_terrain(
        &mut self,
        dt: f32,
        time: f32,
        active_bucket: Option<u32>,
    ) {
        let mut query_positions_xz = Vec::new();
        let mut query_targets: Vec<ButterflyQueryTarget> = Vec::new();

        for (emitter_index, emitter) in self.butterfly_emitters.iter_mut().enumerate() {
            let mut emitter_positions_xz = Vec::new();
            let mut emitter_handles = Vec::new();
            emitter.collect_ground_queries(
                &self.particle_system,
                &mut emitter_positions_xz,
                &mut emitter_handles,
            );
            query_targets.extend(
                emitter_handles
                    .into_iter()
                    .zip(emitter_positions_xz.into_iter())
                    .filter(|(handle, _)| {
                        active_bucket.is_none_or(|bucket| {
                            self.particle_system.handle_bucket(*handle) == Some(bucket)
                        })
                    })
                    .map(|(handle, pos_xz)| {
                        query_positions_xz.push(pos_xz);
                        ButterflyQueryTarget {
                            emitter_index,
                            handle,
                        }
                    }),
            );
        }

        if query_targets.is_empty() {
            return;
        }

        const BORDER_PADDING_INWARD: f32 = 0.001;
        let map_size = super::CHUNK_DIM.as_vec3();
        let max_x = (map_size.x - BORDER_PADDING_INWARD).max(BORDER_PADDING_INWARD);
        let max_z = (map_size.z - BORDER_PADDING_INWARD).max(BORDER_PADDING_INWARD);
        let mut border_despawn_count = 0usize;
        let mut out_of_bounds_before_border = 0usize;
        for (idx, target) in query_targets.iter().enumerate() {
            let Some(mut pos) = self.particle_system.position(target.handle) else {
                continue;
            };

            let mut at_border = false;
            if pos.x < 0.0 || pos.x > map_size.x || pos.z < 0.0 || pos.z > map_size.z {
                out_of_bounds_before_border += 1;
            }
            if pos.x <= BORDER_PADDING_INWARD {
                pos.x = BORDER_PADDING_INWARD;
                at_border = true;
            } else if pos.x >= max_x {
                pos.x = max_x;
                at_border = true;
            }

            if pos.z <= BORDER_PADDING_INWARD {
                pos.z = BORDER_PADDING_INWARD;
                at_border = true;
            } else if pos.z >= max_z {
                pos.z = max_z;
                at_border = true;
            }

            if at_border {
                border_despawn_count += 1;
                let _ = self.particle_system.set_position(target.handle, pos);
                let _ = self.particle_system.despawn(target.handle);
                query_positions_xz[idx] = Vec2::new(pos.x, pos.z);
            }
        }

        let heights = match self
            .tracer
            .query_terrain_heights_batch_with_validity(&query_positions_xz)
        {
            Ok(heights) => heights,
            Err(err) => {
                log::error!("Failed terrain query for butterflies: {}", err);
                return;
            }
        };

        let total_sample_count = heights.len();
        let mut invalid_sample_count = 0usize;
        for (idx, (target, sample)) in query_targets
            .into_iter()
            .zip(heights.into_iter())
            .enumerate()
        {
            if !sample.is_valid {
                invalid_sample_count += 1;
                let query_pos = query_positions_xz[idx];
                log::warn!(
                    "Invalid butterfly terrain ray: origin_xz=({:.4},{:.4}) direction=({:.1},{:.1},{:.1}) emitter_index={} handle={:?}",
                    query_pos.x,
                    query_pos.y,
                    0.0f32,
                    -1.0f32,
                    0.0f32,
                    target.emitter_index,
                    target.handle
                );
                continue;
            }
            if let Some(emitter) = self.butterfly_emitters.get_mut(target.emitter_index) {
                emitter.constrain_to_ground(
                    &mut self.particle_system,
                    target.handle,
                    sample.height,
                    dt,
                    time,
                );
            }
        }

        if invalid_sample_count > 0 {
            let (min_qx, max_qx, min_qz, max_qz) = query_positions_xz.iter().fold(
                (
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                ),
                |(min_x, max_x_q, min_z, max_z_q), q| {
                    (
                        min_x.min(q.x),
                        max_x_q.max(q.x),
                        min_z.min(q.y),
                        max_z_q.max(q.y),
                    )
                },
            );
            log::warn!(
                "Invalid butterfly terrain samples: {}/{}; border_despawns={} out_of_bounds_before_border={}; query_x=[{:.4},{:.4}] query_z=[{:.4},{:.4}] bounds_x=[{:.4},{:.4}] bounds_z=[{:.4},{:.4}]",
                invalid_sample_count,
                total_sample_count,
                border_despawn_count,
                out_of_bounds_before_border,
                min_qx,
                max_qx,
                min_qz,
                max_qz,
                BORDER_PADDING_INWARD,
                max_x,
                BORDER_PADDING_INWARD,
                max_z
            );
        }
    }

    pub(super) fn drive_emitters<E: ParticleEmitter>(
        emitters: &mut [E],
        particle_system: &mut ParticleSystem,
        dt: f32,
        time: f32,
    ) {
        for emitter in emitters {
            emitter.update(particle_system, dt, time);
        }
    }
}

#[derive(Clone, Copy)]
struct ButterflyQueryTarget {
    emitter_index: usize,
    handle: ParticleHandle,
}

// bird query types have been removed
