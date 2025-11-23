# Coding Plans

## Tree Audio & Wind Roadmap

- Tree audio sources now flow through `TreeAudioManager`, which stores `{uuid, tree_id, position, cluster_size}` for every looping emitter.
- Spatial playback still handled by `SpatialSoundManager` (PetalSonic), but higher-level logic (wind modulation, clustering heuristics) can act on the cached metadata without engine round-trips.

### Near-Term Tasks

1. **Global clustering pass**  
   - Instead of clustering per tree, collect all leaf centroids (or existing per-tree clusters) and merge them across tree boundaries using a spatial hash.  
   - Keep metadata so we can later split clusters if trees move or get culled.  
   - Experiment with listener-distance weighting: close to the player, use a smaller merge radius; far away, increase it to keep source counts manageable.

2. **Distance-aware reclustering**  
   - Given cached positions, recompute `desired_cluster_distance` as `base_dist * lerp(near_factor, far_factor, distance_to_listener / falloff)`.  
   - Consider supplementary heuristics: angle between listener forward vector and `(source_pos - listener_pos)`, distance buckets, or even terrain occlusion tests.  
   - Evaluate whether dynamically morphing cluster distances produces noticeable popping; if so, introduce crossfades when merging/splitting emitters.

3. **Wind modulation system**  
   - Treat each emitter’s `(x, z)` as its deterministic “seed”. Sample a 3D noise function `noise(time * freq, pos.x * scale, pos.z * scale)` to obtain smooth gust envelopes that sweep coherently across space.  
   - Map noise output to ±N dB gain adjustments and optionally drive subtle positional offsets (`update_source_pos`) to simulate branches swaying.  
   - Allow global wind parameters (direction, speed, strength) to influence noise phase and bias (e.g., sources downwind respond slightly earlier).

4. **Manager → PetalSonic utilities**  
   - Add helper APIs to `SpatialSoundManager` for bulk gain/pose updates so the wind system can submit batched adjustments without excessive locking.  
   - Investigate exposing “query pose” hooks inside PetalSonic so the manager can double-check engine state if needed (otherwise keep relying on cached positions).

5. **Future integration**  
   - If the clustering + wind workflow proves general, upstream a `TreeAmbienceSystem` module into `petalsonic` so other projects can reuse it.  
   - Document tuning knobs (cluster radius, noise speed/amplitude, distance falloff) for designers.

### Open Questions

- Is clustering distance alone the right abstraction, or should we move toward perceptual metrics that include listener orientation and binaural cues?  
- How aggressive can cross-tree clustering be before the spatial image feels too smeared? Need usability tests with various tree densities.  
- Should wind also influence other ambient layers (grass, shrub rustle) via the same noise field for coherence?

## Flora Variant System Session (2025-11-23)

### Observations

- `src/tracer/flora_construct.rs:5` keeps two hard-coded mesh generators (`gen_grass` iterates vertically, `gen_lavender` adds a stem plus spherical bloom), and `TracerResources` wires both via `FloraMeshResources::new(..., gen_grass/gen_lavender)`.
- `shader/builder/surface/make_surface.comp:18-250` exposes separate counters/buffers (`grass_instance_len`, `lavender_instance_len`, `manual_grass_instances`, `manual_lavender_instances`) and branches into `add_grass_instance` vs `add_lavender_instance` based on a fixed noise threshold.
- `src/builder/surface/resources.rs:12-99` models flora with a `FloraType` enum and a hash map per chunk, forcing every new species to add enum variants, buffer allocations, and descriptor bindings (`SurfaceBuilder::update_grass_instance_set` only knows about Grass/Lavender at `src/builder/surface/mod.rs:88-101`).
- Rendering repeats the same loops for each type: `TracerResources` stores two copies of mesh resources (plus LOD) and `Tracer::record_flora_pass` is invoked four times per frame (Lod0/1 × Grass/Lavender) with manual match arms at `src/tracer/mod.rs:870-881`.

### Goal

Support an arbitrary number of plant species (meshes + placement rules + colors) with minimal boilerplate by making the flora system data-driven end-to-end (definition, instance emission, GPU buffers, and rendering).

### Plan

1. **Define a central registry:** Create a `flora::species` module that exposes a `static FLORA_SPECIES: &[FloraSpeciesDesc]`. Each descriptor holds ids/names, mesh generator fn pointers (both high-detail and lod), default GUI color handles, placement parameters (noise seeds, density thresholds, allowed terrain mask), and render flags. This becomes the single source of truth for Rust and shaders (export constants via `build.rs` or generated GLSL include).
2. **Generalize CPU resources:**
   - Replace the `FloraType` enum and hash map with an indexed structure (`Vec<InstanceResource>` ordered like `FLORA_SPECIES`), plus helper methods to fetch by species id.
   - Update `SurfaceResources::instances`, `SurfaceBuilder::update_*`, and `make_surface_result` buffer layouts to allocate/bind instance buffers/counts in loops rather than per-type fields. Descriptor set 1 for `make_surface` should bind an array of SSBOs (one per species) or a single large buffer segmented by offsets that are passed through the species table.
   - In `TracerResources`, iterate over the registry to instantiate `FloraMeshResources` (and LOD variants) into vectors; provide accessor methods so rendering code can iterate without new match arms.
3. **Shader placement rewrite:**
   - Introduce `layout(std430, set=0, binding=1) buffer B_MakeSurfaceResult { uint active_voxel_len; uint instance_len[MAX_FLORA_SPECIES]; }` plus `layout(set=1, binding=0) buffer B_ManualFloraInstances { Instance data[]; } manual_instances[MAX_FLORA_SPECIES];`.
   - Replace `add_grass_instance/add_lavender_instance` with `add_flora_instance(uint species_idx, uint variant)` that atomically increments `instance_len[species_idx]`. Placement selection becomes data-driven by reading `FloraSpeciesPlacementParams` (seed, density curve, biome tags) from another SSBO/UBO generated off the registry instead of hard-coded noise numbers.
   - Export `MAX_FLORA_SPECIES` + placement params to GLSL via a generated include so the shader can loop through species definitions or evaluate weighted randomness as needed.
4. **Rendering loop simplification:**
   - Store mesh buffers + color controls inside each descriptor; `record_flora_pass` becomes a single loop over species with `if species.render_pass == FloraRenderPass::Flora` etc., issuing draws for whichever species have instances per chunk. Push constants remain per draw but colors now come from species-configured GUI values.
   - GUI/`app::gui_config` should query `FLORA_SPECIES` to build color params dynamically so adding a species only edits the registry.
5. **Authoring workflow:**
   - Provide helper CLI or docs for adding a new species: implement `fn gen_<species>` or load from asset, append a descriptor entry, add placement defaults. Shader + CPU automatically pick it up via generated constants.
   - Add validation (unit tests or asserts) ensuring `FLORA_SPECIES.len()` does not exceed the compile-time `MAX_FLORA_SPECIES` used by GLSL and that per-species instance capacities fit current buffer sizes.

### Open Questions

- Descriptor array vs single large buffer: confirm whether current Vulkan/GLSL setup supports `manual_instances[MAX]`; otherwise fall back to one unified buffer with prefix sums per species. -> See if we have used that feature somewhere else, if so, use that, if not, use a single large buffer with prefix sums per species.
- How should biome/tag-based placement rules be expressed so they stay data-driven yet efficient inside `make_surface.comp`? (e.g., bitmasks derived from voxel types vs height ranges.) -> Use bitmask for now, cause we don't have height differentiation yet for the terrain generation.
- Do we ever need per-species material/shader differences (e.g., vertex shader variation) that would require pipeline specialization instead of pushing colors? -> No.
