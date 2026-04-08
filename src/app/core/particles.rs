use super::App;
use crate::geom::UAabb3;
use crate::particles::{
    ButterflyEmitter, ButterflyEmitterDesc, FallenLeafEmitter, ParticleEmitter, ParticleHandle,
    ParticleSystem, ParticleTickStep,
};
use crate::util::ClusterResult;
use egui::Color32;
use glam::{Vec2, Vec3, Vec4};
use std::f32::consts::TAU;

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

    #[allow(dead_code)]
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
        let (lifetime_min, lifetime_max) = {
            let min = gui_adjustables.butterfly_lifetime_min.value;
            let max = gui_adjustables.butterfly_lifetime_max.value;
            (min.min(max), min.max(max))
        };

        ButterflyEmitterDesc {
            enabled: gui_adjustables.butterflies_enabled.value,
            butterfly_count: Self::butterfly_count_from_per_chunk(
                gui_adjustables.butterflies_per_chunk.value,
            ),
            height_offset_min,
            height_offset_max,
            size: gui_adjustables.butterfly_size.value,
            lifetime_min,
            lifetime_max,
            color_low: Vec4::ONE,
            color_high: Vec4::ONE,
            worm_noise_frequency: gui_adjustables.butterfly_worm_noise_frequency.value,
            worm_noise_detail_frequency: gui_adjustables
                .butterfly_worm_noise_detail_frequency
                .value,
            worm_noise_detail_weight: gui_adjustables.butterfly_worm_noise_detail_weight.value,
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

    /// CPU-only particle work: emitter updates, physics simulation, tick stepping.
    /// No GPU commands are submitted — safe to call while the GPU is still rendering.
    pub(super) fn update_particle_cpu(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }

        self.butterfly_emitter_desc =
            Self::butterfly_desc_from_gui_adjustables(&self.gui_adjustables);
        for emitter in &mut self.butterfly_emitters {
            emitter.apply_desc(&self.butterfly_emitter_desc);
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
    }

    /// GPU terrain queries + particle upload. Must be called AFTER all prior render
    /// fences have been waited on (previous frame's commands must be complete).
    pub(super) fn update_particle_gpu(&mut self) {
        let tick_step = self.particle_system.last_tick_step();
        if tick_step.did_step {
            self.particle_animation_time_sec += tick_step.step_seconds;
            self.plan_butterflies(tick_step);
        }
        self.particle_system
            .write_snapshots(&mut self.particle_snapshots);

        if let Err(err) = self.tracer.upload_particles(&self.particle_snapshots) {
            log::error!("Failed to upload particles: {}", err);
        }
    }

    pub(super) fn plan_butterflies(&mut self, tick_step: ParticleTickStep) {
        use crate::tracer::TerrainRayQuery;

        const MAX_PLACEMENT_FRAMES: u8 = 3;
        const STEP_LEN: f32 = crate::particles::emitters::WORM_STEP_LEN;
        const RAY_EPSILON: f32 = 0.02;

        let map_size = super::CHUNK_DIM.as_vec3();

        // ── Phase 1: Batched placement ──────────────────────────────────
        // Collect all butterflies needing placement: new spawns + deferred retries.
        // Generate one candidate XZ per butterfly, batch into a single GPU query.

        struct PlacementEntry {
            handle: ParticleHandle,
            emitter_idx: usize,
            candidate: Vec3,
            attempts_remaining: u8,
        }

        let mut placement_batch: Vec<PlacementEntry> = Vec::new();

        // Drain deferred retries from last frame (generate new candidate positions)
        let prev_retries = std::mem::take(&mut self.pending_placement_retries);
        for (handle, emitter_idx, attempts_remaining) in prev_retries {
            if !self.particle_system.is_alive_handle(handle) {
                continue;
            }
            if let Some(em) = self.butterfly_emitters.get_mut(emitter_idx) {
                let candidate: Vec3 = em.random_spawn_position_candidate();
                placement_batch.push(PlacementEntry {
                    handle,
                    emitter_idx,
                    candidate,
                    attempts_remaining,
                });
            }
        }

        // Drain new spawns from emitters
        for emitter_idx in 0..self.butterfly_emitters.len() {
            let pending_handles =
                self.butterfly_emitters[emitter_idx].drain_pending_placement_handles();
            for handle in pending_handles {
                if !self.particle_system.is_alive_handle(handle) {
                    continue;
                }
                // Use the spawn position as initial candidate, or generate a new one
                let candidate = self.particle_system.position(handle).unwrap_or_else(|| {
                    self.butterfly_emitters[emitter_idx].random_spawn_position_candidate()
                });
                placement_batch.push(PlacementEntry {
                    handle,
                    emitter_idx,
                    candidate,
                    attempts_remaining: MAX_PLACEMENT_FRAMES,
                });
            }
        }

        // Single GPU dispatch for all placement height queries
        if !placement_batch.is_empty() {
            let xz_positions: Vec<Vec2> = placement_batch
                .iter()
                .map(|e| Vec2::new(e.candidate.x, e.candidate.z))
                .collect();

            let samples = match self
                .tracer
                .query_terrain_heights_batch_with_validity(&xz_positions)
            {
                Ok(s) => s,
                Err(err) => {
                    log::error!("Failed batched terrain height query for placement: {}", err);
                    // Re-queue everything for next frame rather than losing them
                    for entry in placement_batch {
                        if entry.attempts_remaining > 1 {
                            self.pending_placement_retries.push((
                                entry.handle,
                                entry.emitter_idx,
                                entry.attempts_remaining - 1,
                            ));
                        }
                    }
                    return;
                }
            };

            for (entry, sample) in placement_batch.into_iter().zip(samples.into_iter()) {
                if sample.is_valid {
                    let _ = self
                        .particle_system
                        .set_position(entry.handle, entry.candidate);
                } else if entry.attempts_remaining > 1 {
                    // Defer to next frame with decremented counter
                    self.pending_placement_retries.push((
                        entry.handle,
                        entry.emitter_idx,
                        entry.attempts_remaining - 1,
                    ));
                } else {
                    // Out of retries — use fallback position (emitter center Y)
                    let fallback_y = self
                        .butterfly_emitters
                        .get(entry.emitter_idx)
                        .map(|em| em.center.y)
                        .unwrap_or(0.5);
                    let fallback = Vec3::new(entry.candidate.x, fallback_y, entry.candidate.z);
                    let _ = self.particle_system.set_position(entry.handle, fallback);
                }
            }
        }

        // ── Phase 2: Collect active butterflies for movement ────────────

        let mut all_handles: Vec<ParticleHandle> = Vec::new();
        let mut all_positions: Vec<Vec3> = Vec::new();
        let mut all_directions: Vec<Vec3> = Vec::new();
        let mut all_emitter_indices: Vec<usize> = Vec::new();

        for emitter_idx in 0..self.butterfly_emitters.len() {
            let (mut handles, mut positions, mut directions) = {
                let emitter = &mut self.butterfly_emitters[emitter_idx];
                let mut handles = Vec::new();
                let mut positions = Vec::new();
                let mut directions = Vec::new();
                emitter.collect_butterfly_states(
                    &self.particle_system,
                    &mut handles,
                    &mut positions,
                    &mut directions,
                );
                (handles, positions, directions)
            };

            if tick_step.bucket_count > 1 {
                let active_bucket = tick_step.active_bucket;
                let mut filtered_handles = Vec::with_capacity(handles.len());
                let mut filtered_positions = Vec::with_capacity(positions.len());
                let mut filtered_directions = Vec::with_capacity(directions.len());

                for ((handle, position), direction) in handles
                    .into_iter()
                    .zip(positions.into_iter())
                    .zip(directions.into_iter())
                {
                    if self.particle_system.handle_bucket(handle) == Some(active_bucket) {
                        filtered_handles.push(handle);
                        filtered_positions.push(position);
                        filtered_directions.push(direction);
                    }
                }

                handles = filtered_handles;
                positions = filtered_positions;
                directions = filtered_directions;
            }

            all_emitter_indices.resize(all_emitter_indices.len() + handles.len(), emitter_idx);
            all_handles.extend(handles);
            all_positions.extend(positions);
            all_directions.extend(directions);
        }

        if all_handles.is_empty() {
            return;
        }

        // ── Phase 3: Batched movement ray queries (single GPU dispatch) ─

        let n = all_handles.len();
        let mut successes = vec![false; n];
        let mut committed_dirs = all_directions.clone();

        // Build handle→index map for resolving deferred retries from last frame
        let handle_to_idx: std::collections::HashMap<ParticleHandle, usize> = all_handles
            .iter()
            .enumerate()
            .map(|(i, h)| (*h, i))
            .collect();

        struct MoveBatchEntry {
            particle_idx: usize,
            origin: Vec3,
            direction: Vec3,
        }

        let mut batch: Vec<MoveBatchEntry> = Vec::with_capacity(n);
        let mut retry_covered: std::collections::HashSet<usize> = std::collections::HashSet::new();

        // Include deferred movement retries from last frame (resolve handle→current index)
        let prev_move_retries = std::mem::take(&mut self.pending_movement_retries);
        for (handle, _emitter_idx, origin, direction) in prev_move_retries {
            if let Some(&idx) = handle_to_idx.get(&handle) {
                batch.push(MoveBatchEntry {
                    particle_idx: idx,
                    origin,
                    direction,
                });
                retry_covered.insert(idx);
            }
            // If handle not found, particle was despawned — silently drop the retry
        }

        // Add initial movement queries for all particles not already covered by retries
        for i in 0..n {
            if !retry_covered.contains(&i) {
                batch.push(MoveBatchEntry {
                    particle_idx: i,
                    origin: all_positions[i],
                    direction: all_directions[i],
                });
            }
        }

        if !batch.is_empty() {
            let rays: Vec<TerrainRayQuery> = batch
                .iter()
                .map(|e| TerrainRayQuery {
                    origin: e.origin + Vec3::new(0.0, RAY_EPSILON, 0.0),
                    direction: e.direction.normalize_or_zero(),
                })
                .collect();

            let hits = match self.tracer.query_terrain_rays_batch_with_validity(&rays) {
                Ok(h) => h,
                Err(err) => {
                    log::error!("Failed terrain ray query for butterflies: {}", err);
                    return;
                }
            };

            for (entry, hit) in batch.into_iter().zip(hits.into_iter()) {
                let idx = entry.particle_idx;
                if successes[idx] {
                    continue;
                }

                let next_pos = entry.origin + entry.direction * STEP_LEN;

                let out_of_bounds = next_pos.x < 0.0
                    || next_pos.x > map_size.x
                    || next_pos.z < 0.0
                    || next_pos.z > map_size.z;

                let blocked = if !out_of_bounds && hit.is_valid {
                    let hit_dist = (hit.position - entry.origin).length();
                    hit_dist < STEP_LEN - RAY_EPSILON
                } else {
                    false
                };

                if out_of_bounds || blocked {
                    // Generate a new direction and defer to next frame
                    let emitter_idx = all_emitter_indices[idx];
                    let new_dir = if let Some(em) = self.butterfly_emitters.get_mut(emitter_idx) {
                        let new_seed =
                            (entry.direction.x * 1000.0 + entry.direction.z * 100.0 + idx as f32)
                                + 17.3;
                        let new_phase = entry.direction.y * TAU + idx as f32 + 3.7;
                        crate::particles::emitters::generate_worm_direction(
                            &em.worm_noise,
                            &em.worm_noise_detail,
                            em.worm_noise_detail_weight,
                            new_seed,
                            new_phase,
                        )
                    } else {
                        entry.direction
                    };
                    self.pending_movement_retries.push((
                        all_handles[idx],
                        emitter_idx,
                        entry.origin,
                        new_dir,
                    ));
                } else {
                    successes[idx] = true;
                    committed_dirs[idx] = entry.direction;
                }
            }
        }

        // Despawn particles that failed and have no pending retry
        for i in 0..n {
            if !successes[i] {
                let has_pending_retry = self
                    .pending_movement_retries
                    .iter()
                    .any(|(h, _, _, _)| *h == all_handles[i]);
                if !has_pending_retry {
                    let emitter_idx = all_emitter_indices[i];
                    if let Some(em) = self.butterfly_emitters.get_mut(emitter_idx) {
                        em.despawn_butterfly(all_handles[i]);
                    }
                    let _ = self.particle_system.despawn(all_handles[i]);
                }
            }
        }

        for i in 0..n {
            if !successes[i] {
                continue;
            }
            let emitter_idx = all_emitter_indices[i];
            if let Some(em) = self.butterfly_emitters.get_mut(emitter_idx) {
                em.set_butterfly_state(all_handles[i], all_positions[i], committed_dirs[i]);
            }
            let _ = self
                .particle_system
                .set_velocity(all_handles[i], committed_dirs[i] * STEP_LEN);
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
