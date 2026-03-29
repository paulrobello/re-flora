use super::CameraHeadBobDesc;
use glam::{Mat4, Vec3};
use std::f32::consts::TAU;

pub struct HeadBob {
    blend: f32,
    prev_step_phase: f32,
    step_parity: bool,
    pub offset_y: f32,
    pub offset_x: f32,
    pub roll_rad: f32,
}

impl HeadBob {
    pub fn new() -> Self {
        Self {
            blend: 0.0,
            prev_step_phase: 0.0,
            step_parity: false,
            offset_y: 0.0,
            offset_x: 0.0,
            roll_rad: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.blend = 0.0;
        self.prev_step_phase = 0.0;
        self.step_parity = false;
        self.offset_y = 0.0;
        self.offset_x = 0.0;
        self.roll_rad = 0.0;
    }

    pub fn update(
        &mut self,
        step_phase: f32,
        is_active: bool,
        is_running: bool,
        desc: &CameraHeadBobDesc,
        dt: f32,
    ) {
        let target = if is_active { 1.0 } else { 0.0 };
        let t = (desc.smoothing_speed * dt).clamp(0.0, 1.0);
        self.blend += (target - self.blend) * t;

        if is_active && step_phase < self.prev_step_phase {
            self.step_parity = !self.step_parity;
        }
        self.prev_step_phase = step_phase;

        let amp_mul = if is_running {
            desc.sprint_amplitude_mul
        } else {
            1.0
        };
        let phase_rad = step_phase * TAU;

        self.offset_y = phase_rad.sin() * desc.vertical_amplitude * amp_mul * self.blend;

        let lateral_sign = if self.step_parity { -1.0 } else { 1.0 };
        self.offset_x =
            phase_rad.cos() * desc.horizontal_amplitude * amp_mul * self.blend * lateral_sign;

        let roll_amp_rad = desc.roll_amplitude_deg.to_radians();
        self.roll_rad = phase_rad.cos() * roll_amp_rad * amp_mul * self.blend * lateral_sign;
    }

    pub fn apply_to_view_mat(&self, view_mat: Mat4, right: Vec3, up: Vec3) -> Mat4 {
        if self.offset_x.abs() <= f32::EPSILON
            && self.offset_y.abs() <= f32::EPSILON
            && self.roll_rad.abs() <= f32::EPSILON
        {
            return view_mat;
        }

        let bob_translation = Mat4::from_translation(right * self.offset_x + up * self.offset_y);
        let roll_rotation = Mat4::from_rotation_z(self.roll_rad);
        roll_rotation * view_mat * bob_translation.inverse()
    }
}
