# Terrain Edit API Cleanup Plan

## Status (as of now)
The SRP refactor is functionally complete:
- High-level behavior (`tree`, `fence`) is compiled before execution.
- Low-level execution uses engine edits:
  - `VoxelEdit` (voxel mutation)
  - `BuildEdit` (derived-data rebuild)
- `WorldEdit` no longer carries `PlaceTree` / `PlaceFence` variants.
- `TreePlacementService` and `FencePlacementService` exist and compile behavior into low-level edits.

So the remaining work is API cleanliness, not architecture correctness.

## Remaining problem
Both single-edit and batch concepts are visible in code paths.
That creates unnecessary API surface and confusion.

## Goal for this task
Provide one clear execution model:
- `VoxelEdit` and `BuildEdit` remain the atomic command types.
- Execution is batch-first.
- Single-edit execution is internal convenience only (or removed).

## Target shape
Use a single plan object for execution, for example:
- `WorldEditPlan { voxel_edits: Vec<VoxelEdit>, build_edits: Vec<BuildEdit> }`

And one primary executor path:
- `execute_edit_plan(plan)`

Behavior services compile into `WorldEditPlan` (plus side-effect payloads for app-layer systems).

## Concrete refactor steps

### Step 1 - Introduce unified execution container
- Add `WorldEditPlan`.
- Replace scattered `VoxelEditBatch` + `BuildEditBatch` handling at call sites with plan construction.
- Keep adapter conversion where needed during transition.

### Step 2 - Unify backend execution API
- Prefer one backend entrypoint for each class of edit looped by the executor:
  - `apply_voxel_edit(edit)`
  - `apply_build_edit(edit)`
- Ensure all execution flows route through `execute_edit_plan`.
- Remove parallel/duplicated execution helpers once migrated.

### Step 3 - Restrict user-facing surface
- Keep exported/public command vocabulary to atomic edit types and high-level behavior requests.
- Make batch wrappers internal where possible.
- Keep single-edit helpers private convenience wrappers only.

### Step 4 - Tighten naming and docs
- Ensure naming clearly reflects layering:
  - behavior compile
  - engine edit execution
  - side-effect application
- Add short docs on each boundary function.

## Validation checklist
- `cargo check` passes.
- Tree placement output unchanged (same trunk stamping and bounds).
- Fence placement output unchanged.
- Terrain clear/rebuild flows unchanged.
- Procedural generation behavior unchanged.
- No `PlaceTree`/`PlaceFence` logic leaks into engine edit executor.

## Out of scope (for this task)
- New primitive types (cuboid/multi-primitive union).
- Shader contract expansion.
- Gameplay changes.
