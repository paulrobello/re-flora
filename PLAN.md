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

## Add other flora types

see how i differentiate between the generation of grass blade / lavender. and analyze and plan for how to extend this logic, so i can have arbitary number of types of plants

 Now I have a complete understanding of the architecture. Let me summarize my analysis and present a plan:

  Analysis Summary

  Current Architecture

  1. Plant Type Definition (src/builder/surface/resources.rs:11-15):
  pub enum FloraType {
      Grass,
      Lavender,
  }

  2. Geometry Generation (src/tracer/flora_construct.rs):

- gen_grass() - 8 vertical voxels with linear color/wind gradient
- gen_lavender() - 6-voxel stem + spherical leaf ball (radius 2), power curve for color

  3. Resource Storage (src/tracer/resources.rs):

- Separate structs: GrassBladeResources, LavenderResources (duplicated code)
- Both LOD0 and LOD1 versions stored

  4. Instance Buffers (src/builder/surface/resources.rs:62-92):

- FloraInstanceResources uses HashMap<FloraType, InstanceResource>
- Manually inserts each type in constructor

  5. Surface Shader (shader/builder/surface/make_surface.comp:177-251):

- Hardcoded add_grass_instance() and add_lavender_instance() functions
- Noise-based selection: density_noise < 0.603 → lavender, else grass

  6. Render Loop (src/tracer/mod.rs:675-712):

- 4 separate record_flora_pass() calls (2 LODs × 2 plant types)
- Manual match on FloraType to select geometry buffers

  Pain Points for Extensibility

  1. Enum requires code changes - Adding a plant type requires modifying FloraType enum
  2. Duplicated resource structs - GrassBladeResources and LavenderResources are identical
  3. Hardcoded shader functions - Each plant needs its own add_xxx_instance() function
  4. Manual HashMap population - Must add each type explicitly
  5. Render call explosion - Each new plant adds 2 more render calls (LOD0 + LOD1)

  ---
  Plan for Arbitrary Plant Types
  ⎿  User rejected Claude's plan:
     ╭────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
     │ Plan: Extensible Plant Type System                                                                         │
     │                                                                                                            │
     │ Phase 1: Data-Driven Plant Definition                                                                      │
     │                                                                                                            │
     │ 1. Create PlantTypeId enum with registry pattern:                                                          │
     │ // src/builder/surface/flora_types.rs                                                                      │
     │ #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]                                                         │
     │ #[repr(u32)]                                                                                               │
     │ pub enum PlantTypeId {                                                                                     │
     │     Grass = 0,                                                                                             │
     │     Lavender = 1,                                                                                          │
     │     // Future: Flower, Shrub, Fern, etc.                                                                   │
     │ }                                                                                                          │
     │                                                                                                            │
     │ pub const ALL_PLANT_TYPES: &[PlantTypeId] = &[PlantTypeId::Grass, PlantTypeId::Lavender];                  │
     │                                                                                                            │
     │ 2. Unify geometry resources into generic struct:                                                           │
     │ // Replace GrassBladeResources + LavenderResources with:                                                   │
     │ pub struct FloraGeometry {                                                                                 │
     │     pub vertices: Resource<Buffer>,                                                                        │
     │     pub indices: Resource<Buffer>,                                                                         │
     │     pub indices_len: u32,                                                                                  │
     │ }                                                                                                          │
     │                                                                                                            │
     │ // In TracerResources:                                                                                     │
     │ pub flora_geometries: HashMap<PlantTypeId, FloraGeometry>,                                                 │
     │ pub flora_geometries_lod: HashMap<PlantTypeId, FloraGeometry>,                                             │
     │                                                                                                            │
     │ 3. Refactor geometry generation to trait/registry:                                                         │
     │ pub type FloraGenerator = fn(bool) -> Result<(Vec<Vertex>, Vec<u32>)>;                                     │
     │                                                                                                            │
     │ pub fn get_flora_generator(plant_type: PlantTypeId) -> FloraGenerator {                                    │
     │     match plant_type {                                                                                     │
     │         PlantTypeId::Grass => gen_grass,                                                                   │
     │         PlantTypeId::Lavender => gen_lavender,                                                             │
     │     }                                                                                                      │
     │ }                                                                                                          │
     │                                                                                                            │
     │ Phase 2: Shader-Side Extensibility                                                                         │
     │                                                                                                            │
     │ 4. Create flora_instance_buffers.glsl include:                                                             │
     │ // shader/include/flora_types.glsl                                                                         │
     │ #define FLORA_TYPE_GRASS    0                                                                              │
     │ #define FLORA_TYPE_LAVENDER 1                                                                              │
     │ #define FLORA_TYPE_COUNT    2                                                                              │
     │                                                                                                            │
     │ // shader/include/flora_instance_buffers.glsl                                                              │
     │ layout(set = X, binding = Y) buffer FloraInstances_Grass { Instance data[]; } flora_instances_grass;       │
     │ layout(set = X, binding = Y) buffer FloraInstances_Lavender { Instance data[]; } flora_instances_lavender; │
     │                                                                                                            │
     │ void add_flora_instance(ivec3 uvi, uint flora_type, uint sub_type) {                                       │
     │     Instance instance;                                                                                     │
     │     instance.pos = uvec3(make_surface_info.atlas_read_offset + uvi) + uvec3(0, 1, 0);                      │
     │     instance.ty = sub_type;                                                                                │
     │                                                                                                            │
     │     if (flora_type == FLORA_TYPE_GRASS) {                                                                  │
     │         uint idx = atomicAdd(make_surface_result.grass_instance_len, 1);                                   │
     │         flora_instances_grass.data[idx] = instance;                                                        │
     │     } else if (flora_type == FLORA_TYPE_LAVENDER) {                                                        │
     │         uint idx = atomicAdd(make_surface_result.lavender_instance_len, 1);                                │
     │         flora_instances_lavender.data[idx] = instance;                                                     │
     │     }                                                                                                      │
     │ }                                                                                                          │
     │                                                                                                            │
     │ 5. Parameterize plant selection noise:                                                                     │
     │ // Selection based on weighted ranges instead of hardcoded thresholds                                      │
     │ float selection_noise = fnlGetNoise2D(selection_state, uvi.x, uvi.z) * 0.5 + 0.5;                          │
     │ uint selected_type = select_flora_type(selection_noise); // maps noise → type                              │
     │ add_flora_instance(uvi, selected_type, GRASS_TYPE_NORMAL);                                                 │
     │                                                                                                            │
     │ Phase 3: Render Loop Generalization                                                                        │
     │                                                                                                            │
     │ 6. Loop over plant types instead of manual calls:                                                          │
     │ fn record_all_flora_passes(&self, cmdbuf: &CommandBuffer, ...) {                                           │
     │     for &plant_type in ALL_PLANT_TYPES {                                                                   │
     │         let (bottom_color, tip_color) = self.get_plant_colors(plant_type, gui_config);                     │
     │                                                                                                            │
     │         self.record_flora_pass(cmdbuf, &chunks_lod0, LodState::Lod0, plant_type, ...);                     │
     │         self.record_flora_pass(cmdbuf, &chunks_lod1, LodState::Lod1, plant_type, ...);                     │
     │     }                                                                                                      │
     │ }                                                                                                          │
     │                                                                                                            │
     │ 7. Generic flora pass with HashMap lookup:                                                                 │
     │ fn record_flora_pass(&self, ..., plant_type: PlantTypeId, ...) {                                           │
     │     let geometry = match lod_state {                                                                       │
     │         LodState::Lod0 => &self.resources.flora_geometries[&plant_type],                                   │
     │         LodState::Lod1 => &self.resources.flora_geometries_lod[&plant_type],                               │
     │     };                                                                                                     │
     │     // Use geometry.vertices, geometry.indices, geometry.indices_len                                       │
     │ }                                                                                                          │
     │                                                                                                            │
     │ Phase 4: GUI Color Configuration                                                                           │
     │                                                                                                            │
     │ 8. Generalize color config in gui_config.rs:                                                               │
     │ pub struct PlantColorConfig {                                                                              │
     │     pub bottom_color: Color32,                                                                             │
     │     pub tip_color: Color32,                                                                                │
     │ }                                                                                                          │
     │                                                                                                            │
     │ pub flora_colors: HashMap<PlantTypeId, PlantColorConfig>,                                                  │
     │                                                                                                            │
     │ Files to Modify                                                                                            │
     │                                                                                                            │
     │ 1. src/builder/surface/resources.rs - Refactor FloraType → PlantTypeId, loop-based init                    │
     │ 2. src/tracer/flora_construct.rs - Add generator registry                                                  │
     │ 3. src/tracer/resources.rs - Unify into FloraGeometry, use HashMap                                         │
     │ 4. src/tracer/mod.rs - Loop over plant types in render                                                     │
     │ 5. shader/builder/surface/make_surface.comp - Parameterize plant selection                                 │
     │ 6. shader/include/flora_types.glsl - New file for type constants                                           │
     │ 7. src/app/gui_config.rs - Generalize color configs                                                        │
     │                                                                                                            │
     │ Adding a New Plant Type (After Refactor)                                                                   │
     │                                                                                                            │
     │ 1. Add variant to PlantTypeId enum                                                                         │
     │ 2. Add to ALL_PLANT_TYPES constant                                                                         │
     │ 3. Implement gen_xxx() function                                                                            │
     │ 4. Add generator to registry match                                                                         │
     │ 5. Add color config defaults                                                                               │
     │ 6. Update shader FLORA_TYPE_COUNT and selection logic                                                      │
     │                                                                                                            │
     │ No structural code changes needed - just additions!                                                        │
     ╰────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
