# Butterfly Multi-View Atlas Implementation Plan

## Overview

Add 5 camera-relative views to the butterfly sprite atlas, allowing butterflies to face the player correctly based on their flight direction.

**Atlas format**: single grayscale PNG, 80×80 (5 rows × 5 columns)

- row 0: front (0°)
- row 1: front-side (45°)
- row 2: side (90°)
- row 3: back-side (135°)
- row 4: back (180°)

**Coloring**: shader-driven per-instance tint (no longer atlas-driven)

## Required Resource Dependency

Implementation depends on one required image resource:

- required file: `assets/texture/butterfly_16px/butterfly_gray_5x5.png`
- required format: PNG, RGBA
- required size: `80x80`
- required layout: `5` rows (view angles) × `5` columns (animation frames)
- required pixel rules: opaque butterfly pixels with alpha `255`, transparent background with alpha `0`

Startup behavior:

- if the required file is missing, fail fast with a clear error
- if dimensions are not `80x80`, fail fast with a clear error
- if more than one butterfly atlas PNG is present in the folder, fail fast (single-atlas contract)

## Inputs for View Selection

For each butterfly instance, use:

1. butterfly velocity direction (world space)
2. butterfly world position
3. player camera world position

Facing rule: butterfly faces where it flies (velocity direction).

---

## Implementation

### 1. Loader (`src/tracer/resources.rs`)

Update `create_particle_lod_tex_lut` to load exactly one butterfly atlas and extract all 5 rows.

```rust
// discover all butterfly png files
let atlas_paths = ...; // *.png under assets/texture/butterfly_16px
assert!(
    atlas_paths.len() == 1,
    "Expected exactly one butterfly atlas png, found {}",
    atlas_paths.len()
);

let atlas = image::open(&atlas_paths[0])?.to_rgba8();
let (width, height) = atlas.dimensions();
let frame_dim = PARTICLE_SPRITE_FRAME_DIM; // 16

assert!(
    width == frame_dim * 5 && height == frame_dim * 5,
    "Butterfly atlas must be {}x{}, got {}x{}",
    frame_dim * 5,
    frame_dim * 5,
    width,
    height,
);
```

For each row `0..5`, call `extract_row_sequence_layers(&atlas, row, 5, source_label)` and extend `butterfly_layers`.

No fallback path is required.

### 2. Emitter (`src/particles/emitters.rs`)

Since only one atlas, butterfly spawns always use `texture_variant = 0`:

```rust
// In spawn_butterfly:
texture_variant: 0, // single grayscale atlas
```

Simplify or remove variant discovery logic for butterflies.

Recommended:

- remove butterfly texture variant directory scanning
- set butterfly emitter variant count to `1`

### 3. Constants (`src/particles/animation.rs`)

```rust
pub const BUTTERFLY_VIEW_COUNT: u32 = 5;
pub const BUTTERFLY_VIEW_BUCKET_HALF_WIDTH: f32 = 22.5_f32.to_radians();
```

### 4. Runtime View Selection (`src/tracer/mod.rs`)

In `upload_particles`, replace the existing butterfly texture index logic:

```rust
// Add after computing camera vectors
let butterfly_view_count = 5;

// For each butterfly particle:
let vel_xz = Vec2::new(snap.velocity.x, snap.velocity.z);

// use world-space butterfly position (f32), not quantized UVec3 render position
let butterfly_pos_ws = snap.position_ws;
let to_cam_xz = Vec2::new(
    self.camera.position().x - butterfly_pos_ws.x,
    self.camera.position().z - butterfly_pos_ws.z,
);

// Normalize with epsilon
const MIN_SPEED_SQ: f32 = 0.01 * 0.01;
let vel_dir_xz = if vel_xz.length_squared() > MIN_SPEED_SQ {
    vel_xz.normalize()
} else {
    Vec2::ZERO // side view default
};

let to_cam_dir_xz = if to_cam_xz.length_squared() > MIN_SPEED_SQ {
    to_cam_xz.normalize()
} else {
    Vec2::ZERO // side view default
};

// Compute angle between velocity direction and direction to camera
let view_index = if vel_dir_xz == Vec2::ZERO || to_cam_dir_xz == Vec2::ZERO {
    2 // side view (index 2)
} else {
    let dot = (-vel_dir_xz).dot(to_cam_dir_xz).clamp(-1.0, 1.0);
    let angle = dot.acos();

    // Bucket to 5 views: 0=front, 1=front-side, 2=side, 3=back-side, 4=back
    let half_width = BUTTERFLY_VIEW_BUCKET_HALF_WIDTH;
    if angle < half_width {
        0
    } else if angle < half_width * 3.0 {
        1
    } else if angle < half_width * 5.0 {
        2
    } else if angle < half_width * 7.0 {
        3
    } else {
        4
    }
};

// Flip based on camera right vector
let camera_right_xz = Vec2::new(self.camera.vectors().right.x, self.camera.vectors().right.z);
let flip_x = if vel_dir_xz == Vec2::ZERO {
    false
} else {
    vel_dir_xz.dot(camera_right_xz) > 0.0 // if mirrored visually, invert this sign
};

// Compute final texture index
let butterfly_frame_count = BUTTERFLY_FRAMES_PER_VARIANT;
let butterfly_tex_index = animated_variant_layer(
    LEAF_LAYER_COUNT,
    view_index,                    // use view index as variant
    butterfly_view_count,          // 5 views
    butterfly_frame_count,
    BUTTERFLY_ANIM_FRAME_DURATION_SEC,
    time_since_start_sec,
);

let packed_index = pack_particle_tex_index(butterfly_tex_index, flip_x);
```

Apply this only for `ParticleRenderKind::Butterfly`; leaf and bird unchanged.

To support `position_ws` in the snippet above:

- add `position_ws: Vec3` to `ParticleSnapshot`
- fill it from `self.positions[*slot]` in `ParticleSystem::write_snapshots`
- keep existing quantized `position: UVec3` for GPU instance packing

---

## Testing Checklist

- [ ] Atlas loads without warnings (80×80, 5 rows)
- [ ] Butterfly flying toward camera → front view
- [ ] Butterfly flying away → back view
- [ ] Butterfly flying perpendicular → side view
- [ ] Diagonal angles → front-side / back-side
- [ ] Left/right flip works via sprite flip bit
- [ ] Near-zero velocity stable (defaults to side)
- [ ] Bird/leaf rendering unchanged
