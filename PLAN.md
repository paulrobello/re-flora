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

currently the particles are being updated in every single frame for its positions.

to fit in the low framerate style we are currently in, we are using a clamping technique in its shaders to clamp their position in world space.

i would like to request for a completely different approach:

for each frame, we only update the position for a certain part of total particles.

let's say, only 1/N of the total particles, where you use some bucket to assign a update index for each particle upon their creation

the bucket size is N, so the update index is a int ranged from 0 to N-1

i believe this refactor can totally be done in cpu side, the update idx won't need to pass to GPU.

and, just remove the clamping from shader.
