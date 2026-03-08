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
pub const BIRD_SPRITESHEET_WIDTH: u32 = 128;
pub const BIRD_SPRITESHEET_HEIGHT: u32 = 64;

pub const BIRD_SPRITESHEET_REL_PATH: &str = "assets/texture/Bird/Spritesheet/Bird Spritesheet.png";

pub const BUTTERFLY_ANIM_FRAME_DURATION_SEC: f32 = 0.2;
pub const BUTTERFLY_FRAMES_PER_VARIANT: u32 = 5;
pub const BUTTERFLY_VIEW_COUNT: u32 = 5;
pub const BUTTERFLY_VIEW_BUCKET_HALF_WIDTH: f32 = 22.5_f32.to_radians();

pub const BIRD_ANIM_FRAME_DURATION_SEC: f32 = 0.2;
pub const BIRD_IDLE_FRAME_COUNT: u32 = 2;
pub const BIRD_FLYING_FRAME_COUNT: u32 = 7;
pub const BIRD_WALKING_FRAME_COUNT: u32 = 3;
pub const BIRD_EATING_FRAME_COUNT: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum BirdSpriteSequence {
    Idle = 0,
    Flying = 1,
    Walking = 2,
    Eating = 3,
}

impl BirdSpriteSequence {
    pub const fn texture_variant(self) -> u32 {
        self as u32
    }

    pub const fn from_texture_variant(texture_variant: u32) -> Self {
        match texture_variant {
            0 => Self::Idle,
            1 => Self::Flying,
            2 => Self::Walking,
            3 => Self::Eating,
            _ => Self::Idle,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BirdSpritesheetSequenceDef {
    pub row: u32,
    pub frame_count: u32,
}

const BIRD_SEQUENCE_DEFS: [BirdSpritesheetSequenceDef; 4] = [
    BirdSpritesheetSequenceDef {
        row: 0,
        frame_count: BIRD_IDLE_FRAME_COUNT,
    },
    BirdSpritesheetSequenceDef {
        row: 1,
        frame_count: BIRD_FLYING_FRAME_COUNT,
    },
    BirdSpritesheetSequenceDef {
        row: 2,
        frame_count: BIRD_WALKING_FRAME_COUNT,
    },
    BirdSpritesheetSequenceDef {
        row: 3,
        frame_count: BIRD_EATING_FRAME_COUNT,
    },
];

pub const BIRD_TOTAL_FRAME_COUNT: u32 = BIRD_IDLE_FRAME_COUNT
    + BIRD_FLYING_FRAME_COUNT
    + BIRD_WALKING_FRAME_COUNT
    + BIRD_EATING_FRAME_COUNT;

pub const BIRD_SPRITESHEET_SEQUENCE_ORDER: [BirdSpriteSequence; 4] = [
    BirdSpriteSequence::Idle,
    BirdSpriteSequence::Flying,
    BirdSpriteSequence::Walking,
    BirdSpriteSequence::Eating,
];

pub const fn bird_spritesheet_sequence_def(
    sequence: BirdSpriteSequence,
) -> BirdSpritesheetSequenceDef {
    BIRD_SEQUENCE_DEFS[sequence as usize]
}

pub const fn bird_animation_sequence(sequence: BirdSpriteSequence) -> AnimatedTextureSequence {
    match sequence {
        BirdSpriteSequence::Idle => AnimatedTextureSequence::new(0, BIRD_IDLE_FRAME_COUNT),
        BirdSpriteSequence::Flying => {
            AnimatedTextureSequence::new(BIRD_IDLE_FRAME_COUNT, BIRD_FLYING_FRAME_COUNT)
        }
        BirdSpriteSequence::Walking => AnimatedTextureSequence::new(
            BIRD_IDLE_FRAME_COUNT + BIRD_FLYING_FRAME_COUNT,
            BIRD_WALKING_FRAME_COUNT,
        ),
        BirdSpriteSequence::Eating => AnimatedTextureSequence::new(
            BIRD_IDLE_FRAME_COUNT + BIRD_FLYING_FRAME_COUNT + BIRD_WALKING_FRAME_COUNT,
            BIRD_EATING_FRAME_COUNT,
        ),
    }
}
