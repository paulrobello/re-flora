# Plan: Unified GPU Flora Edit Pipeline

## Goal
Refactor flora editing so chunk flora changes always go through one GPU-only occupancy pipeline (no CPU readback/merge), with deterministic flora reconstruction from the existing planting rule.

## Core Data Structure
- Shared temporary occupancy resource (single reusable buffer, not per-chunk).
- Layout: `u32` per voxel in chunk-local space (`0` = empty, `1` = occupied).
- Size target: `VOXEL_DIM_PER_CHUNK.x * VOXEL_DIM_PER_CHUNK.y * VOXEL_DIM_PER_CHUNK.z` entries.
- Reused every edit call; cleared before use.

## Determinism Assumption
- Flora type/seed is fully determined by existing rule and world coordinate.
- Therefore rebuilding instances from occupancy + surface/rule is stable and consistent.
- Existing flora can be reconstructed without storing extra type/index metadata.

## Unified Stages
1. **Instances -> Occupancy**
   - Compute pass reads all species instance buffers for target chunk.
   - Writes occupancy `1` at each stem voxel position.

2. **Occupancy Edit (Unified for tools)**
   - One compute pass for all spherical edits with mode flag.
   - Inputs: occupancy, surface data, sphere center/radius, mode.
   - `remove` mode (shovel): set occupancy to `0` inside sphere.
   - `add` mode (magic wind/staff): inside sphere, set occupancy to `1` only if surface voxel is plantable.

3. **Occupancy -> Instance Buffers**
   - Clear per-species output counters.
   - Compute pass scans occupancy and for each `1` voxel:
     - Evaluates existing planting rule (same as initial generation).
     - Determines species/seed deterministically.
     - Appends to corresponding species instance buffer via atomic counters.

4. **Commit Lengths**
   - Update `instances_len` from GPU counters.
   - Read back only small counter buffer if CPU-side lengths are needed.

## System Mapping
1. **Chunk Init**
   - Surface data -> occupancy seed (plantable positions).
   - Occupancy -> instance buffers (existing planting rule).

2. **Shovel**
   - Instance buffers -> occupancy.
   - Unified occupancy edit in `remove` mode.
   - Occupancy -> instance buffers.

3. **Magic Growth Wind / Staff Regen**
   - Instance buffers -> occupancy.
   - Unified occupancy edit in `add` mode (requires surface data).
   - Occupancy -> instance buffers.

## Implementation Steps
1. Add shared occupancy buffer + per-species counter buffers in `SurfaceResources`.
2. Add new compute shaders:
   - `instances_to_occupancy.comp`
   - `edit_occupancy_sphere.comp` (mode flag: remove/add)
   - `occupancy_to_flora_instances.comp`
3. Wire pipelines and dispatch order in `SurfaceBuilder`.
4. Replace current CPU merge regen path with unified GPU occupancy pipeline.
5. Route shovel and staff to the same occupancy edit stage with different mode.
6. Keep temporary debug counters/logs until visual parity and perf are confirmed.

## Validation
- Functional:
  - Shovel removes flora in sphere.
  - Staff/wind restores flora in sphere based on same planting rules.
  - Repeated edits remain stable and deterministic.
- Performance:
  - Verify no per-edit CPU roundtrip for instance data.
  - Compare frametime while holding shovel/staff before vs after refactor.

## Files Likely Touched
- `src/builder/surface/resources.rs`
- `src/builder/surface/mod.rs`
- `src/app/world_ops.rs`
- `src/app/core/vegetation.rs`
- `src/app/core/input.rs`
- `shader/builder/surface/*.comp`
- `shader/include/*.glsl` (shared structs/helpers if needed)
