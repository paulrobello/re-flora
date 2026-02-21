use super::App;
use crate::audio::SpatialSoundManager;
use crate::geom::UAabb3;
use crate::particles::{
    BirdEmitter, ButterflyEmitter, ButterflyEmitterDesc, FallenLeafEmitter, ParticleEmitter,
    ParticleHandle, ParticleSystem,
};
use crate::tracer::TerrainRayQuery;
use crate::util::ClusterResult;
use egui::Color32;
use glam::{Vec2, Vec3, Vec4};
use rand::Rng;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

const BIRD_AUDIO_PATH: &str = "assets/sfx/SA_EuropeanBirds2_44.1k_v1.2_OneShots/Audio/House Sparrow/Call A/BIRDSong_House Sparrow, Call A 01_SARM_EB2.wav";
const BIRD_AUDIO_VOLUME_DB: f32 = -24.0;

#[derive(Default)]
pub(super) struct BirdAudioBinding {
    sources: HashMap<ParticleHandle, Uuid>,
    active_handles: HashSet<ParticleHandle>,
    positions: Vec<(ParticleHandle, Vec3)>,
}

impl BirdAudioBinding {
    fn clear(&mut self, spatial_sound_manager: &SpatialSoundManager) {
        for source_uuid in self.sources.values().copied() {
            spatial_sound_manager.remove_source(source_uuid);
        }
        self.sources.clear();
        self.active_handles.clear();
        self.positions.clear();
    }

    fn sync(
        &mut self,
        emitters: &mut [BirdEmitter],
        particle_system: &ParticleSystem,
        spatial_sound_manager: &SpatialSoundManager,
    ) {
        self.positions.clear();
        for emitter in emitters {
            emitter.collect_audio_positions(particle_system, &mut self.positions);
        }

        if self.positions.is_empty() {
            self.clear(spatial_sound_manager);
            return;
        }

        self.active_handles.clear();
        for (handle, position) in self.positions.iter().copied() {
            self.active_handles.insert(handle);

            if let Some(source_uuid) = self.sources.get(&handle).copied() {
                if let Err(err) = spatial_sound_manager.update_source_pos(source_uuid, position) {
                    log::warn!("Failed to update bird audio source position: {}", err);
                    spatial_sound_manager.remove_source(source_uuid);
                    self.sources.remove(&handle);
                }
                continue;
            }

            match spatial_sound_manager.add_looping_spatial_source(
                BIRD_AUDIO_PATH,
                BIRD_AUDIO_VOLUME_DB,
                position,
                true,
            ) {
                Ok(source_uuid) => {
                    self.sources.insert(handle, source_uuid);
                }
                Err(err) => {
                    log::warn!("Failed to spawn bird audio source: {}", err);
                }
            }
        }

        let stale_handles: Vec<ParticleHandle> = self
            .sources
            .keys()
            .copied()
            .filter(|handle| !self.active_handles.contains(handle))
            .collect();

        for handle in stale_handles {
            if let Some(source_uuid) = self.sources.remove(&handle) {
                spatial_sound_manager.remove_source(source_uuid);
            }
        }
    }
}

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
    pub(super) fn butterfly_count_from_per_chunk(butterflies_per_chunk: u32) -> u32 {
        super::CHUNK_DIM
            .x
            .saturating_mul(super::CHUNK_DIM.z)
            .saturating_mul(butterflies_per_chunk)
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
            color_low: Self::color32_to_vec4(gui_adjustables.butterfly_wing_color_low.value),
            color_high: Self::color32_to_vec4(gui_adjustables.butterfly_wing_color_high.value),
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

    pub(super) fn ensure_map_bird_emitter(&mut self) {
        if !self.bird_emitters.is_empty() {
            return;
        }

        let (center, extent) = Self::map_butterfly_region();
        self.bird_emitters.push(BirdEmitter::new_bird(
            center,
            extent,
            41_203,
            &self.bird_emitter_desc,
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
        self.ensure_map_bird_emitter();
        let wind_time = self.time_info.time_since_start();
        Self::drive_emitters(
            &mut self.butterfly_emitters,
            &mut self.particle_system,
            dt,
            wind_time,
        );
        self.constrain_butterflies_to_terrain(dt);
        Self::drive_emitters(
            &mut self.bird_emitters,
            &mut self.particle_system,
            dt,
            wind_time,
        );
        self.constrain_birds_to_terrain(dt);
        Self::drive_emitters(
            &mut self.leaf_emitters,
            &mut self.particle_system,
            dt,
            wind_time,
        );

        self.particle_system.update(dt, self.particle_forces);
        self.bird_audio_binding.sync(
            &mut self.bird_emitters,
            &self.particle_system,
            &self.spatial_sound_manager,
        );
        self.particle_system
            .write_snapshots(&mut self.particle_snapshots);

        if let Err(err) = self
            .tracer
            .upload_particles(&self.particle_snapshots, self.time_info.time_since_start())
        {
            log::error!("Failed to upload particles: {}", err);
        }
    }

    pub(super) fn constrain_butterflies_to_terrain(&mut self, dt: f32) {
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

    pub(super) fn constrain_birds_to_terrain(&mut self, dt: f32) {
        let mut query_positions_xz = Vec::new();
        let mut query_targets: Vec<BirdQueryTarget> = Vec::new();

        for (emitter_index, emitter) in self.bird_emitters.iter_mut().enumerate() {
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
                    .map(|(handle, pos_xz)| {
                        query_positions_xz.push(pos_xz);
                        BirdQueryTarget {
                            emitter_index,
                            handle,
                            kind: BirdTerrainQueryKind::Ground,
                        }
                    }),
            );

            let mut landing_positions_xz = Vec::new();
            let mut landing_handles = Vec::new();
            emitter.collect_landing_queries(&mut landing_positions_xz, &mut landing_handles);
            query_targets.extend(
                landing_handles
                    .into_iter()
                    .zip(landing_positions_xz.into_iter())
                    .map(|(handle, pos_xz)| {
                        query_positions_xz.push(pos_xz);
                        BirdQueryTarget {
                            emitter_index,
                            handle,
                            kind: BirdTerrainQueryKind::Landing,
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
        for (idx, target) in query_targets.iter().enumerate() {
            let clamped_x = query_positions_xz[idx]
                .x
                .clamp(BORDER_PADDING_INWARD, max_x);
            let clamped_z = query_positions_xz[idx]
                .y
                .clamp(BORDER_PADDING_INWARD, max_z);
            if clamped_x != query_positions_xz[idx].x || clamped_z != query_positions_xz[idx].y {
                query_positions_xz[idx] = Vec2::new(clamped_x, clamped_z);

                if matches!(target.kind, BirdTerrainQueryKind::Ground) {
                    if let Some(mut pos) = self.particle_system.position(target.handle) {
                        pos.x = clamped_x;
                        pos.z = clamped_z;
                        let _ = self.particle_system.set_position(target.handle, pos);
                    }
                }
            }
        }

        let heights = match self
            .tracer
            .query_terrain_heights_batch_with_validity(&query_positions_xz)
        {
            Ok(heights) => heights,
            Err(err) => {
                log::error!("Failed terrain query for birds: {}", err);
                return;
            }
        };

        let mut invalid_sample_count = 0usize;
        for (_idx, (target, sample)) in query_targets
            .into_iter()
            .zip(heights.into_iter())
            .enumerate()
        {
            if !sample.is_valid {
                invalid_sample_count += 1;
                if matches!(target.kind, BirdTerrainQueryKind::Landing) {
                    if let Some(emitter) = self.bird_emitters.get_mut(target.emitter_index) {
                        emitter.resample_landing_target(&self.particle_system, target.handle);
                    }
                }
                continue;
            }
            match target.kind {
                BirdTerrainQueryKind::Ground => {
                    if let Some(emitter) = self.bird_emitters.get_mut(target.emitter_index) {
                        emitter.apply_ground_height(
                            &mut self.particle_system,
                            target.handle,
                            sample.height,
                            dt,
                        );
                    }
                }
                BirdTerrainQueryKind::Landing => {
                    let src = self
                        .particle_system
                        .position(target.handle)
                        .unwrap_or(Vec3::new(
                            query_positions_xz[_idx].x,
                            sample.height,
                            query_positions_xz[_idx].y,
                        ));
                    let dst = Vec3::new(
                        query_positions_xz[_idx].x,
                        sample.height + 0.02,
                        query_positions_xz[_idx].y,
                    );
                    let waypoint = self.find_bird_flight_waypoint(src, dst, max_x, max_z);
                    if let Some(emitter) = self.bird_emitters.get_mut(target.emitter_index) {
                        emitter.apply_landing_height(
                            &self.particle_system,
                            target.handle,
                            sample.height,
                            waypoint,
                        );
                    }
                }
            }
        }

        if invalid_sample_count > 0 {
            log::warn!(
                "Invalid bird terrain samples: {}/{}",
                invalid_sample_count,
                query_positions_xz.len()
            );
        }
    }

    fn find_bird_flight_waypoint(&mut self, src: Vec3, dst: Vec3, max_x: f32, max_z: f32) -> Vec3 {
        let planar = Vec2::new(dst.x - src.x, dst.z - src.z);
        if planar.length_squared() <= 1.0e-6 {
            return dst;
        }

        const STEP_SIZE: f32 = 0.2;
        const MAX_STEPS: usize = 160;
        const CLEARANCE_EPSILON: f32 = 0.05;
        let azimuth_dir = planar.normalize();
        let mut rng = rand::rng();
        let altitude_rad = rng.random_range(30.0f32..=50.0f32).to_radians();
        let horizontal = altitude_rad.cos();
        let vertical = altitude_rad.sin();
        let t = Vec3::new(
            azimuth_dir.x * horizontal,
            vertical,
            azimuth_dir.y * horizontal,
        );

        let mut last_p = src;
        for step_idx in 1..=MAX_STEPS {
            let step_distance = step_idx as f32 * STEP_SIZE;
            let mut p = src + t * step_distance;
            p.x = p.x.clamp(0.001, max_x);
            p.z = p.z.clamp(0.001, max_z);
            last_p = p;

            let to_dst = dst - p;
            let to_dst_len = to_dst.length();
            if to_dst_len <= 1.0e-6 {
                return p;
            }

            let ray = TerrainRayQuery {
                origin: p,
                direction: to_dst / to_dst_len,
            };

            match self.tracer.query_terrain_ray_with_validity(ray) {
                Ok(hit) => {
                    let blocked = hit.is_valid
                        && (hit.position - p).length() + CLEARANCE_EPSILON < to_dst_len;
                    if !blocked {
                        return p;
                    }
                }
                Err(err) => {
                    log::warn!("Failed bird path ray query: {}", err);
                    return p;
                }
            }
        }

        last_p
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

#[derive(Clone, Copy)]
struct BirdQueryTarget {
    emitter_index: usize,
    handle: ParticleHandle,
    kind: BirdTerrainQueryKind,
}

#[derive(Clone, Copy)]
enum BirdTerrainQueryKind {
    Ground,
    Landing,
}
