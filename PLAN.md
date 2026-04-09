# CLI Flags Reactivation Plan

## Context

- Reference commit: `0f991230b27d2689674306d89b92d91b128c0e01` (revert).
- Reverted commit noted inside it: `8c32c272f4b09749fc7c7c7ab8cb9cbf8d05257f`.
- Current branch has `AppOptions` parsing restored, but only `--windowed` is consumed in runtime logic.
- Requirement for follow-up work: remove `-w` alias and keep only `--windowed` for explicitness.

## Flag Status Matrix

| Flag                       | Parsed now | Consumed now                     | Previous implementation signal (from reference commit)                       | Status        |
| -------------------------- | ---------- | -------------------------------- | ---------------------------------------------------------------------------- | ------------- |
| `--windowed`               | yes        | yes (`src/app/core/boot.rs`)     | removed in `0f991230` then re-added later                                    | active        |
| `-w`                       | yes        | yes (same field as `--windowed`) | existed in `8c32c272` parser                                                 | should remove |
| `--no-shadows`             | yes        | no                               | `RenderFlags.enable_shadows` gated tracer shadow passes                      | unimplemented |
| `--no-denoise`             | yes        | no                               | `RenderFlags.enable_denoiser` gated denoiser + prev-frame copy               | unimplemented |
| `--no-god-rays`            | yes        | no                               | `RenderFlags.enable_god_rays` gated god-ray pass                             | unimplemented |
| `--no-lens-flare`          | yes        | no                               | `RenderFlags.enable_lens_flare` gated lens flare passes and clear            | unimplemented |
| `--no-tracer`              | yes        | no                               | `RenderFlags.enable_tracer` gated tracer compute pass                        | unimplemented |
| `--no-particles`           | yes        | no                               | `RenderFlags.enable_particles` gated particle CPU/GPU and graphics draw path | unimplemented |
| `--no-flora`               | yes        | no                               | `RenderFlags.enable_flora` gated flora/leaves graphics passes                | unimplemented |
| `--screenshot <path>`      | yes        | no                               | app had screenshot timer + `save_screenshot()`                               | unimplemented |
| `--screenshot-delay <sec>` | yes        | no                               | used with screenshot timer and default `5.0`                                 | unimplemented |
| `--auto-exit <sec>`        | yes        | no                               | app had render-start timer and exit trigger                                  | unimplemented |
| `--perf`                   | yes        | no                               | app had `perf_logging` and detailed per-frame timings                        | unimplemented |
| `--help`                   | no         | no                               | no explicit help output path in current parser                               | unimplemented |

## Diagnosis Notes

- The current state is partially restored wiring: parse -> `AppController` -> `App::new(...)`.
- Functional consumption exists only for `windowed` in window creation.
- The pass-disabling flags were previously implemented through a `RenderFlags` bridge in `src/main.rs`, consumed by `src/app/core/mod.rs`, and applied in `src/tracer/mod.rs`.
- Automation flags (`screenshot`, `screenshot-delay`, `auto-exit`, `perf`) were previously implemented directly in app lifecycle/redraw code in `src/app/core/mod.rs`.

## Implementation Plan (per unimplemented flag)

### 0) parser cleanup

- Update argument parsing to accept only `--windowed`.
- Remove `-w` handling from `AppOptions::from_args`.
- Keep behavior otherwise unchanged.

### 1) `--no-shadows`

- Reintroduce runtime render feature toggles (`RenderFlags`) mapped from `AppOptions`.
- Thread flags to tracer recording path.
- Gate shadow map generation and filtering in `Tracer::record_trace`.
- Preserve required barriers/layout transitions when pass is skipped.

### 2) `--no-denoise`

- Gate denoiser resource transition and denoiser pass.
- Gate copy of current frame to previous buffers if denoiser is disabled.
- Verify composition/post-processing still read valid inputs.

### 3) `--no-god-rays`

- Gate god-ray pass dispatch.
- Keep synchronization correct between tracer/composition passes when skipped.

### 4) `--no-lens-flare`

- Gate sun visibility, lens flare, and downsample passes.
- Gate or adjust clear for lens flare counter texture.
- Confirm composition path handles no-lens-flare state without stale data.

### 5) `--no-tracer`

- Gate main tracer compute pass.
- Ensure output texture is still valid for later stages (clear/fallback path).
- Re-check that skipping tracer does not cause undefined layout/read hazards.

### 6) `--no-particles`

- Gate particle CPU simulation update in app loop.
- Gate particle GPU upload/queries.
- Gate particle draw path in tracer graphics passes.
- Keep upload fallback behavior defined (empty particle buffers if needed).

### 7) `--no-flora`

- Gate flora and leaves graphics passes.
- Ensure graphics target clear behavior is correct when flora and particles are both disabled.
- Confirm composition result remains stable with flora disabled.

### 8) `--screenshot <path>` and `--screenshot-delay <sec>`

- Reintroduce app fields for screenshot automation state.
- Reintroduce `save_screenshot()` readback + PNG write path.
- Start timer when first real render begins and trigger one-time capture after delay.
- Add robust logging for success/failure and invalid path errors.

### 9) `--auto-exit <sec>`

- Reintroduce render-start timer tracking.
- Exit app once delay is reached using existing termination path.
- Ensure graceful shutdown (fences/device idle behavior unchanged).

### 10) `--perf`

- Reintroduce per-frame perf logging toggle in app state.
- Re-enable timing instrumentation around major frame stages.
- Keep logs bounded and readable to avoid excessive spam impact.

### 11) `--help`

- Add explicit `--help` handling in argument parsing/entry path.
- Print a concise usage guide that includes every supported flag, expected value type, and defaults where relevant.
- Include at least these usage examples:
  - `--windowed`
  - `--no-shadows --no-denoise`
  - `--screenshot out.png --screenshot-delay 3`
  - `--auto-exit 10 --perf`
- Exit successfully after printing help (do not continue app initialization).
- Ensure `-w` is not documented once removed.

## Suggested Execution Order

1. parser cleanup (`-w` removal).
2. render flag plumbing (`RenderFlags` + tracer integration).
3. pass gates one by one (`shadows` -> `denoise` -> `god-rays` -> `lens-flare` -> `tracer` -> `particles` -> `flora`).
4. automation flags (`screenshot`, `screenshot-delay`, `auto-exit`).
5. perf logging (`--perf`).
6. help/usage output (`--help`).
7. regression verification across combinations.

## Verification Checklist

- `--windowed` works; `-w` no longer recognized.
- Each `--no-*` flag visibly disables only its targeted subsystem.
- No validation/layout errors when any pass is disabled.
- `--screenshot` writes exactly one image at expected delay.
- `--auto-exit` exits near configured delay without crash.
- `--perf` logs timings only when enabled.
- `--help` prints all supported flags, argument forms, defaults, and exits cleanly.
- Combined flags behave deterministically.

## Uncertainty / Risk Notes

- The reverted implementation in `8c32c272` included additional broad runtime changes (loading flow, frame sync, particle behavior). We should restore only flag-related logic, not unrelated behavior changes.
- Pass gating in tracer is synchronization-sensitive; barriers and clear paths must be reviewed carefully to avoid undefined image state.
