# Terrain Build Command Refactor Plan (SRP-first)

## Short answer

Your instinct is correct: `build tree` and `build fence` are high-level gameplay behaviors, not voxel-engine primitives.
The voxel layer should accept generic shape/material edit commands (for example: "stamp round cone", "fill box", "union/intersect/subtract"), while tree/fence systems compile into those commands.

## Current architecture (what exists today)

### Good parts already in place

- Low-level voxel write path is already shape-centric in `PlainBuilder::chunk_modify_with_voxel_type(...)` (`src/builder/plain/mod.rs:226`).
- GPU shader is already primitive/SDF oriented (`RoundCone + BVH`) in `shader/builder/chunk_writer/chunk_modify.comp:6`.
- Tree generation already outputs round-cone trunk geometry in `Tree::trunks()` flow used at `src/app/core.rs:1442`.

### SRP leakage points

- `WorldEdit` mixes high-level behavior and low-level engine edits in one enum (`src/app/core.rs:237`).
  - High-level: `PlaceTree`, `PlaceFence`
  - Low-level: `PlaceTreeGeometry`, `ClearVoxelRegion`, `RebuildMesh`
- Scheduling has dedicated high-level lanes (`resolve_high_level`, `resolve_fence`) in `src/app/core.rs:274`.
- `App` owns behavior expansion and voxel pipeline orchestration inside the same "world edit" mechanism (`src/app/core.rs:1287`).

This makes the command model conceptually inconsistent and makes future primitives harder to scale.

## Target design

### Layer split

- **Behavior layer (game/domain):** tree tool, fence tool, procedural placement, audio/particles coupling.
- **Voxel edit layer (engine):** generic, shape/material operations only.
- **Build pipeline layer:** mesh/contree/scene accel rebuild commands.

### Rule of thumb

- If a command name references a domain object (`Tree`, `Fence`), it belongs in behavior layer.
- If a command name references geometry/material ops (`RoundCone`, `Cuboid`, `VoxelType`, `BooleanOp`), it belongs in voxel layer.

## Proposed command model

### 1) Replace low-level command set

Create an engine-level enum, e.g.:

- `VoxelEdit::StampRoundCones { cones, voxel_type }`
- `VoxelEdit::FillAabb { offset, dim, voxel_type }`
- (optional next) `VoxelEdit::StampCuboids { cuboids, voxel_type }`

And keep build commands separate:

- `BuildEdit::RebuildMesh { bound }`

Then schedule/execute these only in engine backend traits.

### 2) Move behavior commands out of `WorldEdit`

Replace:

- `WorldEdit::PlaceTree`
- `WorldEdit::PlaceFence`

With behavior service calls that produce `VoxelEditBatch + BuildEditBatch`.

Example conceptual flow:

1. `TreePlacementService::compile(request) -> CompiledTreeEdits`
2. compiled result contains:
   - voxel edits (`StampRoundCones`, maybe leaf placement side-channel)
   - build bounds
   - side effects for app systems (audio/emitters/tree registry)
3. app executes low-level batches through backend
4. app applies side effects

Fence does the same through `FencePlacementService`.

## Concrete phased refactor

### Phase 1 - Separate types without behavior change

- Introduce new enums/structs:
  - `VoxelEdit`, `VoxelEditBatch`
  - `BuildEdit`, `BuildEditBatch`
- Keep existing code paths, but route `PlaceTreeGeometry` and `ClearVoxelRegion` through `VoxelEdit` conversion.
- Keep `PlaceTree`/`PlaceFence` temporarily as adapters.

Outcome: no gameplay change, but low-level vocabulary is clean.

### Phase 2 - Extract behavior compilers

- Add:
  - `TreePlacementService`
  - `FencePlacementService`
- Move geometry expansion logic from:
  - `apply_place_tree_edit(...)` (`src/app/core.rs:1413`)
  - `apply_place_fence_edit(...)` (`src/app/core.rs:1385`)
- Services output compiled low-level edits + side-effect payload.

Outcome: app becomes orchestrator, not editor+compiler+executor in one function.

### Phase 3 - Remove high-level variants from world edit pipeline

- Delete `WorldEdit::PlaceTree` and `WorldEdit::PlaceFence`.
- Delete `WorldBuildStage::ResolveHighLevel` and related scheduler fields (`src/app/core.rs:262`).
- Retain only engine-level stage queues:
  - voxel writes
  - accel rebuild

Outcome: world edit pipeline is engine-only and SRP aligned.

### Phase 4 - Generalize shape support (optional, but recommended)

- Add shape union type in engine (CPU + GPU contract), for example:
  - `enum VoxelPrimitive { RoundCone(...), Cuboid(...) }`
- Update shader and buffer layouts to support multiple primitive kinds.
- Keep behavior systems as primitive compilers.

Outcome: future content (rocks, walls, arches) can reuse same command path.

## Minimal API sketch

```rust
// Engine-facing
pub enum VoxelEdit {
    StampRoundCones {
        bvh_nodes: Vec<BvhNode>,
        cones: Vec<RoundCone>,
        voxel_type: u32,
    },
    FillAabb {
        offset: UVec3,
        dim: UVec3,
        voxel_type: u32,
    },
}

pub enum BuildEdit {
    RebuildMesh { bound: UAabb3 },
}

pub trait WorldBuildBackend {
    fn apply_voxel_edit(&mut self, edit: VoxelEdit) -> Result<()>;
    fn apply_build_edit(&mut self, edit: BuildEdit) -> Result<()>;
}
```

## Migration notes (important)

- Do not move tree leaf registration (`tracer.add_tree_leaves`) into voxel engine. Keep it behavior/app side.
- Do not move audio/particle emitters into engine layer.
- Keep `PlainBuilder` focused on primitive voxel stamping and clearing only.
- Preserve bound computation ownership near compilers so rebuild scope stays correct.

## Validation checklist

- Tree placement still produces identical trunk voxels (same cone count and AABB).
- Fence placement still produces identical columns.
- `ClearVoxelRegion` behavior unchanged.
- Mesh/contree/scene accel rebuild order unchanged.
- Procedural forest generation unchanged.
- Audio sources and leaf emitters still attach/remove correctly.

## Suggested implementation order

1. Introduce `VoxelEdit`/`BuildEdit` and backend methods.
2. Convert existing low-level execution to new types.
3. Extract tree/fence compilers into services.
4. Remove high-level world edit variants and resolve stage.
5. Optional: extend primitive set beyond `RoundCone`.

---

If you want, next I can implement Phase 1 directly so your codebase has a clean engine-level command vocabulary first, with no behavior breakage.
