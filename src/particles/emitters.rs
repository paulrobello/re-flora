use std::{f32::consts::TAU, ops::RangeInclusive};

use crate::wind::Wind;
use glam::{Vec3, Vec4};
use rand::{rngs::SmallRng, Rng, SeedableRng};

use super::{ParticleSpawn, ParticleSystem};

pub trait ParticleEmitter {
    fn update(&mut self, system: &mut ParticleSystem, dt: f32, wind: &Wind, time: f32);
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
    pub size: f32,
    pub lifetime: RangeInclusive<f32>,
    pub color_low: Vec4,
    pub color_high: Vec4,
    leaf_positions: Vec<Vec3>,
    rng: SmallRng,
    spawn_accumulator: f32,
}

impl FallenLeafEmitter {
    pub fn new(center: Vec3, extent: Vec3, leaf_positions: Vec<Vec3>, seed: u64) -> Self {
        Self {
            center,
            extent,
            spawn_rate: 50.0,
            base_velocity: Vec3::new(0.0, -0.5, 0.0),
            vertical_speed: -1.5..=-0.3,
            size: 1.0 / 256.0,
            lifetime: 12.0..=24.0,
            color_low: Vec4::new(0.7, 0.3, 0.05, 1.0),
            color_high: Vec4::new(0.95, 0.65, 0.25, 1.0),
            leaf_positions,
            rng: SmallRng::seed_from_u64(seed),
            spawn_accumulator: 0.0,
        }
    }

    pub fn set_leaf_data(&mut self, leaf_positions: Vec<Vec3>) {
        self.leaf_positions = leaf_positions;
    }

    fn spawn_leaf(&mut self, system: &mut ParticleSystem) {
        let spawn_position = if self.leaf_positions.is_empty() {
            self.center
        } else {
            let leaf_idx = self.rng.random_range(0..self.leaf_positions.len());
            self.leaf_positions[leaf_idx]
        };
        let mut velocity = self.base_velocity;
        velocity.y = random_in_range(&mut self.rng, &self.vertical_speed);
        let roll_angle = self.rng.random_range(0.0..TAU);
        let roll_strength = self.rng.random_range(0.05..=0.2);
        velocity.x += roll_angle.cos() * roll_strength;
        velocity.z += roll_angle.sin() * roll_strength;

        let wind_factor = self.rng.random_range(0.6..=1.4);
        let gravity_factor = self.rng.random_range(0.8..=1.2);

        let spawn = ParticleSpawn {
            position: spawn_position,
            velocity,
            color: random_color(&mut self.rng, self.color_low, self.color_high),
            size: self.size,
            lifetime: random_in_range(&mut self.rng, &self.lifetime),
            wind_factor,
            gravity_factor,
        };
        let _ = system.spawn(spawn);
    }
}

impl ParticleEmitter for FallenLeafEmitter {
    fn update(&mut self, system: &mut ParticleSystem, dt: f32, wind: &Wind, time: f32) {
        if self.spawn_rate <= 0.0 {
            return;
        }
        let normalized_wind = wind.sample_normalized(self.center, time).length();
        let wind_multiplier = (0.5 + normalized_wind.clamp(0.0, 1.0)).clamp(0.25, 2.0);
        self.spawn_accumulator += self.spawn_rate * wind_multiplier * dt;
        while self.spawn_accumulator >= 1.0 {
            self.spawn_leaf(system);
            self.spawn_accumulator -= 1.0;
        }
    }
}
