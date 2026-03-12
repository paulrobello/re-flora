#[derive(Clone, Copy, Debug)]
pub struct AnimatedTextureSequence {
    pub start_frame: u32,
    pub frame_count: u32,
}

impl AnimatedTextureSequence {
    pub const fn new(start_frame: u32, frame_count: u32) -> Self {
        Self {
            start_frame,
            frame_count,
        }
    }
}

pub const PARTICLE_SPRITE_FRAME_DIM: u32 = 16;
pub const BUTTERFLY_ANIM_FRAME_DURATION_SEC: f32 = 0.2;
pub const BUTTERFLY_FRAMES_PER_VARIANT: u32 = 5;
pub const BUTTERFLY_VIEW_COUNT: u32 = 5;
pub const BUTTERFLY_VIEW_BUCKET_HALF_WIDTH: f32 = 22.5_f32.to_radians();
