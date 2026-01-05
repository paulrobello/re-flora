use std::{f32::consts::TAU, ops::RangeInclusive};

use glam::{Vec3, Vec4};
use rand::{rngs::SmallRng, Rng, SeedableRng};

use super::{MotionMode, ParticleHandle, ParticleSpawn, ParticleSystem};
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
    let ordered = |a: f32, b: f32| -> (f32, f32) { (a.min(b), a.max(b)) };
    let (min_x, max_x) = ordered(low.x, high.x);
    let (min_y, max_y) = ordered(low.y, high.y);
    let (min_z, max_z) = ordered(low.z, high.z);
    let (min_w, max_w) = ordered(low.w, high.w);

    Vec4::new(
        rng.random_range(min_x..=max_x),
        rng.random_range(min_y..=max_y),
        rng.random_range(min_z..=max_z),
        rng.random_range(min_w..=max_w),
    )
}

#[derive(Clone, Copy, Debug)]
pub struct LeafEmitterDesc {
    pub spawn_rate: f32,
    pub size: f32,
    pub lifetime_min: f32,
    pub lifetime_max: f32,
    pub color_low: Vec4,
    pub color_high: Vec4,
    pub wind_spawn_min_strength: f32,
    pub wind_spawn_max_strength: f32,
    pub wind_spawn_power: f32,
}

impl Default for LeafEmitterDesc {
    fn default() -> Self {
        Self {
            spawn_rate: 0.5,
            size: 1.0 / 256.0,
            lifetime_min: 120.0,
            lifetime_max: 240.0,
            color_low: Vec4::new(212.0 / 255.0, 111.0 / 255.0, 0.0, 1.0),
            color_high: Vec4::new(242.0 / 255.0, 205.0 / 255.0, 0.0, 1.0),
            wind_spawn_min_strength: 0.5,
            wind_spawn_max_strength: 1.0,
            wind_spawn_power: 1.0,
        }
    }
}

pub struct FallenLeafEmitter {
    pub center: Vec3,
    pub spawn_rate: f32,
    pub fall_chance: f32,
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
    pub fn new(center: Vec3, leaf_positions: Vec<Vec3>, seed: u64, desc: &LeafEmitterDesc) -> Self {
        let mut rng = SmallRng::seed_from_u64(seed);
        let fall_chance = rng.random_range(0.2..=1.0);
        Self {
            center,
            spawn_rate: desc.spawn_rate,
            fall_chance,
            size: desc.size,
            lifetime: desc.lifetime_min..=desc.lifetime_max,
            color_low: desc.color_low,
            color_high: desc.color_high,
            wind_spawn_min_strength: desc.wind_spawn_min_strength,
            wind_spawn_max_strength: desc.wind_spawn_max_strength,
            wind_spawn_power: desc.wind_spawn_power,
            leaf_positions,
            rng,
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
        let roll_angle = self.rng.random_range(0.0..TAU);
        let roll_strength = self.rng.random_range(0.05..=0.2);
        velocity.x += roll_angle.cos() * roll_strength;
        velocity.z += roll_angle.sin() * roll_strength;

        let wind_factor = self.rng.random_range(0.6..=1.4);
        let gravity_factor = self.rng.random_range(0.8..=1.0);

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
            speed_noise_offset: self.rng.random_range(0.0..10_000.0),
            motion_mode: MotionMode::Falling,
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
        let wind_multiplier = self.wind_spawn_multiplier(time) * self.fall_chance;
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

#[derive(Clone, Copy, Debug)]
pub struct ButterflyEmitterDesc {
    pub enabled: bool,
    pub spawn_rate: f32,
    pub max_butterflies: u32,
    pub wander_radius: f32,
    pub height_offset_min: f32,
    pub height_offset_max: f32,
    pub lifetime_min: f32,
    pub lifetime_max: f32,
    pub size_min: f32,
    pub size_max: f32,
    pub drift_strength_min: f32,
    pub drift_strength_max: f32,
    pub drift_frequency_min: f32,
    pub drift_frequency_max: f32,
    pub steering_strength: f32,
    pub color_low: Vec4,
    pub color_high: Vec4,
}

impl Default for ButterflyEmitterDesc {
    fn default() -> Self {
        Self {
            enabled: true,
            spawn_rate: 3.0,
            max_butterflies: 6,
            wander_radius: 2.5,
            height_offset_min: 0.4,
            height_offset_max: 2.0,
            lifetime_min: 8.0,
            lifetime_max: 14.0,
            size_min: 0.006,
            size_max: 0.012,
            drift_strength_min: 0.6,
            drift_strength_max: 1.4,
            drift_frequency_min: 1.5,
            drift_frequency_max: 3.5,
            steering_strength: 0.9,
            color_low: Vec4::new(0.88, 0.68, 0.92, 1.0),
            color_high: Vec4::new(0.98, 0.9, 0.78, 1.0),
        }
    }
}

pub struct ButterflyEmitter {
    pub center: Vec3,
    pub wander_radius: f32,
    min_wander_radius: f32,
    pub height_offset: RangeInclusive<f32>,
    pub lifetime: RangeInclusive<f32>,
    pub size: RangeInclusive<f32>,
    pub drift_strength: RangeInclusive<f32>,
    pub drift_frequency: RangeInclusive<f32>,
    pub steering_strength: f32,
    pub color_low: Vec4,
    pub color_high: Vec4,
    pub enabled: bool,
    pub max_butterflies: u32,
    pub spawn_rate: f32,
    rng: SmallRng,
    spawn_accumulator: f32,
    active_handles: Vec<ParticleHandle>,
}

impl ButterflyEmitter {
    pub fn new(center: Vec3, extent: Vec3, seed: u64, desc: &ButterflyEmitterDesc) -> Self {
        let min_wander_radius = desc
            .wander_radius
            .max(extent.x.max(extent.z) * 0.35)
            .max(0.1);
        let mut emitter = Self {
            center,
            wander_radius: min_wander_radius,
            min_wander_radius,
            height_offset: desc.height_offset_min.min(desc.height_offset_max)
                ..=desc.height_offset_max.max(desc.height_offset_min),
            lifetime: desc.lifetime_min.min(desc.lifetime_max)
                ..=desc.lifetime_max.max(desc.lifetime_min),
            size: desc.size_min.min(desc.size_max)..=desc.size_max.max(desc.size_min),
            drift_strength: desc.drift_strength_min.min(desc.drift_strength_max)
                ..=desc.drift_strength_max.max(desc.drift_strength_min),
            drift_frequency: desc.drift_frequency_min.min(desc.drift_frequency_max)
                ..=desc.drift_frequency_max.max(desc.drift_frequency_min),
            steering_strength: desc.steering_strength.max(0.0),
            color_low: desc.color_low,
            color_high: desc.color_high,
            enabled: desc.enabled,
            max_butterflies: desc.max_butterflies,
            spawn_rate: desc.spawn_rate,
            rng: SmallRng::seed_from_u64(seed),
            spawn_accumulator: 0.0,
            active_handles: Vec::new(),
        };
        emitter.clamp_height(center.y);
        emitter
    }

    pub fn apply_desc(&mut self, desc: &ButterflyEmitterDesc) {
        self.enabled = desc.enabled;
        self.spawn_rate = desc.spawn_rate;
        self.max_butterflies = desc.max_butterflies;
        self.wander_radius = desc.wander_radius.max(self.min_wander_radius).max(0.1);
        self.height_offset = desc.height_offset_min.min(desc.height_offset_max)
            ..=desc.height_offset_max.max(desc.height_offset_min);
        self.lifetime =
            desc.lifetime_min.min(desc.lifetime_max)..=desc.lifetime_max.max(desc.lifetime_min);
        self.size = desc.size_min.min(desc.size_max)..=desc.size_max.max(desc.size_min);
        self.drift_strength = desc.drift_strength_min.min(desc.drift_strength_max)
            ..=desc.drift_strength_max.max(desc.drift_strength_min);
        self.drift_frequency = desc.drift_frequency_min.min(desc.drift_frequency_max)
            ..=desc.drift_frequency_max.max(desc.drift_frequency_min);
        self.steering_strength = desc.steering_strength.max(0.0);
        self.color_low = desc.color_low;
        self.color_high = desc.color_high;
        self.clamp_height(self.center.y);
    }

    fn clamp_height(&mut self, base_height: f32) {
        let min = *self.height_offset.start();
        let max = *self.height_offset.end();
        let clamped_min = (base_height + min).max(0.05) - base_height;
        let clamped_max = (base_height + max).max(clamped_min + 0.01) - base_height;
        self.height_offset = clamped_min..=clamped_max;
    }

    fn prune_handles(&mut self, system: &ParticleSystem) {
        self.active_handles
            .retain(|handle| system.is_alive_handle(*handle));
    }

    fn steer_towards_home(&mut self, system: &mut ParticleSystem, dt: f32) {
        let max_height_offset = *self.height_offset.end();
        let min_height_offset = *self.height_offset.start();
        let steering = self.steering_strength * dt;
        for handle in &self.active_handles {
            if let Some(pos) = system.position(*handle) {
                let relative = pos - self.center;
                let horizontal = Vec3::new(relative.x, 0.0, relative.z);
                if horizontal.length_squared() > self.wander_radius * self.wander_radius {
                    let pull = -horizontal.normalize_or_zero() * steering;
                    let _ = system.add_velocity(*handle, pull);
                }

                if relative.y < min_height_offset {
                    let _ = system.add_velocity(*handle, Vec3::new(0.0, steering, 0.0));
                } else if relative.y > max_height_offset {
                    let _ = system.add_velocity(*handle, Vec3::new(0.0, -steering, 0.0));
                }
            }
        }
    }

    fn spawn_butterfly(&mut self, system: &mut ParticleSystem) -> Option<ParticleHandle> {
        let radius_factor = self.rng.random_range(0.35..=1.0);
        let angle = self.rng.random_range(0.0..TAU);
        let radius = self.wander_radius * radius_factor;
        let height_offset = random_in_range(&mut self.rng, &self.height_offset);

        let mut position = self.center;
        position.x += angle.cos() * radius;
        position.z += angle.sin() * radius;
        position.y += height_offset;

        let drift_strength = random_in_range(&mut self.rng, &self.drift_strength);
        let drift_frequency = random_in_range(&mut self.rng, &self.drift_frequency);

        let yaw = self.rng.random_range(0.0..TAU);
        let vertical_bias = self.rng.random_range(-0.2..=0.35);
        let drift_direction = Vec3::new(yaw.cos(), vertical_bias, yaw.sin()).normalize_or_zero();

        let spawn = ParticleSpawn {
            position,
            velocity: drift_direction * drift_strength * 0.35,
            color: random_color(&mut self.rng, self.color_low, self.color_high),
            size: random_in_range(&mut self.rng, &self.size),
            lifetime: random_in_range(&mut self.rng, &self.lifetime),
            wind_factor: 0.0,
            gravity_factor: 0.0,
            drift_direction,
            drift_strength,
            drift_frequency,
            speed_noise_offset: self.rng.random_range(0.0..10_000.0),
            motion_mode: MotionMode::Free,
        };

        system.spawn(spawn)
    }
}

impl ParticleEmitter for ButterflyEmitter {
    fn update(&mut self, system: &mut ParticleSystem, dt: f32, _time: f32) {
        self.prune_handles(system);
        if !self.enabled || self.spawn_rate <= 0.0 || self.max_butterflies == 0 {
            self.spawn_accumulator = 0.0;
            return;
        }

        let max_count = self.max_butterflies as usize;
        self.spawn_accumulator += self.spawn_rate * dt;
        while self.spawn_accumulator >= 1.0 && self.active_handles.len() < max_count {
            if let Some(handle) = self.spawn_butterfly(system) {
                self.active_handles.push(handle);
            }
            self.spawn_accumulator -= 1.0;
        }

        self.steer_towards_home(system, dt);
    }
}
