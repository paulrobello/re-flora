# PLAN: Camera Head Bob (Walk Mode)

## Goal

Add a hybrid head bob (vertical position bob + horizontal sway + subtle roll) to the first-person camera during walk mode. The bob phase is **driven by the existing footstep audio timer** so visual bob and footstep sounds are always in sync. Amplitudes are adjustable via the GUI debug panel.

## Design Decisions

1. **Phase source**: Reuse `PlayerAudioController.time_since_last_step` and `walk_interval` / `run_interval` as the bob clock. No independent oscillator -- zero drift from audio.
2. **View-only offset**: Bob modifies the view matrix, never `Camera.position`. Physics, collision, and audio listener position remain unaffected.
3. **Walk mode only**: Fly mode stays smooth -- no bob applied.
4. **Sprint scaling**: Sprint uses a larger amplitude multiplier. Frequency difference is automatic since the run interval (0.25s) is shorter than walk interval (0.35s).
5. **Smooth transitions**: A lerp-based blend factor ramps the bob in/out when starting/stopping movement or leaving the ground, preventing jarring snaps.

## Files Changed

| File                                | Change                                                                                   |
| ----------------------------------- | ---------------------------------------------------------------------------------------- |
| `src/gameplay/camera/head_bob.rs`   | **New.** `HeadBob` struct + `update()` + `apply_to_view_mat()`                           |
| `src/gameplay/camera/mod.rs`        | Register `head_bob` module                                                               |
| `src/gameplay/camera/controller.rs` | Add `HeadBob` field to `Camera`, call `update()` in walk mode, apply in `get_view_mat()` |
| `src/gameplay/camera/audio.rs`      | Expose `step_phase()` method (read-only)                                                 |
| `src/gameplay/camera/desc.rs`       | Add `CameraHeadBobDesc` with defaults, include in `CameraDesc`                           |
| `config/gui.toml`                   | Add `HeadBob` section with 4 float params                                                |
| `src/app/core/mod.rs`               | Pass GUI headbob values to `tracer.set_head_bob_params(...)` each frame                  |
| `src/tracer/mod.rs`                 | Add `set_head_bob_params()` that forwards to `camera.set_head_bob_params()`              |

## Step-by-step

### Step 1 - `CameraHeadBobDesc` in `desc.rs`

Add a new config struct with sensible defaults:

```rust
pub struct CameraHeadBobDesc {
    pub vertical_amplitude: f32,    // 0.003 world units
    pub horizontal_amplitude: f32,  // 0.0015 world units
    pub roll_amplitude_deg: f32,    // 0.3 degrees
    pub sprint_amplitude_mul: f32,  // 1.5x
    pub smoothing_speed: f32,       // 8.0 (lerp speed for blend factor)
}
```

Add `pub head_bob: CameraHeadBobDesc` to `CameraDesc`.

### Step 2 - Expose step phase from `audio.rs`

Add a read-only method to `PlayerAudioController`:

```rust
/// returns the step cycle progress in [0, 1), synced with footstep audio timer.
/// returns 0.0 when not walking.
pub fn step_phase(&self, is_running: bool) -> f32 {
    let interval = if is_running {
        self.clip_caches.run_interval
    } else {
        self.clip_caches.walk_interval
    };
    (self.time_since_last_step / interval).clamp(0.0, 1.0)
}
```

This is purely additive -- no behavior change to the existing audio system.

### Step 3 - New `head_bob.rs`

The `HeadBob` struct holds computed offsets and a smooth blend factor:

```rust
pub struct HeadBob {
    blend: f32,       // 0.0 = inactive, 1.0 = full bob
    pub offset_y: f32,
    pub offset_x: f32,
    pub roll_rad: f32,
}
```

Core method `update()`:

```rust
pub fn update(
    &mut self,
    step_phase: f32,        // 0..1 from audio timer
    is_active: bool,        // grounded AND moving
    is_running: bool,
    desc: &CameraHeadBobDesc,
    dt: f32,
) {
    // smooth blend toward target (1 when active, 0 when inactive)
    let target = if is_active { 1.0 } else { 0.0 };
    self.blend = lerp(self.blend, target, (desc.smoothing_speed * dt).clamp(0.0, 1.0));

    let amp_mul = if is_running { desc.sprint_amplitude_mul } else { 1.0 };
    let phase_rad = step_phase * TAU;  // full bob cycle = one step interval

    // vertical: sin wave, one full cycle per step
    self.offset_y = (phase_rad.sin()) * desc.vertical_amplitude * amp_mul * self.blend;

    // horizontal sway: cos at half frequency (one sway cycle = two steps)
    self.offset_x = (phase_rad.cos()) * desc.horizontal_amplitude * amp_mul * self.blend;

    // roll: synced with horizontal sway
    let roll_amp_rad = desc.roll_amplitude_deg.to_radians();
    self.roll_rad = (phase_rad.cos()) * roll_amp_rad * amp_mul * self.blend;
}
```

Method `apply_to_view_mat()`:

```rust
pub fn apply_to_view_mat(&self, view_mat: Mat4, right: Vec3, up: Vec3) -> Mat4 {
    let bob_translation = Mat4::from_translation(right * self.offset_x + up * self.offset_y);
    let roll_rotation = Mat4::from_rotation_z(self.roll_rad);
    roll_rotation * view_mat * bob_translation.inverse()
}
```

### Step 4 - Wire into `Camera` in `controller.rs`

- Add `head_bob: HeadBob` field to `Camera` struct.
- In `Camera::new()`, initialize `HeadBob::new()`.
- At the end of `update_transform_walk_mode()`, after the audio update block, add:

```rust
let step_phase = self.player_audio_controller.step_phase(is_running);
self.head_bob.update(
    step_phase,
    is_on_ground && is_moving,
    is_running,
    &self.desc.head_bob,
    frame_delta_time,
);
```

- Modify `get_view_mat()` to apply the bob:

```rust
pub fn get_view_mat(&self) -> Mat4 {
    let base_view = Mat4::look_at_rh(
        self.position,
        self.position + self.vectors.front,
        self.vectors.up,
    );
    self.head_bob.apply_to_view_mat(base_view, self.vectors.right, self.vectors.up)
}
```

When the bob blend is 0, all offsets are 0 and `apply_to_view_mat` returns the unmodified view matrix.

### Step 5 - Add runtime GUI override via `set_head_bob_params()`

Add a method to `Camera`:

```rust
pub fn set_head_bob_params(
    &mut self,
    vertical_amp: f32,
    horizontal_amp: f32,
    roll_amp_deg: f32,
    sprint_amp_mul: f32,
) {
    self.desc.head_bob.vertical_amplitude = vertical_amp;
    self.desc.head_bob.horizontal_amplitude = horizontal_amp;
    self.desc.head_bob.roll_amplitude_deg = roll_amp_deg;
    self.desc.head_bob.sprint_amplitude_mul = sprint_amp_mul;
}
```

Expose through `Tracer`:

```rust
// in src/tracer/mod.rs
pub fn set_head_bob_params(&mut self, v: f32, h: f32, r: f32, s: f32) {
    self.camera.set_head_bob_params(v, h, r, s);
}
```

### Step 6 - GUI params in `config/gui.toml`

Append a new `HeadBob` section:

```toml
[[section]]
name = "HeadBob"

[[section.param]]
id = "headbob_vertical_amp"
kind = "float"
label = "Vertical Amplitude"
type = "Float"
[section.param.data]
value = 0.003
min = 0.0
max = 0.02

[[section.param]]
id = "headbob_horizontal_amp"
kind = "float"
label = "Horizontal Amplitude"
type = "Float"
[section.param.data]
value = 0.0015
min = 0.0
max = 0.01

[[section.param]]
id = "headbob_roll_amp"
kind = "float"
label = "Roll Amplitude (deg)"
type = "Float"
[section.param.data]
value = 0.3
min = 0.0
max = 2.0

[[section.param]]
id = "headbob_sprint_amp_mul"
kind = "float"
label = "Sprint Amplitude Mul"
type = "Float"
[section.param.data]
value = 1.5
min = 1.0
max = 3.0
```

These auto-generate into `GuiAdjustables` via the existing `build.rs` pipeline.

### Step 7 - Pass GUI values each frame in `src/app/core/mod.rs`

Before the `update_camera` call (around line 1170), add:

```rust
self.tracer.set_head_bob_params(
    self.gui_adjustables.headbob_vertical_amp.value,
    self.gui_adjustables.headbob_horizontal_amp.value,
    self.gui_adjustables.headbob_roll_amp.value,
    self.gui_adjustables.headbob_sprint_amp_mul.value,
);
```

## Horizontal sway alternation

The audio timer `time_since_last_step` resets to 0 every step, so `step_phase` always goes 0->1 in the same direction. For proper left-right alternation (sway left on odd steps, right on even), add a `step_parity: bool` field to `HeadBob`, toggled each time `step_phase` wraps. Multiply `offset_x` and `roll_rad` by `-1` on odd steps. This gives natural alternating lateral sway without any changes to the audio system.

## Scope

- ~80 lines new code in `head_bob.rs`
- ~20 lines changes across `controller.rs`, `desc.rs`, `mod.rs`, `audio.rs`
- ~5 lines in `tracer/mod.rs`
- ~5 lines in `app/core/mod.rs`
- ~30 lines in `config/gui.toml`
- No shader changes, no GPU pipeline changes
