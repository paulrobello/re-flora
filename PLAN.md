GUI config reflection refactor plan
===================================

Background and motivation
-------------------------

- `config/gui.toml` is intended to be the single source of truth for all GUI tweakable parameters (ids, labels, types, ranges, and persisted values).
- The current Rust side duplicates this information in several places:
  - the `declare_gui_adjustables!` macro invocation in `src/app/gui_config.rs`
  - the `load_from_config` function, which maps `GuiParamKind` and `GuiParamValue` into concrete `*Param` types
  - the `get_*_param` and `get_*_param_mut` helpers, which map string ids back to struct fields for saving and rendering
- This duplication makes it easy to introduce mismatches between `gui.toml` and `GuiAdjustables` (missing ids, wrong kinds, or params present only on one side).
- Recent changes exposed this clearly: adding a validation step showed that a large number of params in `gui.toml` are not wired into `GuiAdjustables`, and the wiring has to be updated manually in multiple match statements.

Goal
----

Create a single, deterministic reflection layer where `config/gui.toml` is the only source of truth, and all GUI wiring in Rust is generated from it. In particular:

- Every parameter defined in `config/gui.toml` must appear in the GUI with the correct type, label, default, and range.
- There must be no manual string id wiring or hand maintained match statements for GUI parameters.
- If `gui.toml` changes in a way that cannot be reflected (for example, a new kind that is not supported), the build should fail in a clear way, rather than silently ignoring the param at runtime.

Current state (before refactor)
-------------------------------

- `config/gui.toml`
  - Contains sections and params with fields: `id`, `kind`, `label`, `type` (enum name), and `[data]` (value plus optional range).
  - This file is edited at runtime by the app and on disk by hand.

- `src/app/gui_config_model.rs`
  - Defines `GuiConfigFile`, `GuiSection`, `GuiParamKind`, and `GuiParamValue`, plus helpers like `get_float`, `get_int`, etc.
  - This is a generic representation of what is in `gui.toml` and is already a reflection friendly model.

- `src/app/gui_config_loader.rs`
  - Handles loading and saving `GuiConfigFile` from `config/gui.toml`.

- `src/app/gui_config.rs`
  - `declare_gui_adjustables!` macro invocation declares `GuiAdjustables` with a fixed set of fields and compile time defaults.
  - `load_from_config` builds a `HashMap<String, Box<dyn Any>>` from `GuiConfigFile` and then manually constructs `GuiAdjustables` by calling `get_param!("id", TypeParam)` for every field.
  - `get_*_param` and `get_*_param_mut` functions map ids back to references into `GuiAdjustables` for saving and for `render_gui_from_config`.
  - `render_gui_from_config` iterates config sections and params, looks up the corresponding field via the `get_*_param_mut` helpers, and draws sliders, checkboxes, and color pickers.

- Observed problems
  - The set of ids in `GuiAdjustables` is not enforced to match the ids in `gui.toml`.
  - The set of ids in `get_*_param` helpers is also not enforced to match either side.
  - Adding or renaming a param currently requires editing `gui.toml`, `declare_gui_adjustables!`, and at least one `get_*_param` match statement, and these edits are easy to forget.
  - We added runtime validation to check for unwired params, but the fix still requires manual wiring.

Target design (after refactor)
------------------------------

High level idea: generate the `GuiAdjustables` struct and its helpers from `config/gui.toml` at compile time, using a small code generation step. The generated code becomes the single Rust side description of all GUI parameters.

Key properties of the target design:

- `config/gui.toml` remains the single source of truth for:
  - parameter ids
  - kinds and Rust `*Param` types
  - labels
  - default values and ranges
- A build time tool reads `config/gui.toml` and produces a generated Rust module, for example `src/app/generated/gui_adjustables_gen.rs` (or a file in `OUT_DIR` included with `include!`).
- The generated module defines:
  - the `GuiAdjustables` struct with one field per param
  - an implementation of `Default` and/or `from_config` that initializes `GuiAdjustables` from a `GuiConfigFile`
  - the `get_*_param` and `get_*_param_mut` helpers used by `save_to_config` and `render_gui_from_config`
  - optionally, a static description table mapping ids to field offsets or accessors for more introspection (if needed later)
- `src/app/gui_config.rs` becomes a thin wrapper that:
  - includes the generated module
  - provides `GuiAdjustables::default` and `GuiAdjustables::from_config` by delegating to generated code
  - exposes `render_gui_from_config` that relies on generated helpers instead of hand written match statements.
- If a param exists in `gui.toml` but cannot be represented in the generated Rust code (unknown `kind`, invalid ranges, missing type mapping), the code generation step should fail the build with a clear error message.

Implementation plan
-------------------

Phase 1: Stabilize the current reflection layer

1. Keep the existing `GuiConfigFile` and loader as is.
2. Keep the current `render_gui_from_config` as the runtime GUI entry point.
3. Replace the panic for unwired params in `load_from_config` with a warning (already done) so the app runs even while wiring is incomplete.

Phase 2: Introduce code generation skeleton

4. Add a small build time tool responsible for reading `config/gui.toml` and emitting Rust code:
   - Implement this either as:
     - a dedicated binary in `tools/` invoked from `build.rs`, or
     - `build.rs` logic directly if that stays simple enough.
   - Use the existing `GuiConfigFile` model to parse `config/gui.toml` so that there is one definition of the schema.
5. Define a generation target file, for example:
   - `src/app/generated/gui_adjustables_gen.rs` committed to the repo, or
   - a file under `OUT_DIR` (e.g. `include!(concat!(env!("OUT_DIR"), "/gui_adjustables_gen.rs"));`).
   First iteration can write into `src/app/generated/` for easier iteration and review.
6. In `src/app/gui_config.rs`, add an `include!` of the generated file and ensure it compiles alongside the existing manual `GuiAdjustables` definition (initially generated under a different name, e.g. `GeneratedGuiAdjustables`).

Phase 3: Generate the struct and constructors

7. Extend the generator so that it emits:
   - a `pub struct GuiAdjustables` mirroring the fields currently defined by `declare_gui_adjustables!` but driven directly by the list of params in `GuiConfigFile`.
   - `impl GuiAdjustables { pub fn from_config(config: &GuiConfigFile) -> GuiAdjustables { ... } }` that mirrors the logic of `load_from_config` but without any string match boilerplate.
   - `impl Default for GuiAdjustables` that calls `GuiConfigLoader::load()` and `from_config`.
8. Gradually remove the manual `declare_gui_adjustables!` usage for the struct definition, or adjust the macro so it is only responsible for UI metadata that is not present in the config (if any).
9. Update `src/app/core` and any other call sites to use the generated `GuiAdjustables::default` / `from_config` instead of the existing ones.

Phase 4: Generate accessors and GUI wiring

10. Extend the generator to emit `get_*_param` and `get_*_param_mut` functions based on the param list and their kinds:
    - For each param, decide the concrete Rust type (`FloatParam`, `IntParam`, `UintParam`, `BoolParam`, `ColorParam`) from its `kind`.
    - Generate match arms mapping `id` strings to the corresponding struct fields, identical to the current hand written helpers but derived mechanically.
11. Switch `save_to_config` and `render_gui_from_config` to use the generated accessors exclusively.
12. Remove the hand written `get_*_param` / `get_*_param_mut` implementations from `src/app/gui_config.rs` once the generated versions are in place and tests pass.

Phase 5: Enforce single source of truth

13. Move the strict validation into the code generation step:
    - When reading `config/gui.toml` for generation, assert that every param has a known `kind` and can be mapped to a `*Param` type.
    - If any issue is found (unknown kind, incompatible ranges, or anything else), abort generation and print a clear error message.
14. After generation, there should be no need for runtime checks about unwired params, because the generator will have produced a field and accessor for every param or failed the build.
15. Remove the runtime unwired parameter warning from `load_from_config` once the generator is trusted.

Phase 6: Clean up and future improvements

16. Consider whether `gui.toml` should remain the persistent storage format or whether a more compact runtime format is desired; in either case, keep `gui.toml` as the schema and default value source.
17. Optionally, extend the generated code to expose a static description table (array of param descriptors) to simplify GUI rendering further (for example, to support generic picking of widgets without string based lookup).
18. Add a minimal test (or example build) that fails if a new param is added to `gui.toml` but the generator is not updated to support its kind, to protect the reflection layer from silent drift.

Notes and constraints
---------------------

- Keep the refactor incremental: introduce code generation alongside the existing manual implementation, then switch call sites, and only then remove the manual code.
- Prefer straightforward, explicit generated code over highly abstract patterns; readability of the generated `GuiAdjustables` implementation matters for debugging.
- Avoid introducing additional manual mapping tables; the only place that should know the full parameter list is `config/gui.toml` plus the generator.
