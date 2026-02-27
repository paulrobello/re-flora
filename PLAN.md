# PLAN

Currently, we are using a u8 chunk for occupancy data for the floras, where each stores either 0 or 1 indicating if there are flora on the given position.

However, our flora instances have a term storing the plant tick in a u32, so in the vert/frag stages we can use this data to determine the active height of the plant.

There's a need for us to update the occupancy stages, so it support carring that information.

Implementation plan:

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

3. Update compute passes to read and write ticks

   a. clear_occupancy

      - No behavior change, still writes 0.

   b. instances_to_occupancy

      - Replace the write value 1 with `instance.growth_start_tick + 1`.

   c. edit_occupancy_sphere (add and remove)

      - Removal: always write 0.
      - Addition: only write if current occupancy is 0, then write `flora_tick + 1`.
      - Add `flora_tick` into `U_EditOccupancyInfo` and update CPU-side builder in
        `src/builder/surface/mod.rs`.

   d. occupancy_to_flora_instances

      - Replace `instance.growth_start_tick = flora_tick` with decoded occupancy value.
      - Read `occupancy_value = imageLoad(occupancy_data).r`.
      - Skip if `occupancy_value == 0`.
      - Set `instance.growth_start_tick = occupancy_value - 1`.
      - Remove `flora_tick` from `U_OccupancyToInstancesInfo` if no longer needed,
        and update CPU-side buffer layout accordingly.

4. Wire tick in regen paths

   - When regenerating, ensure the edit pass gets the current tick.
   - If `flora_tick` is removed from `U_OccupancyToInstancesInfo`, update
     `update_occupancy_to_instances_info` to only write chunk offset and dimension.

5. Validate behavior

   - Confirm that add preserves existing occupancy values.
   - Confirm that remove clears to 0.
   - Confirm that regrowth preserves prior `growth_start_tick` values.
   - Confirm vertex shaders still compute age correctly using instance `growth_start_tick`.

DO IT LATER:

Add a third tool to the bottom toolbox, that functions as a trim tool for the flora.

This tool is used to trim affacted region to a flora age, which is effectivelly setting the tick stored in occupancy data to: current_tick - target_age

This gives a target_age to all affacted regions
