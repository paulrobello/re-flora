# CPU-GPU data flow redesign

## Problem statement

The current CPU-to-GPU data flow relies on runtime SPIR-V reflection with a
string-based builder API. While this avoids manual `#[repr(C)]` struct
declarations for most buffers, it introduces a different class of boilerplate
and fragility that scales poorly as the project grows.

## Current architecture

GLSL is the single source of truth for uniform/storage buffer layouts. At
startup, shaders are compiled to SPIR-V twice (zero-opt for reflection,
full-opt for execution). `spirv-reflect` extracts `BufferLayout` trees,
and buffers are created from those layouts at runtime.

Data upload uses `StructMemberDataBuilder`, a string-keyed setter that
serializes fields into correct std140/std430 bytes:

```rust
// every single field requires this ceremony
let data = StructMemberDataBuilder::from_buffer(&resources.sun_info)
    .set_field("sun_dir", PlainMemberTypeWithData::Vec3(sun_dir.to_array()))
    .set_field("sun_size", PlainMemberTypeWithData::Float(sun_size))
    // ... repeat for every field
    .build()?;
resources.sun_info.fill_with_raw_u8(&data)?;
```

Descriptor binding is automated via `#[derive(ResourceContainer)]` and
`auto_update_descriptor_sets`, which matches Rust field names to GLSL binding
names. This part works well.

## Concrete problems

### 1. Verbose field-by-field serialization

`buffer_updater.rs` is 417 lines of almost pure mechanical boilerplate.
Every field requires `.set_field("name", PlainMemberTypeWithData::Type(val))`.
The type tag (`Float`, `Vec3`, `Mat4`, etc.) is redundant information since
reflection already knows the type; the caller is forced to restate it.

### 2. No compile-time safety

Field names are strings. A typo like `"sun_colour"` instead of `"sun_color"`
compiles fine and only fails at runtime. Type mismatches (passing `Float` where
`UInt` is expected) are also runtime errors. Removed or renamed shader fields
silently become dead code or runtime panics.

### 3. Parameter explosion through the call chain

`Tracer::update_buffers()` (`src/tracer/mod.rs:445`) takes 60+ parameters,
passes them to `BufferUpdater` functions which each take many parameters, which
then pass them one-by-one to `set_field`. The same data is named and passed
through three layers. Adding a single field to a GLSL buffer requires touching
the shader, the updater function, the caller, and sometimes the GUI config.

### 4. Dual declarations for vertex/instance/push-constant types

Several types are manually defined on both sides:

| Rust                                              | GLSL                                          | Note                     |
| ------------------------------------------------- | --------------------------------------------- | ------------------------ |
| `Instance` (`builder/surface/resources.rs:17`)    | `Instance` (`shader/include/instance.glsl:4`) | Has a TODO about this    |
| `PushConstantStd140` (`tracer/mod.rs:80`)         | push constant block in vertex shaders         | Manual `_padding` fields |
| `ParticleInstanceGpu` (`tracer/resources.rs:145`) | vertex input attributes                       | Implicit layout coupling |
| `Vertex` (`tracer/vertex.rs`)                     | `in uvec2 in_packed_data`                     | Packed format coupling   |

These will silently diverge when one side is updated without the other.

### 5. Buffer creation boilerplate

Every reflected buffer requires ~7 lines of identical ceremony
(`src/tracer/resources.rs:287-424`): get layout from shader module, call
`Buffer::from_buffer_layout` with the same `device`, `allocator`,
`BufferUsage::empty()`, `MemoryLocation::CpuToGpu` args. Repeated 15 times
in `TracerResources::new()` alone.

## Proposed solution: build-time codegen from GLSL

The core idea: **parse GLSL buffer declarations at build time and generate
`#[repr(C)]` Rust structs with correct std140/std430 layout, so that CPU-side
buffer filling becomes a single `buffer.fill(&my_struct)` call.**

GLSL remains the source of truth. No new definition language. The runtime
reflection system stays for descriptor auto-binding (it already works well).
Code generation handles only the typed data upload path.

### How it works

```
build.rs
  |
  +--> compile .glsl to SPIR-V (shaderc, zero-opt)
  +--> reflect SPIR-V (spirv-reflect)
  +--> for each uniform/storage buffer block, emit a Rust struct
  +--> write to src/generated/gpu_structs.rs (or OUT_DIR)
```

For a shader declaring:

```glsl
layout(set = 0, binding = 1) uniform U_SunInfo {
    vec3 sun_dir;
    float sun_size;
    vec3 sun_color;
    float sun_luminance;
    float sun_display_luminance;
    float sun_altitude;
    float sun_azimuth;
} sun_info;
```

The codegen produces:

```rust
/// Auto-generated from `U_SunInfo` in shader/tracer/tracer.comp
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SunInfo {
    pub sun_dir: [f32; 3],
    pub sun_size: f32,      // std140: vec3 padded to 16 bytes, float fills the gap
    pub sun_color: [f32; 3],
    pub sun_luminance: f32,
    pub sun_display_luminance: f32,
    pub sun_altitude: f32,
    pub sun_azimuth: f32,
    pub _pad0: [u8; 4],     // std140 rounding to 16-byte multiple
}
```

Padding bytes and field offsets are derived directly from the reflected
`offset` and `padded_size` values in `PlainMemberLayout`, so the generated
struct is guaranteed to match the GPU layout bit-for-bit.

### Before vs. after

**Before** (current `buffer_updater.rs`):

```rust
pub fn update_sun_info(
    resources: &TracerResources,
    sun_dir: Vec3,
    sun_size: f32,
    sun_color: Vec3,
    sun_luminance: f32,
    sun_display_luminance: f32,
    sun_altitude: f32,
    sun_azimuth: f32,
) -> Result<()> {
    let data = StructMemberDataBuilder::from_buffer(&resources.sun_info)
        .set_field("sun_dir", PlainMemberTypeWithData::Vec3(sun_dir.to_array()))
        .set_field("sun_size", PlainMemberTypeWithData::Float(sun_size))
        .set_field("sun_color", PlainMemberTypeWithData::Vec3(sun_color.to_array()))
        .set_field("sun_luminance", PlainMemberTypeWithData::Float(sun_luminance))
        .set_field("sun_display_luminance", PlainMemberTypeWithData::Float(sun_display_luminance))
        .set_field("sun_altitude", PlainMemberTypeWithData::Float(sun_altitude))
        .set_field("sun_azimuth", PlainMemberTypeWithData::Float(sun_azimuth))
        .build()?;
    resources.sun_info.fill_with_raw_u8(&data)?;
    Ok(())
}
```

**After**:

```rust
let sun = gpu::SunInfo {
    sun_dir: sun_dir.to_array(),
    sun_size,
    sun_color: sun_color.to_array(),
    sun_luminance,
    sun_display_luminance,
    sun_altitude,
    sun_azimuth,
    ..bytemuck::Zeroable::zeroed()
};
resources.sun_info.fill(&sun)?;
```

- No string field names. Typos are compile errors.
- No `PlainMemberTypeWithData` wrappers. Types are enforced by the struct.
- No `StructMemberDataBuilder` ceremony. Just construct and fill.
- Padding is generated, not hand-written.

### What this fixes for each problem

| Problem                     | Solution                                                                                |
| --------------------------- | --------------------------------------------------------------------------------------- |
| Verbose serialization       | Struct construction replaces builder chain                                              |
| No compile-time safety      | Struct fields are statically typed; renamed/removed shader fields break the build       |
| Parameter explosion         | Pass the generated struct directly instead of 20 loose args                             |
| Dual declarations           | `Instance`, push constants, etc. are generated from GLSL; delete the manual Rust copies |
| Buffer creation boilerplate | Unchanged (already acceptable), but `from_buffer_layout` could gain a shorthand         |

### Scope of generated structs

**Generate for:**

- All `uniform` buffer blocks (`U_CameraInfo`, `U_SunInfo`, `U_GuiInput`, etc.)
- All `buffer` (storage) blocks where the Rust side writes data
  (`B_PlayerCollisionResult`, etc.)
- Shared struct types used in both vertex attributes and storage buffers
  (`Instance`)
- Push constant blocks

**Do not generate for:**

- GPU-only storage buffers that the CPU never touches (e.g. `B_ContreeNodeData`
  which is filled by compute shaders)
- Image/sampler bindings (handled by textures, no struct needed)

### Naming convention

The GLSL type name `U_CameraInfo` maps to Rust struct `CameraInfo`.
The prefix (`U_` for uniform, `B_` for buffer) is stripped. If a name
collision occurs (unlikely), the full name is kept.

All generated structs live in a single module: `src/generated/gpu_structs.rs`
(or under `$OUT_DIR` if we want to keep them out of source control).

### Deduplication of identical buffer blocks

Multiple shaders may declare identical blocks (e.g. `U_CameraInfo` appears in
`tracer.comp`, `flora.vert`, `composition.comp`, etc.). The codegen should:

1. Collect all blocks across all shaders
2. Group by type name
3. Verify that identically-named blocks have identical layouts (error if not)
4. Emit each struct exactly once

### `Buffer::fill` typed path

`Buffer` already has `fill<T: Pod>(&self, data: &[T])`. For single-struct
uniform buffers, a convenience method is useful:

```rust
impl Buffer {
    pub fn fill_uniform<T: Pod>(&self, value: &T) -> Result<()> {
        self.fill_with_raw_u8(bytemuck::bytes_of(value))
    }
}
```

### What stays the same

- **Runtime reflection for descriptor binding**: `ResourceContainer`,
  `auto_update_descriptor_sets`, and the `manual_` prefix convention all work
  well. No changes needed.
- **Shader hot-reloading** (if ever added): runtime reflection is the right
  tool for that. The two systems can coexist.
- **`StructMemberDataBuilder`**: keep it available for dynamic/debug use cases
  (e.g. GUI-driven field tweaking), but it stops being the primary data upload
  path.
- **`data_reader.rs`**: still useful for GPU readback of reflected buffers.

## Implementation plan

## Progress todo list

### Completed

- [x] **Phase 1.1** add `shaderc` and `spirv-reflect` to `[build-dependencies]` in `Cargo.toml`.
- [x] **Phase 1.2** implement build-time GPU struct codegen in `build.rs`:
  - shader compile for selected `.vert` and `.comp` files
  - SPIR-V reflection for uniform and storage buffers
  - dedup by GLSL type name
  - emit `src/generated/gpu_structs.rs`
  - emit explicit `_padN` fields from reflected `offset` and `padded_size`
- [x] **Phase 1.3** add generated module wiring:
  - `src/generated/mod.rs`
  - `mod generated;` in `src/main.rs`
- [x] **Phase 1.4** add `Buffer::fill_uniform<T: bytemuck::Pod>(&self, value: &T)`.
- [x] **Phase 1.5** verify compile and layout generation (`cargo check` passes).
- [x] **Phase 2.1** migrate `src/tracer/buffer_updater.rs` from
      `StructMemberDataBuilder` to generated typed structs + `fill_uniform`.
- [x] **Phase 2.2** migrate terrain query count write path in `src/tracer/mod.rs`
      to generated `TerrainQueryCount`.
- [x] **Phase 2.3** migrate player collision result read path in
      `src/tracer/mod.rs` to generated `PlayerCollisionResult` + bit-cast for
      ring distances.

### In progress

- [ ] **Phase 2.4** remove remaining `StructMemberDataBuilder` and
      `PlainMemberTypeWithData` usage from non-tracer systems:
  - `src/builder/contree/mod.rs`
  - `src/builder/plain/mod.rs`
  - `src/builder/scene_accel/mod.rs`
  - `src/builder/surface/mod.rs`

### Not started

- [ ] **Phase 2.5** evaluate deleting `src/tracer/buffer_updater.rs` (or keep as
      thin typed wrapper) after full migration in builders.
- [ ] **Phase 3.1** generate `Instance` from shader source and remove manual Rust
      `Instance` in `src/builder/surface/resources.rs`.
- [ ] **Phase 3.2** generate push constant structs and remove manual
      `PushConstantStd140` in `src/tracer/mod.rs`.
- [ ] **Phase 3.3** audit `ParticleInstanceGpu` and `Vertex` for optional
      generation.
- [ ] **Phase 4.1** optional shorthand `Buffer::new_uniform::<T>(...)` to reduce
      `Buffer::from_buffer_layout` boilerplate in resource constructors.

### Commits created so far

- `f2807dd2` add build-time gpu struct codegen from GLSL shader reflection
- `2ef34fba` migrate buffer updater to generated gpu structs
- `b608f27b` migrate terrain_query_count and player_collision_result to generated structs

### Phase 1: build-time codegen infrastructure

1. Add GLSL-to-SPIR-V compilation in `build.rs` (reuse the same `shaderc`
   setup from `compiler.rs`, zero-opt only).
2. Add `spirv-reflect` as a build dependency.
3. Walk all `.vert`, `.frag`, `.comp` files under `shader/`.
4. For each reflected buffer block, emit a `#[repr(C)]` struct with correct
   offsets and padding.
5. Write the output to `src/generated/gpu_structs.rs`.
6. Add `mod generated;` to `main.rs`.

### Phase 2: migrate buffer updates

1. Replace `buffer_updater.rs` functions one at a time, starting with simple
   buffers like `U_EnvInfo` (1 field) and `U_ShadingInfo` (1 field).
2. Work up to complex buffers like `U_GuiInput` (18 fields) and
   `U_CameraInfo` (7 fields).
3. Collapse the parameter lists in `Tracer::update_buffers()` by passing
   generated structs or small groups of structs.
4. Delete `buffer_updater.rs` when all buffers are migrated.

### Phase 3: eliminate dual declarations

1. Generate `Instance` from `shader/include/instance.glsl` and delete the
   manual Rust `Instance` struct in `builder/surface/resources.rs`.
2. Generate push constant structs (with correct std140 padding) and delete
   `PushConstantStd140` from `tracer/mod.rs`.
3. Audit `ParticleInstanceGpu` and `Vertex` for possible generation (these
   use packed formats so may need special handling).

### Phase 4: optional buffer creation shorthand

Consider a helper to reduce the repeated `from_buffer_layout` calls:

```rust
// before: 7 lines per buffer
let gui_input_layout = tracer_sm.get_buffer_layout("U_GuiInput").unwrap();
let gui_input = Buffer::from_buffer_layout(
    device.clone(), allocator.clone(),
    gui_input_layout.clone(),
    BufferUsage::empty(),
    gpu_allocator::MemoryLocation::CpuToGpu,
);

// after: 1 line
let gui_input = Buffer::new_uniform::<gpu::GuiInput>(device.clone(), allocator.clone());
```

This is lower priority since it is less impactful than the data upload changes.

## Risks and open questions

- **Build time impact**: compiling all shaders in `build.rs` adds to build
  time. Mitigation: cache SPIR-V artifacts, only recompile changed shaders
  (timestamp or hash check). The `include` directives need dependency tracking.
- **`#include` resolution**: shaders use `GL_GOOGLE_include_directive`. The
  build script needs the same include path setup as the runtime compiler.
  `shaderc` supports this via `CompileOptions::set_include_callback`.
- **std140 vs std430**: uniform buffers use std140, storage buffers use std430.
  The reflected offsets already encode the correct layout, so the codegen just
  follows the offsets. But we should verify with a test that generated struct
  sizes match reflected sizes.
- **Nested structs**: some buffer blocks contain nested GLSL structs. The
  codegen should flatten or generate nested Rust structs accordingly. The
  current `StructMemberLayout` already handles nesting, so the data is there.
- **Arrays**: `PlainMemberType::Array` exists but is underspecified. If any
  generated buffer uses arrays, the codegen needs element type and count from
  reflection. This may need `spirv-reflect` array stride info.

## Summary

The current system avoids type duplication for most buffers but replaces it
with string-based runtime serialization that is verbose, unsafe at compile
time, and causes parameter explosion. The proposed build-time codegen keeps
GLSL as the source of truth while generating typed Rust structs that make
buffer uploads a single struct-fill call. The existing descriptor auto-binding
system is unaffected.

Estimated scope: ~500 lines of `build.rs` codegen logic, then incremental
migration of buffer update call sites. Each phase is independently shippable.
