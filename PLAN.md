# GUI config single source of truth plan

## Goal

Move all GUI config definitions out of Rust defaults and into one git tracked file at `config/gui.toml`.

The TOML file becomes the only source of truth for:

- section grouping
- parameter ids
- parameter kinds
- labels
- value bounds
- default values

Runtime behavior should panic fast with clear diagnostics when config is invalid.

## Scope

In scope:

- schema design for `config/gui.toml`
- strict loading and validation at startup
- refactor `GuiAdjustables` initialization to load from file
- remove hardcoded GUI ranges and labels from UI code

Out of scope for this phase:

- user specific config files
- merge layers or override priority
- save to disk feature

## File and module changes

1. Add `config/gui.toml` with full current config data.
2. Add `src/app/gui_config_model.rs` for serde data models.
3. Add `src/app/gui_config_loader.rs` for read plus validate logic.
4. Update `src/app/mod.rs` exports and module wiring.
5. Refactor `src/app/gui_config.rs` to stop carrying value metadata in code.
6. Refactor `src/gui_adjustables.rs` to construct runtime state from loaded metadata.
7. Update `src/app/core/mod.rs` UI controls to use metadata labels and ranges from loaded config.
8. Add tests for parse and validation behavior.

## Dependencies

Add to `Cargo.toml`:

- `serde` with derive
- `toml`

No user directory helper crates are needed.

## Config schema

Top level:

- `schema_version = 1`
- repeated `[[section]]` tables

Section table:

- `name` string
- repeated `[[section.param]]` tables

Param table fields:

- `id` string, globally unique
- `kind` enum string: `float`, `int`, `uint`, `bool`, `color`
- `label` string
- `value` typed by kind
- `min` and `max` required for `float`, `int`, `uint`
- `min` and `max` forbidden for `bool`, `color`

Color canonical format:

- hex string only
- allow `#RRGGBB` and `#RRGGBBAA`
- default alpha `FF` when only RGB is provided

## Validation rules

Validation should run once during startup and fail before render loop starts.

Required checks:

1. file exists and is readable
2. TOML parses successfully
3. `schema_version` is supported
4. section names are unique
5. param ids are unique globally
6. each param uses allowed fields for its kind
7. numeric params have `min <= max`
8. numeric `value` is in range
9. color matches canonical hex format

Error handling policy:

- collect all validation errors in one pass
- panic once with a detailed multi line message
- each line includes file path, section, param id, and reason

## Runtime representation

Use two layers:

1. file shaped model for deserialize
2. validated runtime model used by UI and systems

Runtime model should provide:

- section ordered parameter lists for rendering
- typed parameter containers for fast access
- id based index map for stable lookups

## UI integration

Replace duplicated hardcoded sliders and labels with metadata driven controls.

For each parameter:

- label from config
- widget kind from `kind`
- numeric bounds from `min` and `max`
- current value from runtime state

Keep current panel layout and behavior unless tied to hardcoded metadata.

## Migration steps

### step 1

Create `config/gui.toml` by copying current values from `src/app/gui_config.rs`.

### step 2

Implement model plus loader plus validator modules.

### step 3

Wire startup flow to load config before `GuiAdjustables` creation.

### step 4

Build `GuiAdjustables` from validated config.

### step 5

Remove remaining hardcoded UI bounds and labels in `src/app/core/mod.rs`.

### step 6

Delete obsolete hardcoded default paths after behavior parity is confirmed.

## Testing plan

Add tests for:

- valid config happy path
- missing file
- parse failure
- unsupported schema version
- duplicate section names
- duplicate param ids
- wrong type per kind
- out of range numeric value
- invalid color hex

Also run:

- `cargo check`
- relevant app tests if present

## Acceptance criteria

- app startup reads only `config/gui.toml` for GUI config definitions
- no hardcoded GUI value defaults remain in Rust for these params
- no hardcoded slider bounds or labels remain for these params
- invalid config causes panic with actionable diagnostics
- config is tracked in git and review friendly

## Risks and mitigations

Risk: broad refactor in `src/app/core/mod.rs` due to repeated inline controls.

Mitigation:

- keep changes incremental
- preserve existing control ids where possible
- validate behavior section by section

Risk: silent type drift between config and runtime access.

Mitigation:

- strict upfront validation
- typed runtime model and checked accessors
- focused tests for boundary and mismatch cases
