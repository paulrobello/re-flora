# Coding Plans

## Particle System (CPU-first)

Goal: add a CPU-simulated particle system that renders voxel-sized cubes for fallen leaves and butterflies. We will reuse concepts from the flora voxel pipeline (color packing, shader materials) but allow free-floating positions (float3) and per-particle colors, velocities, and lifetime.

### 1. Study & Requirements

- Review existing flora voxel representation to understand color encoding, voxel instancing, and rendering interfaces.
- Identify any reusable compression utilities; note where particle data must diverge (non-grid-aligned positions, dynamic updates).

### 2. Data Model

- Design a struct-of-arrays storage for particles: positions (Vec3), velocities (Vec3), colors/material ids, lifetime/current age, size.
- Implement a fixed-capacity ring buffer or free-list allocator so spawn/despawn is O(1) without heap allocations each frame.
- Mirror the buffer to GPU-friendly memory (e.g., staging buffer) but keep simulation authoritative on CPU.

### 3. Emitters & Behaviors

- Define an `Emitter` trait that can spawn particles with configurable rate, initial velocity distributions, and color palettes (leaf vs butterfly).
- Implement behavior modules: gravity + wind drift for leaves, spline/path following with noise for butterflies, along with optional callbacks for gameplay triggers.
- Provide scripting hooks to start/stop emitters and to inject forces when interactions happen (e.g., player footsteps).

### 4. Simulation Loop (CPU)

- Each frame: iterate the live particle arrays, integrate velocity/position (semi-implicit Euler), apply forces, update age, kill expired particles.
- Keep the loop data-oriented to leverage cache/SIMD; consider chunking updates or using rayon if counts grow.
- Expose debug metrics (particle count, spawn rate) to ensure CPU cost stays acceptable; aim for <10k active particles initially.

### 5. Rendering Path

- Build or extend an instanced draw that consumes the particle buffer: upload positions/colors to a dynamic GPU buffer and issue a single instanced cube draw using existing voxel shaders.
- Ensure butterflies can swap the instanced mesh (e.g., small quad/mesh) while reusing the same per-instance buffer format.
- Add frustum culling or LOD batching if the particle count grows.

### 6. Integration & Testing

- Create sample emitters: (1) leaf drift from canopy, settling on ground; (2) butterfly flock following a spline.
- Validate serialization/debug tooling (optional) so emitters can be tweaked quickly.
- Profile CPU vs GPU time; once CPU ceiling is hit, plan a follow-up phase to migrate high-volume effects to GPU compute while keeping control logic on CPU.

## Contree Builder macOS Compatibility

Goal: Make the current RTX-focused contree build pipeline (`shader/builder/contree/*.comp`, orchestrated by `src/builder/contree/mod.rs`) work on macOS/MoltenVK GPUs that lack shader clock, 64-bit atomics, and `VK_EXT_shader_atomic_float`.

### 1. Capability Audit & Runtime Gating

- Enumerate every non-core feature the builder assumes (e.g., `shader_int64`, `VK_EXT_shader_atomic_float`, `VK_KHR_shader_clock`, storage-buffer atomics in `make_surface.comp`, `leaf_write.comp`, `tree_write.comp`).
- Extend device creation (`src/vkn/context/device.rs:122`) to query `vk::PhysicalDeviceFeatures2` + extension feature structs, capture what is supported on the active GPU, and expose the result via a `DeviceCapabilities` struct.
- Thread this capability info into the builder/tracer setup so we can branch between RTX and fallback paths without recompiling.

### 2. Vulkan Device Setup Refactor

- Update `create_device` to only enable extensions/features advertised by the physical device; MoltenVK will refuse to create a device otherwise.
- Introduce feature flags that can be consumed by the shader compiler (`#define HAS_UINT64_ATOMICS 0/1`, etc.) so the GLSL can compile different branches for macOS.
- Add logging + validation so when a mac GPU is detected we surface exactly which optional features were disabled (useful for debugging user reports).

### 3. Atomic-Free Contree Builder Path

- Replace the global `atomicAdd` counters in `shader/builder/contree/leaf_write.comp` and `tree_write.comp` with a two-pass scheme:
  1. Pass A: per-workgroup counting → store counts into a new SSBO (`workgroup_counts[level]`) using only shared memory/barriers.
  2. Pass B: exclusive-scan those counts (GPU prefix sum kernel or CPU-side scan after `read_back`) to produce `write_offsets` for each workgroup.
  3. Pass C: scatter voxels/nodes into the dense buffers using the deterministic offsets; no atomics needed.
- Mirror this strategy for `shader/builder/surface/make_surface.comp` where flora instances and `active_voxel_len` currently use atomics—emit per-species histograms, scan, then scatter.
- Update `src/builder/contree/mod.rs` to allocate the new intermediate buffers, dispatch the additional passes per level, and keep the RTX path (atomics) behind a capability check.

### 4. CPU Fallback for Extreme Cases

- Implement a CPU version of the contree builder (likely in `src/builder/contree/mod.rs` or a sibling module) that can generate the same node/leaf buffers from the surface image when GPU storage-buffer atomics are unavailable or too slow.
- Share serialization formats so that the CPU/GPU paths can both fill the `ContreeBuilderResources` buffers, letting the rest of the renderer stay unchanged.

### 5. Validation & Tooling

- Add deterministic tests that run both the RTX (atomic) and mac (atomic-free) builders on small voxel scenes and compare node/leaf counts and hashes to ensure parity.
- Extend CI scripts to run one build under MoltenVK (or the Vulkan Portability validation layer) so regressions in the fallback path are caught early.
- Document the new capability matrix in `README.md` and add troubleshooting steps for mac users (e.g., “if prefix-sum path is active, expect 10–15% longer build time”).

### 6. Performance-Neutral Migration Strategy

- Baseline today’s GPU time for each contree pass (leaf/tree write, surface builder, concat) on RTX hardware, storing metrics so we can prove parity after migration.
- Implement a deterministic, three-stage pipeline (Count → Scan → Scatter) that replaces each atomic region, but keep the count and scatter kernels sharing the same memory access pattern, workgroup size, and subgroup operations as the existing code so occupancy stays identical.
- Use subgroup operations (`subgroupAdd`, `subgroupExclusiveAdd`) where available to keep count kernels as fast as atomic increments; guard them with `VK_KHR_shader_subgroup_extended_types` fallback paths for macOS so no branch sacrifices throughput.
- Run the prefix-sum stage either:
  - On GPU using a hierarchical scan (local scan + global reduction) that keeps all accesses coalesced, or
  - On CPU via mapped memory only when the device reports low compute throughput (not the mac case); in either case, assert that the total number of memory transactions matches the RTX baseline.
- Feed the scan result back into the scatter pass via device-local buffers so we avoid synchronization with the CPU; reuse the same workgroup scheduling as before to maintain load balance.
- Validate performance parity by re-running the baselines inside an automated benchmark harness (e.g., headless `cargo run --release -- contree-bench`) on both RTX and macOS devices, and only ship when the mac path stays within ±2% of the RTX timings for identical inputs; if needed, tune workgroup sizes dynamically per GPU to remove any discrepancy.
