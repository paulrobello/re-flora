use fastnoise_lite::{FastNoiseLite, NoiseType};
use glam::{UVec3, Vec3, Vec4};

/// Default maximum particle capacity shared between the CPU simulation and GPU buffer.
pub const PARTICLE_CAPACITY: usize = 16_384;
// Keep in sync with shader/particles/particle.vert scaling_factor (1.0 / 256.0).
const PARTICLE_POSITION_SCALE: f32 = 256.0;

/// Handle that uniquely identifies a live particle.
/// Internally, it keeps track of the slot index and a generation counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParticleHandle {
    index: u32,
    generation: u32,
}

impl ParticleHandle {
    #[allow(dead_code)]
    pub const fn invalid() -> Self {
        Self {
            index: u32::MAX,
            generation: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionMode {
    /// Default falling behaviour driven by noise and gravity.
    Falling,
    /// Free-flight particles that keep their velocity, only damped over time.
    Free,
}

/// Parameters used when spawning a new particle.
#[derive(Clone, Copy, Debug)]
pub struct ParticleSpawn {
    pub position: Vec3,
    pub velocity: Vec3,
    pub color: Vec4,
    pub size: f32,
    pub lifetime: f32,
    pub wind_factor: f32,
    pub gravity_factor: f32,
    /// Random drift direction for turbulent motion
    pub drift_direction: Vec3,
    /// Strength of the drift/turbulence
    pub drift_strength: f32,
    /// How quickly the drift changes over time
    pub drift_frequency: f32,
    /// Per-particle offset for the Perlin speed sampling to decorrelate leaves
    pub speed_noise_offset: f32,
    /// Motion integration mode for this particle.
    pub motion_mode: MotionMode,
    /// If true, particle transitions to a sinking phase when lifetime elapses.
    pub sink_on_lifetime: bool,
    /// Downward speed used during the sinking phase.
    pub sink_speed: f32,
    /// Optional texture variant for render-time atlas selection.
    pub texture_variant: u32,
}

impl Default for ParticleSpawn {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            color: Vec4::ONE,
            size: 1.0,
            lifetime: 1.0,
            wind_factor: 1.0,
            gravity_factor: 1.0,
            drift_direction: Vec3::ZERO,
            drift_strength: 0.0,
            drift_frequency: 1.0,
            speed_noise_offset: 0.0,
            motion_mode: MotionMode::Falling,
            sink_on_lifetime: false,
            sink_speed: 0.1,
            texture_variant: 0,
        }
    }
}

/// Parameters driving the global forces applied during simulation.
#[derive(Clone, Copy, Debug)]
pub struct SpeedNoise {
    /// Frequency of the Perlin sampling along time.
    pub frequency: f32,
    /// Minimum downward speed (positive value) mapped from noise.
    pub min_speed: f32,
    /// Maximum downward speed (positive value) mapped from noise.
    pub max_speed: f32,
}

impl Default for SpeedNoise {
    fn default() -> Self {
        Self {
            min_speed: -0.05,
            max_speed: 0.18,
            frequency: 0.5,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ParticleForces {
    /// Linear damping factor (0..1). Use small values to avoid instability.
    pub linear_damping: f32,
    /// Perlin-driven speed profile (used for falling leaves).
    pub speed_noise: SpeedNoise,
}

impl Default for ParticleForces {
    fn default() -> Self {
        Self {
            linear_damping: 0.0,
            speed_noise: SpeedNoise::default(),
        }
    }
}

/// A lightweight copy of particle data used by the renderer.
#[derive(Clone, Copy, Debug)]
pub struct ParticleSnapshot {
    pub position: UVec3,
    pub color: Vec4,
    pub size: f32,
    pub kind: ParticleRenderKind,
    pub texture_variant: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleRenderKind {
    Leaf,
    Butterfly,
}

/// Keeps particle data in a struct-of-arrays layout for cache-friendly updates.
pub struct ParticleSystem {
    positions: Vec<Vec3>,
    velocities: Vec<Vec3>,
    colors: Vec<Vec4>,
    sizes: Vec<f32>,
    wind_factors: Vec<f32>,
    gravity_factors: Vec<f32>,
    drift_directions: Vec<Vec3>,
    drift_strengths: Vec<f32>,
    drift_frequencies: Vec<f32>,
    lifetimes: Vec<f32>,
    ages: Vec<f32>,
    generations: Vec<u32>,
    motion_modes: Vec<MotionMode>,
    sink_on_lifetime: Vec<bool>,
    sink_speeds: Vec<f32>,
    is_sinking: Vec<bool>,
    is_alive: Vec<bool>,
    alive_indices: Vec<usize>,
    free_list: Vec<usize>,
    max_particles: usize,
    speed_noise_offsets: Vec<f32>,
    texture_variants: Vec<u32>,
    speed_noise: FastNoiseLite,
}

impl ParticleSystem {
    pub fn new(max_particles: usize) -> Self {
        assert!(max_particles > 0, "ParticleSystem needs capacity > 0");
        let zero_vec3 = Vec3::ZERO;
        let zero_vec4 = Vec4::ZERO;
        let mut free_list = Vec::with_capacity(max_particles);
        for idx in (0..max_particles).rev() {
            free_list.push(idx);
        }

        let mut speed_noise = FastNoiseLite::with_seed(1337);
        speed_noise.set_noise_type(Some(NoiseType::Perlin));
        speed_noise.set_frequency(Some(SpeedNoise::default().frequency));

        Self {
            positions: vec![zero_vec3; max_particles],
            velocities: vec![zero_vec3; max_particles],
            colors: vec![zero_vec4; max_particles],
            sizes: vec![0.0; max_particles],
            wind_factors: vec![1.0; max_particles],
            gravity_factors: vec![1.0; max_particles],
            drift_directions: vec![zero_vec3; max_particles],
            drift_strengths: vec![0.0; max_particles],
            drift_frequencies: vec![1.0; max_particles],
            lifetimes: vec![0.0; max_particles],
            ages: vec![0.0; max_particles],
            generations: vec![0; max_particles],
            motion_modes: vec![MotionMode::Falling; max_particles],
            sink_on_lifetime: vec![false; max_particles],
            sink_speeds: vec![0.1; max_particles],
            is_sinking: vec![false; max_particles],
            is_alive: vec![false; max_particles],
            alive_indices: Vec::with_capacity(max_particles),
            free_list,
            max_particles,
            speed_noise_offsets: vec![0.0; max_particles],
            texture_variants: vec![0; max_particles],
            speed_noise,
        }
    }

    fn occupy_slot(&mut self) -> Option<usize> {
        self.free_list.pop()
    }

    fn retire_slot(&mut self, slot: usize) {
        self.is_alive[slot] = false;
        self.free_list.push(slot);
    }

    fn validate_handle(&self, handle: ParticleHandle) -> Option<usize> {
        let idx = handle.index as usize;
        if idx >= self.max_particles {
            return None;
        }
        if !self.is_alive[idx] {
            return None;
        }
        if self.generations[idx] != handle.generation {
            return None;
        }
        Some(idx)
    }

    /// Number of currently active particles.
    #[allow(dead_code)]
    pub fn alive_count(&self) -> usize {
        self.alive_indices.len()
    }

    /// Maximum number of particles that can exist at once.
    #[allow(dead_code)]
    pub fn capacity(&self) -> usize {
        self.max_particles
    }

    /// Spawns a new particle using the provided description.
    /// Returns a handle that can be used to manipulate the particle later.
    pub fn spawn(&mut self, spawn: ParticleSpawn) -> Option<ParticleHandle> {
        let slot = self.occupy_slot()?;

        let new_generation = self.generations[slot].wrapping_add(1).max(1);
        self.generations[slot] = new_generation;
        self.positions[slot] = spawn.position;
        self.velocities[slot] = spawn.velocity;
        self.colors[slot] = spawn.color;
        self.sizes[slot] = spawn.size.max(0.001);
        self.wind_factors[slot] = spawn.wind_factor.max(0.0);
        self.gravity_factors[slot] = spawn.gravity_factor.max(0.0);
        self.drift_directions[slot] = spawn.drift_direction.normalize_or_zero();
        self.drift_strengths[slot] = spawn.drift_strength.max(0.0);
        self.drift_frequencies[slot] = spawn.drift_frequency.max(0.001);
        self.motion_modes[slot] = spawn.motion_mode;
        self.sink_on_lifetime[slot] = spawn.sink_on_lifetime;
        self.sink_speeds[slot] = spawn.sink_speed.max(0.01);
        self.is_sinking[slot] = false;
        self.lifetimes[slot] = spawn.lifetime.max(0.001);
        self.ages[slot] = 0.0;
        self.is_alive[slot] = true;
        self.alive_indices.push(slot);
        self.speed_noise_offsets[slot] = spawn.speed_noise_offset;
        self.texture_variants[slot] = spawn.texture_variant;

        Some(ParticleHandle {
            index: slot as u32,
            generation: new_generation,
        })
    }

    /// Marks a particle as dead immediately.
    #[allow(dead_code)]
    pub fn despawn(&mut self, handle: ParticleHandle) -> bool {
        if let Some(idx) = self.validate_handle(handle) {
            if let Some(alive_idx) = self
                .alive_indices
                .iter()
                .position(|alive_slot| *alive_slot == idx)
            {
                // this is O(1), orders of magnitude faster than remove(idx)
                // the only downside is that the order of the alive_indices is not preserved,
                // but we don't care about that in this use case
                self.alive_indices.swap_remove(alive_idx);
            }
            self.retire_slot(idx);
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.alive_indices.clear();
        self.free_list.clear();
        for idx in (0..self.max_particles).rev() {
            self.is_alive[idx] = false;
            self.free_list.push(idx);
        }
    }

    fn kill_dead_particle(&mut self, alive_list_idx: usize, slot: usize) {
        self.alive_indices.swap_remove(alive_list_idx);
        self.retire_slot(slot);
    }

    /// Advances the simulation by `dt` seconds and applies forces/damping.
    /// Supports both falling particles and free-flight motion with the same drift model.
    pub fn update(&mut self, dt: f32, forces: ParticleForces) {
        if dt <= 0.0 || self.alive_indices.is_empty() {
            return;
        }
        let damping = 1.0_f32 - forces.linear_damping.clamp(0.0, 0.999);
        let clamped_freq = forces.speed_noise.frequency.max(0.0001);
        self.speed_noise.set_frequency(Some(clamped_freq));

        let mut alive_cursor = 0;
        while alive_cursor < self.alive_indices.len() {
            let slot = self.alive_indices[alive_cursor];

            let vel = &mut self.velocities[slot];
            let mode = self.motion_modes[slot];
            let is_sinking = self.is_sinking[slot];

            // Apply randomized turbulent drift
            let age = self.ages[slot];
            let drift_phase = age * self.drift_frequencies[slot];
            let drift_offset_x = (drift_phase * 2.3).sin() * 0.7 + (drift_phase * 1.1).cos() * 0.3;
            let drift_offset_y = (drift_phase * 1.7).sin() * 0.5;
            let drift_offset_z = (drift_phase * 3.1).cos() * 0.7 + (drift_phase * 1.9).sin() * 0.3;

            let turbulence = Vec3::new(drift_offset_x, drift_offset_y, drift_offset_z);
            let drift_force =
                (self.drift_directions[slot] + turbulence * 0.5) * self.drift_strengths[slot];
            *vel += drift_force * dt;

            if is_sinking {
                vel.x *= damping * 0.96;
                vel.z *= damping * 0.96;
                vel.y = -self.sink_speeds[slot];
            } else {
                match mode {
                    MotionMode::Falling => {
                        let gravity_scale = self.gravity_factors[slot];
                        // Clamp and order the speed range
                        let (min_speed, max_speed) =
                            if forces.speed_noise.min_speed <= forces.speed_noise.max_speed {
                                (forces.speed_noise.min_speed, forces.speed_noise.max_speed)
                            } else {
                                (forces.speed_noise.max_speed, forces.speed_noise.min_speed)
                            };

                        let noise_t = age + self.speed_noise_offsets[slot];
                        let noise_val =
                            self.speed_noise.get_noise_2d(noise_t, 0.0).clamp(-1.0, 1.0);
                        let normalized = noise_val * 0.5 + 0.5; // 0..1
                        let target_speed =
                            (min_speed + (max_speed - min_speed) * normalized) * gravity_scale;

                        // Keep horizontal motion damped; vertical comes purely from noise.
                        vel.x *= damping;
                        vel.z *= damping;
                        vel.y = -target_speed;
                    }
                    MotionMode::Free => {
                        *vel *= damping;
                        let max_speed = 3.0;
                        let speed = vel.length();
                        if speed > max_speed {
                            *vel *= max_speed / speed;
                        }
                    }
                }
            }

            self.positions[slot] += *vel * dt;
            self.ages[slot] += dt;

            if !self.is_sinking[slot]
                && self.sink_on_lifetime[slot]
                && self.ages[slot] >= self.lifetimes[slot]
            {
                self.is_sinking[slot] = true;
            }

            // Sink-enabled particles only despawn once they go below the ground plane.
            let should_despawn = if self.is_sinking[slot] {
                self.positions[slot].y < 0.0
            } else {
                self.ages[slot] >= self.lifetimes[slot] || self.positions[slot].y < 0.0
            };
            if should_despawn {
                self.kill_dead_particle(alive_cursor, slot);
                continue;
            }

            alive_cursor += 1;
        }
    }

    /// Copies the alive particle data into the provided buffer for rendering.
    pub fn write_snapshots(&self, out: &mut Vec<ParticleSnapshot>) {
        out.clear();
        out.reserve(self.alive_indices.len());
        for slot in &self.alive_indices {
            let kind = match self.motion_modes[*slot] {
                MotionMode::Falling => ParticleRenderKind::Leaf,
                MotionMode::Free => ParticleRenderKind::Butterfly,
            };
            out.push(ParticleSnapshot {
                position: Self::quantize_position(self.positions[*slot]),
                color: self.colors[*slot],
                size: self.sizes[*slot],
                kind,
                texture_variant: self.texture_variants[*slot],
            });
        }
    }

    #[allow(dead_code)]
    pub fn position(&self, handle: ParticleHandle) -> Option<Vec3> {
        self.validate_handle(handle).map(|idx| self.positions[idx])
    }

    #[allow(dead_code)]
    pub fn velocity(&self, handle: ParticleHandle) -> Option<Vec3> {
        self.validate_handle(handle).map(|idx| self.velocities[idx])
    }

    #[allow(dead_code)]
    pub fn flip_planar_motion(
        &mut self,
        handle: ParticleHandle,
        flip_x: bool,
        flip_z: bool,
    ) -> bool {
        if let Some(idx) = self.validate_handle(handle) {
            if flip_x {
                self.velocities[idx].x = -self.velocities[idx].x;
                self.drift_directions[idx].x = -self.drift_directions[idx].x;
            }
            if flip_z {
                self.velocities[idx].z = -self.velocities[idx].z;
                self.drift_directions[idx].z = -self.drift_directions[idx].z;
            }
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn set_position(&mut self, handle: ParticleHandle, pos: Vec3) -> bool {
        if let Some(idx) = self.validate_handle(handle) {
            self.positions[idx] = pos;
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn set_velocity(&mut self, handle: ParticleHandle, vel: Vec3) -> bool {
        if let Some(idx) = self.validate_handle(handle) {
            self.velocities[idx] = vel;
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn set_color(&mut self, handle: ParticleHandle, color: Vec4) -> bool {
        if let Some(idx) = self.validate_handle(handle) {
            self.colors[idx] = color;
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn set_size(&mut self, handle: ParticleHandle, size: f32) -> bool {
        if let Some(idx) = self.validate_handle(handle) {
            self.sizes[idx] = size.max(0.001);
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn add_velocity(&mut self, handle: ParticleHandle, delta: Vec3) -> bool {
        if let Some(idx) = self.validate_handle(handle) {
            self.velocities[idx] += delta;
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn is_alive_handle(&self, handle: ParticleHandle) -> bool {
        self.validate_handle(handle).is_some()
    }

    fn quantize_position(position: Vec3) -> UVec3 {
        let scaled = (position * PARTICLE_POSITION_SCALE).round();
        let clamp_component = |component: f32| component.clamp(0.0, u32::MAX as f32) as u32;
        UVec3::new(
            clamp_component(scaled.x),
            clamp_component(scaled.y),
            clamp_component(scaled.z),
        )
    }
}
