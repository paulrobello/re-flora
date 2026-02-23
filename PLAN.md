# Plan: Wooden Staff Flora Regeneration

## Goal

When the wooden staff is used, reuse the shovel's edit origin and radius, but regenerate flora in the affected area using the same rules as initial placement. Existing flora instances must remain unchanged (no overwrites).

## Current Behavior (Baseline)

- Shovel: `try_shovel_dig` -> `apply_surface_terrain_removal` -> `mesh_generate_preserve_flora_for_sphere_edit`.
- Flora removal/edit path uses `edit_flora_instances.comp` (removes non-grass inside radius, keeps grass and resets growth tick).
- Initial flora placement uses `place_flora.comp` during `SurfaceBuilder::build_surface`.

## Plan

1. **Add staff input path**
   - Mirror shovel ray query logic (`query_camera_ray_terrain_intersection`) and timings for the staff slot.
   - Introduce a staff action handler (e.g., `try_staff_regenerate`) that uses the same origin and radius as shovel edits.

2. **Define a new flora regeneration edit path**
   - Create a new world operation akin to `mesh_generate_preserve_flora_for_sphere_edit` that triggers flora regeneration in a sphere without clearing existing instances.
   - Ensure this path uses the same placement rules as `place_flora.comp` (density/biome/plantability), but only for missing flora spots.

3. **Implement a regeneration compute pipeline**
   - Add a new compute shader (e.g., `regenerate_flora_instances.comp`) that:
     - Consumes current instance buffers and builds a "occupied" mask for the edit sphere.
     - Runs the same placement rules as `place_flora.comp` and adds new instances only where no existing flora is present.
   - Update `SurfaceBuilder` resources to include new pipeline buffers as needed (scratch/bitmask), keeping buffer sizes conservative.

4. **Wire regeneration into the builder**
   - Add a `SurfaceBuilder::regenerate_flora_instances` method that:
     - Takes chunk id, edit center, radius, and tick.
     - Preserves existing instances; appends any newly generated instances.
   - Integrate this into the new world op from step 2, iterating affected chunks.

5. **Hook staff usage to regeneration**
   - Use the staff slot to trigger the regeneration path on click/hold, reusing shovel timing and sound behavior where appropriate.
   - Confirm that shovel behavior remains unchanged.

6. **Verification**
   - Manual: dig with shovel to clear flora; use staff to restore flora in same radius and verify no existing flora is overwritten.
   - Confirm regenerated flora matches initial distribution patterns by comparing to untouched terrain.

## Files Likely Touched

- `src/app/core/input.rs` (staff input handling)
- `src/app/core/vegetation.rs` (new world op entry point)
- `src/app/world_ops.rs` (new regeneration op)
- `src/builder/surface/mod.rs` + `resources.rs` (new pipeline and buffers)
- `shader/builder/surface/` (new compute shader)
