use std::ops::RangeInclusive;

use glam::{Vec3, Vec4};
use rand::{rngs::SmallRng, Rng, SeedableRng};

use super::{ParticleSpawn, ParticleSystem};

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
    pub size: f32,
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
            size: 1.0 / 256.0,
            lifetime: 4.0..=8.0,
            color_low: Vec4::new(0.7, 0.3, 0.05, 1.0),
            color_high: Vec4::new(0.95, 0.65, 0.25, 1.0),
            rng: SmallRng::seed_from_u64(seed),
            spawn_accumulator: 0.0,
        }
    }

    fn spawn_leaf(&mut self, system: &mut ParticleSystem) {
        let offset = Vec3::new(
            self.rng.random_range(-self.extent.x..=self.extent.x),
            self.rng.random_range(-self.extent.y..=self.extent.y),
            self.rng.random_range(-self.extent.z..=self.extent.z),
        );
        let mut velocity = self.base_velocity;
        velocity.y = random_in_range(&mut self.rng, &self.vertical_speed);
        velocity.x += self
            .rng
            .random_range(-self.horizontal_jitter..=self.horizontal_jitter);
        velocity.z += self
            .rng
            .random_range(-self.horizontal_jitter..=self.horizontal_jitter);

        let spawn = ParticleSpawn {
            position: self.center + offset,
            velocity,
            color: random_color(&mut self.rng, self.color_low, self.color_high),
            size: self.size,
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
