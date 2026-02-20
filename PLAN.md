# Terrain Removal Plan (Shovel)

## Goal

Add a terrain-edit operation that removes voxels inside a radius around a center point, and wire it to shovel usage.

## Scope

- Add a new world-edit primitive for sphere/radius-based edits.
- Reuse the existing voxel edit pipeline (`VoxelEdit` -> `PlainBuilder` -> mesh rebuild).
- Drive the operation from gameplay input when shovel is selected.

## Implementation Steps

1. Add removal edit types in app layer

- File: `src/app/core.rs`
- Add a new edit payload:
  - `TerrainRemovalEdit { center: Vec3, radius: f32 }`
- Extend voxel edit enum with a sphere-capable branch:
  - Preferred: `VoxelEdit::StampSpheres { bvh_nodes, spheres, voxel_type }`
- Add a small compile service (`TerrainRemovalService`) that converts world-space center/radius to voxel-space sphere data + rebuild bound.

2. Extend builder resources for spheres

- File: `src/builder/plain/resources.rs`
- Add a `spheres` GPU buffer to `PlainBuilderResources`.
- Allocate/fill it using the same pattern as `round_cones` and `cuboids`.

3. Extend `PlainBuilder` APIs for sphere modify

- File: `src/builder/plain/mod.rs`
- Add constants:
  - `PRIMITIVE_KIND_SPHERE`
  - `VOXEL_TYPE_EMPTY = 0` (for removal writes)
- Add methods:
  - `chunk_modify_spheres_with_voxel_type(...)`
  - internal `update_spheres(...)`
- Route `VoxelEdit::StampSpheres` through this method.

4. Add sphere hit logic to chunk modify shader

- File: `shader/builder/chunk_writer/chunk_modify.comp`
- Add `Sphere` struct and `B_Spheres` binding.
- Add primitive branch:
  - if `primitive_kind == PRIMITIVE_KIND_SPHERE`, test distance to center <= radius.
- On hit, write `fill_voxel_type` to `chunk_atlas`.
  - Removal uses `VOXEL_TYPE_EMPTY`.

5. Compile removal request to bounded world edit

- File: `src/app/core.rs`
- In `TerrainRemovalService::compile`:
  - Convert to voxel space (`* 256.0`) consistent with existing placement code.
  - Build sphere AABB and BVH root bound.
  - Clamp bound to world dimensions (`CHUNK_DIM * VOXEL_DIM_PER_CHUNK`) to avoid out-of-range rebuilds.
- Return `WorldEditPlan::with_voxel_and_build(...)`.

6. Input and equip gating (shovel)

- File: `src/app/core.rs`
- Add gameplay trigger path (mouse click/hold) for terrain removal.
- Gate by selected bottom-bar slot (shovel slot only).
- Initial center calculation:
  - `center = camera_pos + camera_front * interaction_distance`
- Expose radius + distance as constants first; tune after testing.

7. Rate limiting and batching

- File: `src/app/core.rs`
- Prevent over-updating while holding mouse:
  - e.g. carve at fixed interval (50-100ms).
- If multiple carve ops happen in one frame, union bounds and rebuild once.

8. Validation

- Functional checks:
  - Removing voxels works at map center, borders, chunk boundaries.
  - Only in-radius voxels are cleared.
  - Shovel gating is respected.
- Stability/perf:
  - Holding carve does not stall frame time excessively.

## Rollout Order

1. Backend support (types + builder + shader) with a debug call site.
2. Hook gameplay input + shovel gating.
3. Tune radius/distance/rate-limit and verify UX.

## Risks / Watchouts

- Coordinate-space mismatch (world units vs voxel units).
- Rebuild bounds must be clamped to world chunk space.
- Frequent rebuilds can be expensive without throttling.
