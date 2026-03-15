#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct AnimatedTextureSequence {
    pub start_frame: u32,
    pub frame_count: u32,
}

impl AnimatedTextureSequence {
    #[allow(dead_code)]
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
// number of logical butterfly views used at runtime
pub const BUTTERFLY_VIEW_COUNT: u32 = 2;

// mapping from logical view index to physical atlas row (0-based).
// view 0: heading left-up 45deg (second row in the sprite sheet)
// view 1: heading left-down 45deg (fourth row in the sprite sheet)
pub const BUTTERFLY_ATLAS_ROW_FOR_VIEW: [u32; BUTTERFLY_VIEW_COUNT as usize] = [1, 3];
