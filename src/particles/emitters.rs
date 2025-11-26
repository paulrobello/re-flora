use std::ops::RangeInclusive;

use glam::{Vec3, Vec4};
use rand::{rngs::SmallRng, Rng, SeedableRng};

use super::{ParticleHandle, ParticleSpawn, ParticleSystem};

pub trait ParticleEmitter {
    fn update(&mut self, system: &mut ParticleSystem, dt: f32);
}

fn random_in_range(rng: &mut SmallRng, range: &RangeInclusive<f32>) -> f32 {
    let start = *range.start();
    let end = *range.end();
    if (end - start).abs() <= f32::EPSILON {
        start
    } else {
        rng.random_range(start..=end)
    }
}

fn random_color(rng: &mut SmallRng, low: Vec4, high: Vec4) -> Vec4 {
    Vec4::new(
        rng.random_range(low.x..=high.x),
        rng.random_range(low.y..=high.y),
        rng.random_range(low.z..=high.z),
        rng.random_range(low.w..=high.w),
    )
}

pub struct FallenLeafEmitter {
    pub center: Vec3,
    pub extent: Vec3,
    pub spawn_rate: f32,
    pub base_velocity: Vec3,
    pub vertical_speed: RangeInclusive<f32>,
    pub horizontal_jitter: f32,
    pub size: RangeInclusive<f32>,
    pub lifetime: RangeInclusive<f32>,
    pub color_low: Vec4,
    pub color_high: Vec4,
    rng: SmallRng,
    spawn_accumulator: f32,
}

impl FallenLeafEmitter {
    pub fn new(center: Vec3, extent: Vec3, seed: u64) -> Self {
        Self {
            center,
            extent,
            spawn_rate: 100.0,
            base_velocity: Vec3::new(0.0, -0.5, 0.0),
            vertical_speed: -1.5..=-0.3,
            horizontal_jitter: 0.5,
            size: 0.05..=0.12,
            lifetime: 4.0..=8.0,
            color_low: Vec4::new(0.7, 0.3, 0.05, 1.0),
            color_high: Vec4::new(0.95, 0.65, 0.25, 1.0),
            rng: SmallRng::seed_from_u64(seed),
            spawn_accumulator: 0.0,
        }
    }

    fn spawn_leaf(&mut self, system: &mut ParticleSystem) {
        let offset = Vec3::new(
            self.rng
                .random_range(-self.extent.x..=self.extent.x),
            self.rng
                .random_range(-self.extent.y..=self.extent.y),
            self.rng
                .random_range(-self.extent.z..=self.extent.z),
        );
        let mut velocity = self.base_velocity;
        velocity.y = random_in_range(&mut self.rng, &self.vertical_speed);
        velocity.x += self.rng.random_range(-self.horizontal_jitter..=self.horizontal_jitter);
        velocity.z += self.rng.random_range(-self.horizontal_jitter..=self.horizontal_jitter);

        let spawn = ParticleSpawn {
            position: self.center + offset,
            velocity,
            color: random_color(&mut self.rng, self.color_low, self.color_high),
            size: random_in_range(&mut self.rng, &self.size),
            lifetime: random_in_range(&mut self.rng, &self.lifetime),
        };
        let _ = system.spawn(spawn);
    }
}

impl ParticleEmitter for FallenLeafEmitter {
    fn update(&mut self, system: &mut ParticleSystem, dt: f32) {
        if self.spawn_rate <= 0.0 {
            return;
        }
        self.spawn_accumulator += self.spawn_rate * dt;
        while self.spawn_accumulator >= 1.0 {
            self.spawn_leaf(system);
            self.spawn_accumulator -= 1.0;
        }
    }
}

struct ButterflyState {
    handle: Option<ParticleHandle>,
    phase: f32,
    vertical_phase: f32,
    angular_speed: f32,
    orbit_radius: f32,
    height_offset: f32,
    size: f32,
    color: Vec4,
    last_position: Vec3,
}

pub struct ButterflyEmitter {
    pub center: Vec3,
    pub target_count: usize,
    pub orbit_radius: RangeInclusive<f32>,
    pub height: RangeInclusive<f32>,
    pub angular_speed: RangeInclusive<f32>,
    pub flutter_speed: RangeInclusive<f32>,
    pub size: RangeInclusive<f32>,
    pub lifetime: f32,
    pub color_low: Vec4,
    pub color_high: Vec4,
    rng: SmallRng,
    butterflies: Vec<ButterflyState>,
}

impl ButterflyEmitter {
    pub fn new(center: Vec3, target_count: usize, seed: u64) -> Self {
        Self {
            center,
            target_count,
            orbit_radius: 0.6..=1.2,
            height: 0.5..=1.5,
            angular_speed: 0.5..=1.5,
            flutter_speed: 1.0..=3.0,
            size: 0.05..=0.12,
            lifetime: 30.0,
            color_low: Vec4::new(0.8, 0.3, 0.8, 1.0),
            color_high: Vec4::new(1.0, 0.6, 0.9, 1.0),
            rng: SmallRng::seed_from_u64(seed),
            butterflies: Vec::new(),
        }
    }

    fn ensure_capacity(&mut self) {
        if self.butterflies.len() < self.target_count {
            self.butterflies
                .resize_with(self.target_count, || ButterflyState {
                    handle: None,
                    phase: 0.0,
                    vertical_phase: 0.0,
                    angular_speed: 1.0,
                    orbit_radius: 1.0,
                    height_offset: 1.0,
                    size: 0.1,
                    color: Vec4::ONE,
                    last_position: self.center,
                });
        }
    }

    fn spawn_state(
        state: &mut ButterflyState,
        system: &mut ParticleSystem,
        rng: &mut SmallRng,
        center: Vec3,
        lifetime: f32,
        color_low: Vec4,
        color_high: Vec4,
        angular_speed: &RangeInclusive<f32>,
        orbit_radius: &RangeInclusive<f32>,
        height: &RangeInclusive<f32>,
        size: &RangeInclusive<f32>,
    ) {
        state.phase = rng.random_range(0.0..=std::f32::consts::TAU);
        state.vertical_phase = rng.random_range(0.0..=std::f32::consts::TAU);
        state.angular_speed = random_in_range(rng, angular_speed);
        state.orbit_radius = random_in_range(rng, orbit_radius);
        state.height_offset = random_in_range(rng, height);
        state.size = random_in_range(rng, size);
        state.color = random_color(rng, color_low, color_high);

        let position = center
            + Vec3::new(
                state.orbit_radius * state.phase.cos(),
                state.height_offset,
                state.orbit_radius * state.phase.sin(),
            );
        state.last_position = position;

        let spawn = ParticleSpawn {
            position,
            velocity: Vec3::ZERO,
            color: state.color,
            size: state.size,
            lifetime,
        };

        state.handle = system.spawn(spawn);
    }

    fn update_state(
        state: &mut ButterflyState,
        system: &mut ParticleSystem,
        rng: &mut SmallRng,
        dt: f32,
        center: Vec3,
        flutter_speed: &RangeInclusive<f32>,
    ) {
        state.phase += state.angular_speed * dt;
        state.vertical_phase += random_in_range(rng, flutter_speed) * dt;

        let flutter = state.vertical_phase.sin() * 0.2;
        let new_position = center
            + Vec3::new(
                state.orbit_radius * state.phase.cos(),
                state.height_offset + flutter,
                state.orbit_radius * state.phase.sin(),
            );

        if let Some(handle) = state.handle {
            system.set_position(handle, new_position);
            let velocity = (new_position - state.last_position) / dt.max(1e-3);
            system.set_velocity(handle, velocity);
            system.set_color(handle, state.color);
            system.set_size(handle, state.size);
        }
        state.last_position = new_position;
    }
}

impl ParticleEmitter for ButterflyEmitter {
    fn update(&mut self, system: &mut ParticleSystem, dt: f32) {
        if self.target_count == 0 {
            return;
        }
        self.ensure_capacity();
        let mut rng = self.rng.clone();
        let center = self.center;
        let lifetime = self.lifetime;
        let color_low = self.color_low;
        let color_high = self.color_high;
        let angular_speed = self.angular_speed.clone();
        let orbit_radius = self.orbit_radius.clone();
        let height = self.height.clone();
        let size = self.size.clone();
        let flutter_speed = self.flutter_speed.clone();

        for state in &mut self.butterflies {
            let handle_alive = state
                .handle
                .map(|handle| system.is_alive_handle(handle))
                .unwrap_or(false);
            if !handle_alive {
                Self::spawn_state(
                    state,
                    system,
                    &mut rng,
                    center,
                    lifetime,
                    color_low,
                    color_high,
                    &angular_speed,
                    &orbit_radius,
                    &height,
                    &size,
                );
            }

            if state.handle.is_some() {
                Self::update_state(
                    state,
                    system,
                    &mut rng,
                    dt,
                    center,
                    &flutter_speed,
                );
            }
        }

        self.rng = rng;
    }
}
