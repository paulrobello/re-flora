# PLAN

Currently, we are using a u8 chunk for occupancy data for the floras, where each stores either 0 or 1 indicating if there are flora on the given position.

However, our flora instances have a term storing the plant tick in a u32, so in the vert/frag stages we can use this data to determine the active height of the plant.

There's a need for us to update the occupancy stages, so it support carring that information.

Phase 1:

1. Upgrade occupancy texture format to u32
   - Change occupancy texture format from R8 to R32.
   - Update all occupancy image bindings from r8ui to r32ui.
   - Files to update:
     - `src/builder/surface/resources.rs` -> occupancy texture format.
     - `shader/builder/surface/clear_occupancy.comp`
     - `shader/builder/surface/instances_to_occupancy.comp`
     - `shader/builder/surface/edit_occupancy_sphere.comp`
     - `shader/builder/surface/occupancy_to_flora_instances.comp`

   Sentinel: 0x00000000 means no flora.

2. Encode growth tick in occupancy
    - Store `growth_start_tick + 1` in occupancy.
    - On read, treat 0 as empty, otherwise decode as `stored_value - 1`.
    - Wrap behavior is acceptable: if `growth_start_tick` is `u32::MAX`, the stored value will be 0 and treated as empty. We are not handling this edge case in phase 1.

3. Update compute passes to read and write ticks

   a. clear_occupancy
   - No behavior change, still writes 0.

   b. instances_to_occupancy
   - Replace the write value 1 with `instance.growth_start_tick + 1`.

    c. edit_occupancy_sphere (add and remove)
    - Removal: always write 0.
    - Addition: only write if current occupancy is 0, then write `flora_tick + 1`.
    - Addition must preserve existing non-zero occupancy values (read-before-write guard).
    - Add `flora_tick` into `U_EditOccupancyInfo` and update CPU-side builder in
      `src/builder/surface/mod.rs`.

    d. occupancy_to_flora_instances
    - Replace `instance.growth_start_tick = flora_tick` with decoded occupancy value.
   - Read `occupancy_value = imageLoad(occupancy_data).r`.
   - Skip if `occupancy_value == 0`.
   - Set `instance.growth_start_tick = occupancy_value - 1`.
    - Remove `flora_tick` from `U_OccupancyToInstancesInfo` and update CPU-side
      buffer layout accordingly.

4. Wire tick in regen paths
    - When regenerating, ensure the edit pass gets the current tick.
    - Update `update_occupancy_to_instances_info` to only write chunk offset and dimension
      after `flora_tick` removal.

5. Validate behavior
     - Confirm that add preserves existing occupancy values.
     - Confirm that remove clears to 0.
     - Confirm that regrowth preserves prior `growth_start_tick` values.
     - Confirm vertex shaders still compute age correctly using instance `growth_start_tick`.
     - Spot-check a regen edit: read back instances and verify `growth_start_tick` matches
       decoded occupancy (`stored_value - 1`) for a few points.

Phase 1 implementation checklist:

1. `src/builder/surface/resources.rs`
   - Change `occupancy_desc.format` from `vk::Format::R8_UINT` to `vk::Format::R32_UINT`.

2. `shader/builder/surface/clear_occupancy.comp`
   - Change occupancy image binding format from `r8ui` to `r32ui`.
   - Keep clearing value as `uvec4(0u, 0u, 0u, 0u)`.

3. `shader/builder/surface/instances_to_occupancy.comp`
   - Change occupancy image binding format from `r8ui` to `r32ui`.
   - Replace occupancy write `1u` with `instance.growth_start_tick + 1u`.

4. `shader/builder/surface/edit_occupancy_sphere.comp`
   - Change occupancy image binding format from `r8ui` to `r32ui`.
   - Add `uint flora_tick;` to `U_EditOccupancyInfo`.
   - For remove mode: always store `0u`.
   - For add mode:
     - Read current occupancy at `stem_local`.
     - Only write when current occupancy is `0u`.
     - Write `flora_tick + 1u`.

5. `src/builder/surface/mod.rs`
   - Update `update_edit_occupancy_info(...)` signature to accept `flora_tick: u32`.
   - Write `flora_tick` into `U_EditOccupancyInfo`.
   - Pass `flora_tick` to `update_edit_occupancy_info(...)` from:
     - `seed_and_rebuild_flora_from_surface(...)`
     - `run_occupancy_edit(...)`
   - In `run_occupancy_edit(...)`, stop ignoring `_flora_tick`; use it as `flora_tick`.

6. `shader/builder/surface/occupancy_to_flora_instances.comp`
   - Remove `uint flora_tick;` from `U_OccupancyToInstancesInfo`.
   - Change occupancy image binding format from `r8ui` to `r32ui`.
   - Read `occupancy_value = imageLoad(occupancy_data, stem_local).r`.
   - Skip when `occupancy_value == 0u`.
   - Set `instance.growth_start_tick = occupancy_value - 1u`.

7. `src/builder/surface/mod.rs` (occupancy to instances info path)
   - Update `update_occupancy_to_instances_info(...)` to remove `flora_tick` parameter.
   - Stop setting `flora_tick` field in the CPU-side struct writer.
   - Update all call sites to pass only chunk offset and chunk dimension.

8. Runtime checks after implementation
   - Remove in sphere: edited region occupancy becomes 0, non-edited region unchanged.
   - Add in sphere: pre-existing non-zero occupancy is preserved.
   - Regen after remove+add: restored instances keep per-instance decoded tick values.
   - Visual check: plant age/height progression still behaves correctly in render.

Phase 2:

Definition:

- `growth_start_tick` means the world tick when this instance started growing.
  Age in ticks is `current_flora_tick - growth_start_tick`.

Phase 2 implementation plan:

1. Tool 2: add flora to existing terrain (same height as init flora)
   - Behavior: when adding new flora, set `growth_start_tick` so its age matches init flora.
   - Source of truth: `FLORA_FULL_GROWTH_TICKS` in `src/app/core/mod.rs`.
   - Use `growth_start_tick = current_flora_tick - FLORA_FULL_GROWTH_TICKS`.
     - Use wrapping subtract (matches existing tick behavior) unless explicitly clamped.
   - This ensures the new flora renders at full growth like init flora.

2. Tool 3: grass trimming tool (new toolbox slot)
   - Add a third tool icon in the bottom toolbox, slot index 2.
   - Icon asset: `assets/texture/Pixel_Farming_Tools_IconSet_16px/Individuals/11_Wooden_Hoe.PNG`.
   - Selection and input handling should mirror shovel/staff patterns.

3. Shared radius
   - The trimming tool uses the same radius and region shape as tool 1 and tool 2.

4. Trimming behavior (occupancy updates)
   - Goal: trim only older flora, never increase height.
   - For each affected cell:
     - Read `occupancy_value` (0 means empty).
     - Skip if empty.
     - Decode `growth_start_tick = occupancy_value - 1`.
     - Compute `current_age = current_flora_tick - growth_start_tick` (wrapping ok).
     - Only trim if `current_age > target_age`.
     - If trim applies, set new `growth_start_tick = current_flora_tick - target_age`.
     - Underflow behavior: clamp `current_flora_tick - target_age` to 0 (saturating subtract).
     - Store back as `new_growth_start_tick + 1`.

5. Target age
   - `target_age = FLORA_FULL_GROWTH_TICKS / 2`.
   - With current constant this is `15` ticks.

6. Shader + CPU wiring
   - Extend edit/occupancy compute path to support a trim mode:
     - Add mode enum/flag for trim.
     - Add `current_flora_tick` and `target_age` to the edit info buffer.
   - Use the same occupancy edit shader and branching, or create a dedicated trim shader if cleaner.

7. Validation checks
   - Trim does not add flora to empty cells.
   - Trim does not affect younger flora (age <= target_age).
   - Trim clamps underflow correctly.
   - Radius matches tools 1 and 2.
   - Icon appears in slot 3 and selection works.
