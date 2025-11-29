use crate::wind::Wind;
use glam::{Vec3, Vec4};

/// Default maximum particle capacity shared between the CPU simulation and GPU buffer.
pub const PARTICLE_CAPACITY: usize = 16_384;

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

/// Parameters used when spawning a new particle.
#[derive(Clone, Copy, Debug)]
pub struct ParticleSpawn {
    pub position: Vec3,
    pub velocity: Vec3,
    pub color: Vec4,
    pub size: f32,
    pub lifetime: f32,
}

impl Default for ParticleSpawn {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            color: Vec4::ONE,
            size: 1.0,
            lifetime: 1.0,
        }
    }
}

/// Parameters driving the global forces applied during simulation.
#[derive(Clone, Copy, Debug)]
pub struct ParticleForces {
    /// Constant acceleration applied to every particle. Use for gravity/wind.
    pub global_acceleration: Vec3,
    /// Linear damping factor (0..1). Use small values to avoid instability.
    pub linear_damping: f32,
}

impl Default for ParticleForces {
    fn default() -> Self {
        Self {
            global_acceleration: Vec3::ZERO,
            linear_damping: 0.0,
        }
    }
}

/// A lightweight copy of particle data used by the renderer.
#[derive(Clone, Copy, Debug)]
pub struct ParticleSnapshot {
    pub position: Vec3,
    pub color: Vec4,
    pub size: f32,
}

/// Keeps particle data in a struct-of-arrays layout for cache-friendly updates.
pub struct ParticleSystem {
    positions: Vec<Vec3>,
    velocities: Vec<Vec3>,
    colors: Vec<Vec4>,
    sizes: Vec<f32>,
    lifetimes: Vec<f32>,
    ages: Vec<f32>,
    generations: Vec<u32>,
    is_alive: Vec<bool>,
    alive_indices: Vec<usize>,
    free_list: Vec<usize>,
    max_particles: usize,
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

        Self {
            positions: vec![zero_vec3; max_particles],
            velocities: vec![zero_vec3; max_particles],
            colors: vec![zero_vec4; max_particles],
            sizes: vec![0.0; max_particles],
            lifetimes: vec![0.0; max_particles],
            ages: vec![0.0; max_particles],
            generations: vec![0; max_particles],
            is_alive: vec![false; max_particles],
            alive_indices: Vec::with_capacity(max_particles),
            free_list,
            max_particles,
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
        self.lifetimes[slot] = spawn.lifetime.max(0.001);
        self.ages[slot] = 0.0;
        self.is_alive[slot] = true;
        self.alive_indices.push(slot);

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
    pub fn update(&mut self, dt: f32, forces: ParticleForces) {
        if dt <= 0.0 || self.alive_indices.is_empty() {
            return;
        }
        let damping = 1.0_f32 - forces.linear_damping.clamp(0.0, 0.999);

        let mut alive_cursor = 0;
        while alive_cursor < self.alive_indices.len() {
            let slot = self.alive_indices[alive_cursor];

            let vel = &mut self.velocities[slot];
            *vel += forces.global_acceleration * dt;
            *vel *= damping;

            self.positions[slot] += *vel * dt;
            self.ages[slot] += dt;

            // expire particles when lifetime ends or once they fall below the ground plane
            let should_despawn =
                self.ages[slot] >= self.lifetimes[slot] || self.positions[slot].y < 0.0;
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
            out.push(ParticleSnapshot {
                position: self.positions[*slot],
                color: self.colors[*slot],
                size: self.sizes[*slot],
            });
        }
    }

    #[allow(dead_code)]
    pub fn position(&self, handle: ParticleHandle) -> Option<Vec3> {
        self.validate_handle(handle).map(|idx| self.positions[idx])
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

    /// Pushes particle velocities toward the sampled wind field.
    pub fn apply_wind(&mut self, dt: f32, wind: &Wind, time: f32, responsiveness: f32) {
        if dt <= 0.0 || self.alive_indices.is_empty() {
            return;
        }
        let gain = (responsiveness.max(0.0)) * dt;
        if gain <= 0.0 {
            return;
        }

        for &slot in &self.alive_indices {
            let target_vel = wind.sample(self.positions[slot], time);
            let delta = target_vel - self.velocities[slot];
            self.velocities[slot] += delta * gain;
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
}
