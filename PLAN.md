# Plan to Split `src/app/core.rs` (KISS + SRP)

## Goal
- Reduce `src/app/core.rs` from a 4k-line god file into small, single-purpose modules.
- Keep runtime behavior the same while improving readability, testability, and ownership.
- Preserve external API (`AppController` still drives `App` the same way).

## KISS + SRP Rules for This Refactor
- One reason to change per module.
- Keep module boundaries practical, not academic (avoid too many tiny files).
- Move code first, then improve internals in small follow-ups.
- No behavior changes mixed into extraction commits.

## Target Structure

```text
src/app/
  app_controller.rs
  mod.rs
  core/
    mod.rs                # App struct + high-level orchestration only
    boot.rs               # App::new setup and initialization pipeline
    lifecycle.rs          # on_terminate, on_about_to_wait, on_resize, drop helpers
    input.rs              # window/device input handling and cursor/tool state
    render_loop.rs        # redraw frame pipeline and GPU submission/present flow
    ui.rs                 # egui panels/layout dispatch and UI-driven state updates
    ui_style.rs           # colors, widget visuals, style/theme helpers
    world_edits.rs        # VoxelEdit/BuildEdit/WorldEditPlan + backend trait
    world_ops.rs          # mesh generation + edit-plan execution plumbing
    vegetation.rs         # tree placement, procedural generation, fence placement
    particles.rs          # emitters, constraints, particle simulation/update
    environment.rs        # day/night math and environment parameter helpers
```

Notes:
- Keep `GuiAdjustables` in `gui_config.rs` for now.
- Keep `App` as the integration root in `core/mod.rs`.

## Responsibility Map (What Moves Where)

### 1) `core/world_edits.rs`
- Move data/abstractions only:
  - `TreePlacement`, `TreeAddOptions`, edit structs, `VoxelEdit`, `BuildEdit`, `WorldEditPlan`.
  - `WorldBuildBackend` and `BuilderOnlyWorldBackend`.
- Why: these are pure domain/edit contracts and should not live inside app event code.

### 2) `core/world_ops.rs`
- Move world edit execution and mesh rebuilding:
  - `execute_edit_plan*`, `mesh_generate`, chunk-affected helper.
  - `impl WorldBuildBackend for App` (can stay near App or here if ergonomic).
- Why: single place for "apply world mutation + rebuild affected data".

### 3) `core/vegetation.rs`
- Move tree and map-vegetation logic:
  - Tree compile/placement services, tree records, add/remove/clear/regenerate tree flow.
  - Fence post placement and terrain height queries used by vegetation placement.
  - Tree variation config and UI editor helpers specific to tree tuning.
- Why: vegetation lifecycle is a coherent domain separate from rendering/input.

### 4) `core/particles.rs`
- Move particle-only logic:
  - emitter wrappers, map emitters, update loop, terrain constraints, emitter mutation.
  - `ButterflyQueryTarget`, `BirdQueryTarget`.
- Why: simulation update and constraints are isolated from UI and GPU frame submission.

### 5) `core/ui_style.rs`
- Move static UI style/theme concerns:
  - color constants, `apply_gui_style`, `widget_visuals`, item panel visuals.
- Why: styling should be isolated from state mutation and event handling.

### 6) `core/ui.rs`
- Move UI composition and UI-to-state updates:
  - config/settings panel rendering, item panel draw entry, FPS overlay.
  - return a compact `UiActions` struct to request side effects (rebuild tree, regenerate forest).
- Why: UI rendering and gameplay side effects should be separated by a small action contract.

### 7) `core/input.rs`
- Move input logic:
  - keyboard/mouse handling, slot selection, shovel input gate, cursor sync.
- Why: input state transitions should be easy to reason about independently.

### 8) `core/render_loop.rs`
- Move redraw pipeline:
  - per-frame update order, tracer buffer updates, trace + blit + render pass + submit/present.
- Why: rendering path is the hottest and most complex path; isolate it for clarity.

### 9) `core/boot.rs` + `core/lifecycle.rs` + `core/environment.rs`
- `boot.rs`: constructor and setup only (`App::new`, resource creation, initial world seed).
- `lifecycle.rs`: resize/terminate/about-to-wait/drop-related lifecycle.
- `environment.rs`: sun/day-night math and small pure helpers.
- Why: boot and lifecycle are distinct from runtime frame logic.

## Refactor Order (Low-Risk Sequence)

1. Extract `world_edits.rs` (types/traits only, no behavior changes).
2. Extract `world_ops.rs` and keep call sites unchanged.
3. Extract `environment.rs` (pure helpers first).
4. Extract `particles.rs` (mostly internal methods, minimal external touchpoints).
5. Extract `vegetation.rs`.
6. Extract `ui_style.rs`, then `ui.rs` with a `UiActions` return object.
7. Extract `input.rs`.
8. Extract `render_loop.rs`.
9. Keep `core/mod.rs` as orchestrator and remove dead code/imports.

## Definition of Done Per Step
- `cargo check` passes after each extraction.
- No change in controls, tree placement flow, particle behavior, or render output semantics.
- Module has a clear single responsibility and no circular dependency.
- `core/mod.rs` mostly wires calls; it should not contain long algorithmic blocks.

## Guardrails
- Do not rewrite algorithms during extraction.
- If a function is large, move it as-is first, then split internally in a separate commit.
- Prefer `pub(super)` visibility to avoid leaking internals.
- Keep data ownership where it already lives unless extraction requires passing a small context struct.

## Suggested Commit Plan
- `refactor: extract world edit contracts from app core`
- `refactor: isolate world mesh rebuild and edit execution pipeline`
- `refactor: move particle simulation and terrain constraints into core particles module`
- `refactor: move vegetation placement and procedural tree flow into dedicated module`
- `refactor: separate ui styling and panel composition from app event loop`
- `refactor: split input handling and frame rendering pipeline from app core`

## Expected Result
- `core/mod.rs` becomes a readable coordinator.
- Each module has a narrow reason to change (SRP).
- Future features (new tools, new emitters, new UI panel, new world edit) can land without touching unrelated systems.
