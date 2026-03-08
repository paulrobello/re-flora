# grass coloring rework

## goal

replace noisy hash based grass color variation with stable world space color patching.

## later optimization

- move grass patch random source from per vertex shader compute to flora generation.
- store either:
  - normalized patch noise value, or
  - compact band index (`0..2`) in instance data.
- use stored value in vertex shader to reduce repeated ALU.
- only do this after profiling confirms stage 1 cost is material.

# wind rework

## current model (as implemented)

- wind uses two scrolling OpenSimplex2 fBm fields:
  - direction field
  - strength field
- direction is built from primary + detail sample, converted to planar angle.
- strength noise is remapped to min/max wind strength.
- flora vertex shaders sample wind by world position and current time.
- wind displacement is scaled by squared vertical gradient, so blade bases move less than tips.

## known note

- CPU and shader wind minimum strength constants currently differ:
  - `src/wind.rs`: `WIND_MIN_STRENGTH = 1.5`
  - `shader/include/wind.glsl`: `WIND_MIN_STRENGTH = 0.5`
- this creates behavior mismatch between systems driven by CPU wind and GPU wind.

## follow up (not in this stage)

- align CPU and GPU wind constants and verify downstream particle behavior.

# particle update rules

## objective

reduce per-frame particle simulation cost while preserving the current low-framerate visual style.

## approach

- assign each particle a stable update bucket `update_bucket` in range `0..N-1` at spawn time.
- on frame `f`, only update particles where `update_bucket == (f % N)`.
- non-updated particles keep their previous transform for that frame.
- to preserve average motion over time, updated particles integrate using scaled timestep (`dt * N`).

## implementation notes

- bucket assignment should be decorrelated (stable random/hash from particle id + spawn seed) to avoid visible striping.
- run this scheduling on CPU if particle simulation is CPU-owned; do not send `update_bucket` to GPU unless needed by existing GPU-side systems.
- remove shader world-space clamping once CPU-side bounds/respawn rules are in place.

## safety and validation

- define explicit bounds + respawn policy after clamping removal.
- verify no instability (teleporting, tunneling, NaNs) under large `N`.
- measure:
  - CPU update time reduction vs baseline
  - visual artifact threshold per `N` at target camera distances
