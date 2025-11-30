use std::{f32::consts::TAU, ops::RangeInclusive};

use glam::{Vec3, Vec4};
use rand::{rngs::SmallRng, Rng, SeedableRng};

use super::{ParticleSpawn, ParticleSystem};
use crate::wind::Wind;

pub trait ParticleEmitter {
    fn update(&mut self, system: &mut ParticleSystem, dt: f32, time: f32);
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

#[derive(Clone, Copy, Debug)]
pub struct LeafEmitterDesc {
    pub spawn_rate: f32,
    pub wind_spawn_min_strength: f32,
    pub wind_spawn_max_strength: f32,
    pub wind_spawn_power: f32,
}

impl Default for LeafEmitterDesc {
    fn default() -> Self {
        Self {
            spawn_rate: 5.0,
            wind_spawn_min_strength: 0.4,
            wind_spawn_max_strength: 1.0,
            wind_spawn_power: 1.0,
        }
    }
}

pub struct FallenLeafEmitter {
    pub center: Vec3,
    pub spawn_rate: f32,
    pub vertical_speed: RangeInclusive<f32>,
    pub size: f32,
    pub lifetime: RangeInclusive<f32>,
    pub color_low: Vec4,
    pub color_high: Vec4,
    pub wind_spawn_min_strength: f32,
    pub wind_spawn_max_strength: f32,
    pub wind_spawn_power: f32,
    leaf_positions: Vec<Vec3>,
    rng: SmallRng,
    spawn_accumulator: f32,
    wind: Wind,
}

impl FallenLeafEmitter {
    pub fn new(
        center: Vec3,
        leaf_positions: Vec<Vec3>,
        seed: u64,
        color_low: Vec4,
        color_high: Vec4,
        desc: &LeafEmitterDesc,
    ) -> Self {
        Self {
            center,
            spawn_rate: desc.spawn_rate,
            vertical_speed: -1.5..=-0.3,
            size: 1.0 / 256.0,
            lifetime: 12.0..=24.0,
            color_low,
            color_high,
            wind_spawn_min_strength: desc.wind_spawn_min_strength,
            wind_spawn_max_strength: desc.wind_spawn_max_strength,
            wind_spawn_power: desc.wind_spawn_power,
            leaf_positions,
            rng: SmallRng::seed_from_u64(seed),
            spawn_accumulator: 0.0,
            wind: Wind::new(),
        }
    }

    fn spawn_leaf(&mut self, system: &mut ParticleSystem) {
        let spawn_position = if self.leaf_positions.is_empty() {
            self.center
        } else {
            let leaf_idx = self.rng.random_range(0..self.leaf_positions.len());
            self.leaf_positions[leaf_idx]
        };
        let mut velocity = Vec3::ZERO;
        velocity.y = random_in_range(&mut self.rng, &self.vertical_speed);
        let roll_angle = self.rng.random_range(0.0..TAU);
        let roll_strength = self.rng.random_range(0.05..=0.2);
        velocity.x += roll_angle.cos() * roll_strength;
        velocity.z += roll_angle.sin() * roll_strength;

        let wind_factor = self.rng.random_range(0.6..=1.4);
        let gravity_factor = self.rng.random_range(0.8..=1.2);

        // Randomize drift direction for turbulent motion
        let drift_angle = self.rng.random_range(0.0..TAU);
        let drift_direction = Vec3::new(
            drift_angle.cos(),
            self.rng.random_range(-0.2..=0.2),
            drift_angle.sin(),
        );
        let drift_strength = self.rng.random_range(0.3..=0.8);
        let drift_frequency = self.rng.random_range(0.5..=2.0);

        let spawn = ParticleSpawn {
            position: spawn_position,
            velocity,
            color: random_color(&mut self.rng, self.color_low, self.color_high),
            size: self.size,
            lifetime: random_in_range(&mut self.rng, &self.lifetime),
            wind_factor,
            gravity_factor,
            drift_direction,
            drift_strength,
            drift_frequency,
        };
        let _ = system.spawn(spawn);
    }

    fn wind_spawn_multiplier(&self, time: f32) -> f32 {
        let normalized_strength = self
            .wind
            .sample_normalized(self.center, time)
            .length()
            .clamp(0.0, 1.0);
        let (min_strength, max_strength) =
            if self.wind_spawn_min_strength <= self.wind_spawn_max_strength {
                (self.wind_spawn_min_strength, self.wind_spawn_max_strength)
            } else {
                (self.wind_spawn_max_strength, self.wind_spawn_min_strength)
            };
        let range = max_strength - min_strength;
        if range <= f32::EPSILON {
            return if normalized_strength >= max_strength {
                1.0
            } else {
                0.0
            };
        }
        let scaled = ((normalized_strength - min_strength) / range).clamp(0.0, 1.0);
        let exponent = self.wind_spawn_power.max(0.001);
        scaled.powf(exponent)
    }
}

impl ParticleEmitter for FallenLeafEmitter {
    fn update(&mut self, system: &mut ParticleSystem, dt: f32, time: f32) {
        if self.spawn_rate <= 0.0 {
            return;
        }
        let wind_multiplier = self.wind_spawn_multiplier(time);
        if wind_multiplier <= 0.0 {
            return;
        }
        let effective_spawn_rate = self.spawn_rate * wind_multiplier;
        self.spawn_accumulator += effective_spawn_rate * dt;
        while self.spawn_accumulator >= 1.0 {
            self.spawn_leaf(system);
            self.spawn_accumulator -= 1.0;
        }
    }
}
