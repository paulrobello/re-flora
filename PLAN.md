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
