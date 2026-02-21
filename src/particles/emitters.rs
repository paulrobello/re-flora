use std::{collections::HashMap, f32::consts::TAU, ops::RangeInclusive};

use glam::{Vec2, Vec3, Vec4};
use rand::{rngs::SmallRng, Rng, SeedableRng};

use super::{MotionMode, ParticleHandle, ParticleRenderKind, ParticleSpawn, ParticleSystem};
use crate::util::get_project_root;
use crate::wind::Wind;

pub trait ParticleEmitter {
    fn update(&mut self, system: &mut ParticleSystem, dt: f32, time: f32);
}

const BUTTERFLY_TEXTURE_DIR_REL_PATH: &str = "assets/texture/butterfly_16px";
const BIRD_TEXTURE_FILE_REL_PATH: &str = "assets/texture/Bird/Individual Sprites/BirdIdle1.png";

fn discover_butterfly_texture_variant_count() -> u32 {
    let dir_path = get_project_root() + "/" + BUTTERFLY_TEXTURE_DIR_REL_PATH;
    let Ok(entries) = std::fs::read_dir(&dir_path) else {
        log::warn!(
            "Failed to read butterfly texture directory '{}'; defaulting to one variant",
            dir_path
        );
        return 1;
    };

    let count = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
        })
        .count() as u32;

    count.max(1)
}

fn discover_bird_texture_variant_count() -> u32 {
    let path = get_project_root() + "/" + BIRD_TEXTURE_FILE_REL_PATH;
    if !std::path::Path::new(&path).exists() {
        log::warn!(
            "Bird sprite texture '{}' is missing; defaulting to one variant",
            path
        );
    }
    1
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
            sink_on_lifetime: true,
            sink_speed: self.rng.random_range(0.08..=0.18),
            texture_variant: 0,
            render_kind: ParticleRenderKind::Leaf,
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
    pub butterfly_count: u32,
    pub wander_radius: f32,
    pub height_offset_min: f32,
    pub height_offset_max: f32,
    pub size: f32,
    pub drift_strength_min: f32,
    pub drift_strength_max: f32,
    pub drift_frequency_min: f32,
    pub drift_frequency_max: f32,
    pub steering_strength: f32,
    pub bob_frequency_hz: f32,
    pub bob_strength: f32,
    pub color_low: Vec4,
    pub color_high: Vec4,
}

impl Default for ButterflyEmitterDesc {
    fn default() -> Self {
        Self {
            enabled: true,
            butterfly_count: 128,
            wander_radius: 2.5,
            height_offset_min: 0.06,
            height_offset_max: 0.14,
            size: 0.018,
            drift_strength_min: 0.6,
            drift_strength_max: 1.4,
            drift_frequency_min: 1.5,
            drift_frequency_max: 3.5,
            steering_strength: 0.9,
            bob_frequency_hz: 2.2,
            bob_strength: 1.4,
            color_low: Vec4::new(0.95, 0.9, 0.55, 1.0),
            color_high: Vec4::new(1.0, 0.97, 0.72, 1.0),
        }
    }
}

pub struct ButterflyEmitter {
    pub center: Vec3,
    pub wander_radius: f32,
    min_wander_radius: f32,
    pub height_offset: RangeInclusive<f32>,
    pub size: f32,
    pub drift_strength: RangeInclusive<f32>,
    pub drift_frequency: RangeInclusive<f32>,
    pub steering_strength: f32,
    pub bob_frequency_hz: f32,
    pub bob_strength: f32,
    pub color_low: Vec4,
    pub color_high: Vec4,
    pub enabled: bool,
    pub butterfly_count: u32,
    texture_variant_count: u32,
    render_kind: ParticleRenderKind,
    rng: SmallRng,
    active_handles: Vec<ParticleHandle>,
    smoothed_ground_height: HashMap<ParticleHandle, f32>,
}

impl ButterflyEmitter {
    pub fn new(center: Vec3, extent: Vec3, seed: u64, desc: &ButterflyEmitterDesc) -> Self {
        Self::new_with_render_kind(
            center,
            extent,
            seed,
            desc,
            ParticleRenderKind::Butterfly,
            discover_butterfly_texture_variant_count(),
        )
    }

    fn new_with_render_kind(
        center: Vec3,
        extent: Vec3,
        seed: u64,
        desc: &ButterflyEmitterDesc,
        render_kind: ParticleRenderKind,
        texture_variant_count: u32,
    ) -> Self {
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
            size: desc.size.max(0.001),
            drift_strength: desc.drift_strength_min.min(desc.drift_strength_max)
                ..=desc.drift_strength_max.max(desc.drift_strength_min),
            drift_frequency: desc.drift_frequency_min.min(desc.drift_frequency_max)
                ..=desc.drift_frequency_max.max(desc.drift_frequency_min),
            steering_strength: desc.steering_strength.max(0.0),
            bob_frequency_hz: desc.bob_frequency_hz.max(0.0),
            bob_strength: desc.bob_strength.max(0.0),
            color_low: desc.color_low,
            color_high: desc.color_high,
            enabled: desc.enabled,
            butterfly_count: desc.butterfly_count,
            texture_variant_count,
            render_kind,
            rng: SmallRng::seed_from_u64(seed),
            active_handles: Vec::new(),
            smoothed_ground_height: HashMap::new(),
        };
        emitter.clamp_height(center.y);
        emitter
    }

    pub fn apply_desc(&mut self, desc: &ButterflyEmitterDesc) {
        self.enabled = desc.enabled;
        self.butterfly_count = desc.butterfly_count;
        self.wander_radius = desc.wander_radius.max(self.min_wander_radius).max(0.1);
        self.height_offset = desc.height_offset_min.min(desc.height_offset_max)
            ..=desc.height_offset_max.max(desc.height_offset_min);
        self.size = desc.size.max(0.001);
        self.drift_strength = desc.drift_strength_min.min(desc.drift_strength_max)
            ..=desc.drift_strength_max.max(desc.drift_strength_min);
        self.drift_frequency = desc.drift_frequency_min.min(desc.drift_frequency_max)
            ..=desc.drift_frequency_max.max(desc.drift_frequency_min);
        self.steering_strength = desc.steering_strength.max(0.0);
        self.bob_frequency_hz = desc.bob_frequency_hz.max(0.0);
        self.bob_strength = desc.bob_strength.max(0.0);
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
        self.smoothed_ground_height
            .retain(|handle, _| system.is_alive_handle(*handle));
    }

    fn steer_towards_home(&mut self, system: &mut ParticleSystem, dt: f32, time: f32) {
        let max_height_offset = *self.height_offset.end();
        let min_height_offset = *self.height_offset.start();
        let steering = self.steering_strength * dt;
        let vertical_span = (max_height_offset - min_height_offset).max(0.01);
        let flutter_angular_speed = TAU * self.bob_frequency_hz.max(0.0);
        let flutter_pull = self.bob_strength * dt;
        for handle in &self.active_handles {
            if let Some(pos) = system.position(*handle) {
                let relative = pos - self.center;
                let horizontal = Vec3::new(relative.x, 0.0, relative.z);
                if horizontal.length_squared() > self.wander_radius * self.wander_radius {
                    let pull = -horizontal.normalize_or_zero() * steering;
                    let _ = system.add_velocity(*handle, pull);
                }

                // Add a rapid flap-like vertical target so butterflies frequently bob up/down.
                let phase_offset = pos.x * 2.7 + pos.z * 3.3;
                let flutter = (time * flutter_angular_speed + phase_offset).sin();
                let target_offset = min_height_offset + (0.5 + 0.5 * flutter) * vertical_span;
                let y_error = target_offset - relative.y;
                let _ = system.add_velocity(*handle, Vec3::new(0.0, y_error * flutter_pull, 0.0));

                // Keep hard vertical bounds to avoid runaway drift.
                if relative.y < min_height_offset {
                    let _ = system.add_velocity(*handle, Vec3::new(0.0, steering, 0.0));
                } else if relative.y > max_height_offset {
                    let _ = system.add_velocity(*handle, Vec3::new(0.0, -steering, 0.0));
                }
            }
        }
    }

    fn enforce_size_on_active(&self, system: &mut ParticleSystem) {
        for handle in &self.active_handles {
            let _ = system.set_size(*handle, self.size);
        }
    }

    fn trim_active_to_count(&mut self, system: &mut ParticleSystem, target_count: usize) {
        while self.active_handles.len() > target_count {
            if let Some(handle) = self.active_handles.pop() {
                let _ = system.despawn(handle);
                self.smoothed_ground_height.remove(&handle);
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
            size: self.size,
            lifetime: f32::MAX,
            wind_factor: 0.0,
            gravity_factor: 0.0,
            drift_direction,
            drift_strength,
            drift_frequency,
            speed_noise_offset: self.rng.random_range(0.0..10_000.0),
            motion_mode: MotionMode::Free,
            sink_on_lifetime: false,
            sink_speed: 0.0,
            texture_variant: self.rng.random_range(0..self.texture_variant_count),
            render_kind: self.render_kind,
        };

        system.spawn(spawn)
    }

    pub fn collect_ground_queries(
        &mut self,
        system: &ParticleSystem,
        out_positions_xz: &mut Vec<Vec2>,
        out_handles: &mut Vec<ParticleHandle>,
    ) {
        self.prune_handles(system);
        for handle in &self.active_handles {
            if let Some(pos) = system.position(*handle) {
                out_positions_xz.push(Vec2::new(pos.x, pos.z));
                out_handles.push(*handle);
            }
        }
    }

    pub fn constrain_to_ground(
        &mut self,
        system: &mut ParticleSystem,
        handle: ParticleHandle,
        ground_height: f32,
        dt: f32,
    ) {
        let Some(mut pos) = system.position(handle) else {
            return;
        };

        // Temporal smoothing prevents per-frame query jitter from snapping butterflies.
        const GROUND_TRACKING_TAU_SEC: f32 = 0.22;
        const MAX_GROUND_STEP_PER_SEC: f32 = 3.5;
        const MAX_VERTICAL_CORRECTION_PER_SEC: f32 = 1.8;

        let clamped_dt = dt.max(1.0 / 240.0);
        let tracked_ground = self
            .smoothed_ground_height
            .entry(handle)
            .or_insert(ground_height);
        let max_ground_step = MAX_GROUND_STEP_PER_SEC * clamped_dt;
        let limited_measurement = *tracked_ground
            + (ground_height - *tracked_ground).clamp(-max_ground_step, max_ground_step);
        let alpha = 1.0 - (-clamped_dt / GROUND_TRACKING_TAU_SEC).exp();
        *tracked_ground += (limited_measurement - *tracked_ground) * alpha;

        let min_height = *tracked_ground + *self.height_offset.start();
        let max_height = *tracked_ground + *self.height_offset.end();
        let max_vertical_correction = MAX_VERTICAL_CORRECTION_PER_SEC * clamped_dt;

        if pos.y < min_height {
            pos.y = (pos.y + max_vertical_correction).min(min_height);
        } else if pos.y > max_height {
            pos.y = (pos.y - max_vertical_correction).max(max_height);
        }
        let _ = system.set_position(handle, pos);
    }
}

impl ParticleEmitter for ButterflyEmitter {
    fn update(&mut self, system: &mut ParticleSystem, dt: f32, time: f32) {
        self.prune_handles(system);
        let target_count = if self.enabled {
            self.butterfly_count as usize
        } else {
            0
        };
        self.trim_active_to_count(system, target_count);
        if target_count == 0 {
            return;
        }

        self.enforce_size_on_active(system);

        while self.active_handles.len() < target_count {
            if let Some(handle) = self.spawn_butterfly(system) {
                self.active_handles.push(handle);
            } else {
                break;
            }
        }

        self.steer_towards_home(system, dt, time);
    }
}

pub type BirdEmitterDesc = ButterflyEmitterDesc;

const BIRD_COUNT: usize = 50;
const BIRD_FLIGHT_SPEED: f32 = 0.52;
const BIRD_GROUND_IDLE_MIN_SEC: f32 = 4.0;
const BIRD_GROUND_IDLE_MAX_SEC: f32 = 10.0;
const BIRD_LANDING_RADIUS: f32 = 0.08;
const BIRD_GROUND_OFFSET: f32 = 0.02;
const BIRD_TARGET_PADDING: f32 = 0.02;

#[derive(Clone, Copy, Debug)]
enum BirdMode {
    Grounded {
        next_takeoff_time: f32,
    },
    AwaitingLanding {
        target_xz: Vec2,
    },
    Flying {
        current_target: Vec3,
        next_target: Option<Vec3>,
    },
}

pub struct BirdEmitter {
    center: Vec3,
    bounds_min: Vec2,
    bounds_max: Vec2,
    size: f32,
    color_low: Vec4,
    color_high: Vec4,
    enabled: bool,
    texture_variant_count: u32,
    render_kind: ParticleRenderKind,
    rng: SmallRng,
    active_handles: Vec<ParticleHandle>,
    smoothed_ground_height: HashMap<ParticleHandle, f32>,
    states: HashMap<ParticleHandle, BirdMode>,
}

impl BirdEmitter {
    pub fn new_bird(center: Vec3, extent: Vec3, seed: u64, desc: &BirdEmitterDesc) -> Self {
        let bounds_min = Vec2::new(center.x - extent.x, center.z - extent.z);
        let bounds_max = Vec2::new(center.x + extent.x, center.z + extent.z);
        let (bounds_min, bounds_max) = Self::normalize_bounds(bounds_min, bounds_max);
        Self {
            center,
            bounds_min,
            bounds_max,
            size: desc.size.max(0.001),
            color_low: desc.color_low,
            color_high: desc.color_high,
            enabled: desc.enabled,
            texture_variant_count: discover_bird_texture_variant_count(),
            render_kind: ParticleRenderKind::Bird,
            rng: SmallRng::seed_from_u64(seed),
            active_handles: Vec::new(),
            smoothed_ground_height: HashMap::new(),
            states: HashMap::new(),
        }
    }

    pub fn apply_desc(&mut self, desc: &BirdEmitterDesc) {
        self.enabled = desc.enabled;
        self.size = desc.size.max(0.001);
        self.color_low = desc.color_low;
        self.color_high = desc.color_high;
    }

    fn normalize_bounds(min: Vec2, max: Vec2) -> (Vec2, Vec2) {
        let min_x = min.x.min(max.x);
        let max_x = min.x.max(max.x);
        let min_z = min.y.min(max.y);
        let max_z = min.y.max(max.y);
        (Vec2::new(min_x, min_z), Vec2::new(max_x, max_z))
    }

    fn prune_handles(&mut self, system: &ParticleSystem) {
        self.active_handles
            .retain(|handle| system.is_alive_handle(*handle));
        self.smoothed_ground_height
            .retain(|handle, _| system.is_alive_handle(*handle));
        self.states
            .retain(|handle, _| system.is_alive_handle(*handle));
    }

    fn enforce_size_on_active(&self, system: &mut ParticleSystem) {
        for handle in &self.active_handles {
            let _ = system.set_size(*handle, self.size);
        }
    }

    fn trim_active_to_count(&mut self, system: &mut ParticleSystem, target_count: usize) {
        while self.active_handles.len() > target_count {
            if let Some(handle) = self.active_handles.pop() {
                let _ = system.despawn(handle);
                self.smoothed_ground_height.remove(&handle);
                self.states.remove(&handle);
            }
        }
    }

    fn next_takeoff_time(&mut self, time: f32) -> f32 {
        let delay = self
            .rng
            .random_range(BIRD_GROUND_IDLE_MIN_SEC..=BIRD_GROUND_IDLE_MAX_SEC);
        time + delay
    }

    fn sample_target_xz(&mut self, origin: Vec2) -> Vec2 {
        let min_x = (self.bounds_min.x + BIRD_TARGET_PADDING).min(self.bounds_max.x);
        let max_x = (self.bounds_max.x - BIRD_TARGET_PADDING).max(min_x);
        let min_z = (self.bounds_min.y + BIRD_TARGET_PADDING).min(self.bounds_max.y);
        let max_z = (self.bounds_max.y - BIRD_TARGET_PADDING).max(min_z);

        let mut target = Vec2::new(
            self.rng.random_range(min_x..=max_x),
            self.rng.random_range(min_z..=max_z),
        );

        let min_distance = 0.6;
        if (target - origin).length_squared() < min_distance * min_distance {
            let angle = self.rng.random_range(0.0..TAU);
            let radius = self.rng.random_range(min_distance..=min_distance * 1.6);
            let offset = Vec2::new(angle.cos(), angle.sin()) * radius;
            target = origin + offset;
            target.x = target.x.clamp(min_x, max_x);
            target.y = target.y.clamp(min_z, max_z);
        }

        target
    }

    fn spawn_bird(&mut self, system: &mut ParticleSystem) -> Option<ParticleHandle> {
        let spawn_xz = self.sample_target_xz(Vec2::new(self.center.x, self.center.z));
        let position = Vec3::new(spawn_xz.x, self.center.y, spawn_xz.y);

        let spawn = ParticleSpawn {
            position,
            velocity: Vec3::ZERO,
            color: random_color(&mut self.rng, self.color_low, self.color_high),
            size: self.size,
            lifetime: f32::MAX,
            wind_factor: 0.0,
            gravity_factor: 0.0,
            drift_direction: Vec3::ZERO,
            drift_strength: 0.0,
            drift_frequency: 1.0,
            speed_noise_offset: self.rng.random_range(0.0..10_000.0),
            motion_mode: MotionMode::Free,
            sink_on_lifetime: false,
            sink_speed: 0.0,
            texture_variant: self.rng.random_range(0..self.texture_variant_count),
            render_kind: self.render_kind,
        };

        system.spawn(spawn)
    }

    fn ensure_state(&mut self, handle: ParticleHandle, time: f32) {
        if !self.states.contains_key(&handle) {
            let next_time = self.next_takeoff_time(time);
            self.states.insert(
                handle,
                BirdMode::Grounded {
                    next_takeoff_time: next_time,
                },
            );
        }
    }

    fn clamp_inside_bounds(&mut self, system: &mut ParticleSystem, handle: ParticleHandle) -> bool {
        let Some(mut pos) = system.position(handle) else {
            return false;
        };

        let mut clamped = false;
        if pos.x < self.bounds_min.x {
            pos.x = self.bounds_min.x;
            clamped = true;
        } else if pos.x > self.bounds_max.x {
            pos.x = self.bounds_max.x;
            clamped = true;
        }

        if pos.z < self.bounds_min.y {
            pos.z = self.bounds_min.y;
            clamped = true;
        } else if pos.z > self.bounds_max.y {
            pos.z = self.bounds_max.y;
            clamped = true;
        }

        if clamped {
            let _ = system.set_position(handle, pos);
        }
        clamped
    }

    pub fn collect_ground_queries(
        &mut self,
        system: &ParticleSystem,
        out_positions_xz: &mut Vec<Vec2>,
        out_handles: &mut Vec<ParticleHandle>,
    ) {
        self.prune_handles(system);
        for handle in &self.active_handles {
            if let Some(mode) = self.states.get(handle) {
                if matches!(
                    mode,
                    BirdMode::Grounded { .. } | BirdMode::AwaitingLanding { .. }
                ) {
                    if let Some(pos) = system.position(*handle) {
                        out_positions_xz.push(Vec2::new(pos.x, pos.z));
                        out_handles.push(*handle);
                    }
                }
            }
        }
    }

    pub fn collect_landing_queries(
        &mut self,
        out_positions_xz: &mut Vec<Vec2>,
        out_handles: &mut Vec<ParticleHandle>,
    ) {
        for (handle, mode) in self.states.iter() {
            if let BirdMode::AwaitingLanding { target_xz } = mode {
                out_positions_xz.push(*target_xz);
                out_handles.push(*handle);
            }
        }
    }

    pub fn resample_landing_target(&mut self, system: &ParticleSystem, handle: ParticleHandle) {
        let Some(pos) = system.position(handle) else {
            return;
        };
        let origin = Vec2::new(pos.x, pos.z);
        let target_xz = self.sample_target_xz(origin);
        self.states
            .insert(handle, BirdMode::AwaitingLanding { target_xz });
    }

    pub fn apply_landing_height(
        &mut self,
        system: &ParticleSystem,
        handle: ParticleHandle,
        ground_height: f32,
        waypoint: Vec3,
    ) {
        let Some(pos) = system.position(handle) else {
            return;
        };

        let target = Vec3::new(pos.x, ground_height + BIRD_GROUND_OFFSET, pos.z);
        if let Some(BirdMode::AwaitingLanding { target_xz }) = self.states.get(&handle) {
            let landing = Vec3::new(target_xz.x, ground_height + BIRD_GROUND_OFFSET, target_xz.y);
            let path_start = if (waypoint - landing).length_squared() <= 1.0e-4 {
                landing
            } else {
                waypoint
            };
            let next_target = if (path_start - landing).length_squared() <= 1.0e-4 {
                None
            } else {
                Some(landing)
            };
            self.states.insert(
                handle,
                BirdMode::Flying {
                    current_target: path_start,
                    next_target,
                },
            );
        } else {
            let path_start = if (waypoint - target).length_squared() <= 1.0e-4 {
                target
            } else {
                waypoint
            };
            let next_target = if (path_start - target).length_squared() <= 1.0e-4 {
                None
            } else {
                Some(target)
            };
            self.states.insert(
                handle,
                BirdMode::Flying {
                    current_target: path_start,
                    next_target,
                },
            );
        }
    }

    pub fn apply_ground_height(
        &mut self,
        system: &mut ParticleSystem,
        handle: ParticleHandle,
        ground_height: f32,
        dt: f32,
    ) {
        let Some(mut pos) = system.position(handle) else {
            return;
        };

        const GROUND_TRACKING_TAU_SEC: f32 = 0.18;
        const MAX_GROUND_STEP_PER_SEC: f32 = 4.5;

        let clamped_dt = dt.max(1.0 / 240.0);
        let tracked_ground = self
            .smoothed_ground_height
            .entry(handle)
            .or_insert(ground_height);
        let max_ground_step = MAX_GROUND_STEP_PER_SEC * clamped_dt;
        let limited_measurement = *tracked_ground
            + (ground_height - *tracked_ground).clamp(-max_ground_step, max_ground_step);
        let alpha = 1.0 - (-clamped_dt / GROUND_TRACKING_TAU_SEC).exp();
        *tracked_ground += (limited_measurement - *tracked_ground) * alpha;

        pos.y = *tracked_ground + BIRD_GROUND_OFFSET;
        let _ = system.set_position(handle, pos);
    }
}

impl ParticleEmitter for BirdEmitter {
    fn update(&mut self, system: &mut ParticleSystem, dt: f32, time: f32) {
        self.prune_handles(system);
        let target_count = if self.enabled { BIRD_COUNT } else { 0 };
        self.trim_active_to_count(system, target_count);
        if target_count == 0 {
            return;
        }

        self.enforce_size_on_active(system);

        while self.active_handles.len() < target_count {
            if let Some(handle) = self.spawn_bird(system) {
                self.active_handles.push(handle);
                let next_time = self.next_takeoff_time(time);
                self.states.insert(
                    handle,
                    BirdMode::Grounded {
                        next_takeoff_time: next_time,
                    },
                );
            } else {
                break;
            }
        }

        let active_handles = self.active_handles.clone();
        for handle in active_handles {
            self.ensure_state(handle, time);

            if self.clamp_inside_bounds(system, handle) {
                let _ = system.set_velocity(handle, Vec3::ZERO);
                let next_time = self.next_takeoff_time(time);
                self.states.insert(
                    handle,
                    BirdMode::Grounded {
                        next_takeoff_time: next_time,
                    },
                );
                continue;
            }

            let Some(pos) = system.position(handle) else {
                continue;
            };

            match self.states.get(&handle).copied() {
                Some(BirdMode::Grounded { next_takeoff_time }) => {
                    let _ = system.set_velocity(handle, Vec3::ZERO);
                    if time >= next_takeoff_time {
                        let origin = Vec2::new(pos.x, pos.z);
                        let target_xz = self.sample_target_xz(origin);
                        self.states
                            .insert(handle, BirdMode::AwaitingLanding { target_xz });
                    }
                }
                Some(BirdMode::AwaitingLanding { .. }) => {
                    let _ = system.set_velocity(handle, Vec3::ZERO);
                }
                Some(BirdMode::Flying {
                    current_target,
                    next_target,
                }) => {
                    let to_target = current_target - pos;
                    let distance = to_target.length();
                    if distance <= BIRD_LANDING_RADIUS || distance <= BIRD_FLIGHT_SPEED * dt {
                        let _ = system.set_position(handle, current_target);
                        if let Some(next) = next_target {
                            self.states.insert(
                                handle,
                                BirdMode::Flying {
                                    current_target: next,
                                    next_target: None,
                                },
                            );
                        } else {
                            let _ = system.set_velocity(handle, Vec3::ZERO);
                            let next_time = self.next_takeoff_time(time);
                            self.states.insert(
                                handle,
                                BirdMode::Grounded {
                                    next_takeoff_time: next_time,
                                },
                            );
                        }
                    } else {
                        let desired = to_target / distance * BIRD_FLIGHT_SPEED;
                        let _ = system.set_velocity(handle, desired);
                    }
                }
                None => {}
            }
        }
    }
}
