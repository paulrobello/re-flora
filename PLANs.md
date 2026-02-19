# PLANs

## Goal: Robust, Generalized, Modular Voxel World Building

Make the voxel world building pipeline robust, generalized, and modular across terrain, trees,
flora, and future structures. Replace specialized one-off flows with a unified world-edit model,
explicit build stages, precise dirty tracking, and testable transactional execution.

## Full Plan

### 1) Define a Unified World Edit API

Introduce a shared `WorldEdit` command model so all world mutations use one path:

- `Fill`
- `SdfUnion`
- `SdfSubtract`
- `PlacePrefab`
- `ScatterPoints`

All systems (trees, terrain tools, future structures) should emit these commands instead of
calling low-level builders directly.

### 2) Split World Building Into Explicit Stages

Create clear stage boundaries:

- Stage A: `VoxelWrite` (mutate voxel atlas)
- Stage B: `DerivedData` (surface extraction + flora instance derivation)
- Stage C: `AccelBuild` (contree build + scene accel updates)
- Stage D: `GameplayAttachments` (audio, particles, metadata links)

Each stage should have a stable interface and explicit inputs/outputs.

### 3) Add Precise Dirty Region Tracking

Track touched voxel bounds per edit and map them to affected chunk IDs once.

- Maintain per-chunk and per-stage dirty flags
  - `voxel_dirty`
  - `surface_dirty`
  - `accel_dirty`
- Rebuild only dirty chunks, not broad historical unions.

### 4) Make Edits Transactional

Add `WorldEditBatch`:

- Validate all commands first
- Apply atomically
- Trigger dependent stage rebuilds only after successful apply
- On failure, preserve prior state (or rollback touched chunks)

### 5) Generalize Builder Backends

Abstract current builder implementations behind traits:

- `VoxelWriter`
- `SurfaceBuilder`
- `AccelBuilder`

Current plain/surface/contree path becomes one backend implementation. This allows future CPU
fallback, streaming, or experimental builders without changing higher-level edit logic.

### 6) Decouple Tree-Specific Logic From Core Build Pipeline

Tree generation should produce:

- generic `WorldEdit` commands for geometry
- generic attachment descriptors for leaves/audio/particles

Keep tree behavior as a user of the world pipeline, not a special hardcoded path.

### 7) Add a Dependency Graph and Scheduler

Encode dependencies between stages:

`VoxelWrite -> DerivedData -> AccelBuild -> GameplayAttachments`

Use a scheduler to parallelize independent per-chunk work while preserving dependency order and
safe synchronization.

### 8) Add Robustness Guards and Invariants

Enforce correctness at API boundaries:

- OOB and invalid bound checks
- chunk-bound invariants for all edits
- idempotency expectations for repeat edits
- structured error reporting by stage/chunk

### 9) Test Strategy

Add automated coverage for both correctness and rebuild scope:

- golden tests for voxel diffs from known edit batches
- chunk rebuild scope tests (only expected chunks rebuilt)
- stress tests for large procedural edit batches
- regression tests for tree place/remove consistency

### 10) Migration Plan

Phase 1:

- Wrap existing tree placement flow in `WorldEditBatch`
- Keep old builders internally, only change entry point shape

Phase 2:

- Route terrain/procedural generation through the same world edit API

Phase 3:

- Remove direct low-level builder calls from app/gameplay layer

Phase 4:

- Enable full stage scheduler and backend trait abstraction
- Delete legacy special-case pipeline paths
