# re-flora

Voxel ray-tracing engine using Vulkan compute shaders, Rust. macOS target via MoltenVK.

## Build & Run (Makefile)

**IMPORTANT**: Always use Makefile targets. Never run the binary directly or use `pkill -f "re-flora"` (kills tmux sessions too).

```bash
make build-release              # Release build with validation layers
make build-release-novalidation # Release build without validation layers
make run-windowed               # Windowed mode (1280x720), with validation
make run-windowed-novalidation  # Windowed mode, no validation layers (faster)
make kill                       # Kill running instance (uses pkill -x, safe for tmux)
make fmt                        # cargo fmt
make lint                       # cargo clippy
make checkall                   # fmt + lint + test
make deps                       # Install brew dependencies
```

## CLI Flags

| Flag | Description |
|------|-------------|
| `--windowed` / `-w` | Run in windowed mode (1280x720) instead of borderless fullscreen |
| `--no-shadows` | Disable shadow tracer + VSM filtering passes |
| `--no-tracer` | Disable main ray tracer pass |
| `--no-denoise` | Disable temporal + spatial denoiser passes |
| `--no-god-rays` | Disable volumetric god ray pass |
| `--no-lens-flare` | Disable lens flare (sun visible + flare + downsample) passes |
| `--no-particles` | Disable particle simulation (butterflies, leaves) |
| `--no-flora` | Disable flora/leaves graphics passes (grass, tree leaves) |
| `--screenshot <path>` | Save GPU readback screenshot to `<path>` after rendering starts |
| `--screenshot-delay <secs>` | Seconds to wait after rendering starts before taking screenshot (default: 5) |
| `--auto-exit <secs>` | Auto-exit N seconds after rendering starts |
| `--perf` | Enable per-frame FPS and timing output to console |

Combine flags for profiling: `make run-windowed-novalidation ARGS="--no-shadows --no-tracer --no-denoise --no-god-rays --no-lens-flare --no-particles --no-flora"`

### Automated Screenshot Workflow

Use `--screenshot` with `--auto-exit` for headless testing. The app loads ~2 min (50 chunks + tree), then rendering starts. Set delays accordingly:

```bash
# Take screenshot 10s after rendering starts, exit 5s after that
make run-windowed-novalidation ARGS="--screenshot /tmp/screenshot.png --screenshot-delay 10 --auto-exit 15"
```

**IMPORTANT**: Always use the built-in `--screenshot` flag for GPU screenshots. `screencapture` cannot capture Vulkan/MoltenVK windows. The `--auto-exit` flag ensures the process terminates cleanly so scripts can wait on it.

## Testing Workflow

1. After code changes: `make build-release-novalidation` (fastest feedback)
2. Run with logging: `make run-windowed-novalidation` — FPS and PERF logs print to terminal every 500ms
3. Wait ~2 minutes for all 50 chunks to load, then check `[FPS]` and `[PERF]` log lines
4. Kill when done: `make kill`
5. Full checks before commit: `make checkall`

## Profiling (PERF logs)

The app logs per-frame timing:
```
[PERF] egui: 0.1ms | particle: 0.0ms | fence: 0.0ms | acquire: 0.0ms | img_fence: 1350.0ms | record: 0.4ms | present: 0.0ms | camera: 0.0ms | total: 1351.0ms
```
- `egui`: Time for egui update + UI recording
- `particle`: Time in particle simulation (terrain queries are the expensive part)
- `fence`: Time waiting for frames_in_flight fence (frame N-2 GPU completion)
- `acquire`: Time in vkAcquireNextImageKHR
- `img_fence`: Time waiting for per-swapchain-image fence (**main GPU bottleneck**)
- `record`: CPU time to record command buffer + queue_submit
- `present`: Time in vkQueuePresentKHR
- `camera`: Time in update_camera (GPU readback)

## macOS / MoltenVK Notes

- Vulkan environment variables are set in the Makefile (DYLD_LIBRARY_PATH, VK_ICD_FILENAMES, etc.)
- Phonon (Steam Audio) library path is auto-detected from build artifacts
- Validation layers are available but add overhead; use `novalidation` builds for FPS testing
- D32_SFLOAT with STORAGE_BIT works on MoltenVK for read-only access (imageLoad), but prefer SAMPLED + sampler2D/texelFetch for read-only depth access
- `GL_ARB_gpu_shader_int64` is NOT natively supported on Metal/MoltenVK — uint64_t in shaders causes software emulation
- Metal/MoltenVK penalizes large const arrays with loop-based indexing (~25-500x slowdown); use hash functions or unrolled if-else chains instead. This applies to ALL shaders including compute — not just vertex shaders.
- FastNoiseLite (fast_noise_lite.glsl) uses large const float arrays (GRADIENTS_2D, RAND_VECS_2D etc.) — NEVER use in compute shaders on MoltenVK. Use hash-based value noise FBM instead.
- FBM noise (4+ octave) in vertex shaders is extremely expensive on Metal; use hash-based approximations for LOD/shadow passes

## Architecture

- Build: `build.rs` compiles GLSL shaders via shaderc, extracts struct layouts via SPIR-V reflection, generates Rust code
- Render pipeline: Compute ray tracer + graphics flora/particle passes + denoiser + composition
- Contree: Quadtree-like voxel octree traversal (up to 1024 iterations per ray)
- Frame sync: Double-buffered (MAX_FRAMES_IN_FLIGHT=2), triple-buffered swapchain, CPU waits on per-image fence
- Scaling: Renders at 0.5x window resolution, upscaled in post-processing
- Shadow map: 1024x1024 D32_SFLOAT, fixed resolution

## Performance (M4 Max, MoltenVK)

**Six bottlenecks found and fixed: FBM noise in vertex shaders, skylight constant-array penalty, leaves shadow FBM wind, and three compute shader const-array penalties (surface builder, flora seeding, heightmap).**

### Profiling results (progressive isolation)

| Config | img_fence | FPS | Finding |
|--------|-----------|-----|---------|
| All passes disabled | 0ms | 4100 | Baseline overhead |
| All passes disabled (after skylight fix) | 0.6ms | ~1000 | Skylight was 48ms base |
| Flora only (original) | ~1350ms | 0.7 | Flora vertex shaders = ~1300ms |
| Flora only (LOD1 fix) | ~66ms | 14.7 | LOD1 FBM replaced with hashes |
| Flora only (+ skylight fix) | ~48ms | 20 | Skylight constant-array removed |
| Flora only (+ shadow fix) | ~12ms | 80 | Shadow FBM wind eliminated |
| Full pipeline (after render fixes) | ~16ms | **58** | **5.3x total improvement** |
| Full pipeline (+ compute fixes) | ~8ms | **117** | **10x total improvement** |

### Fix 1: Flora LOD1 vertex shader (shader/foliage/flora_lod.vert)

LOD1 flora draws 116K instances × 48 indices = 5.57M vertices. The original shader ran per-vertex:
- 3× FBM noise calls (4 octaves each) for wind = ~83M noise evaluations/frame
- VSM shadow map texture sample + matrix multiply per vertex
- 12-iteration Box-Muller for grass height sampling
- 3-octave Perlin FBM for grass band color

All replaced with cheap hash-based approximations:
- Wind: `sin(time + hash_phase) * 0.3` per instance seed
- Shadow: analytical dot-product only (skip VSM texture)
- Height: single Wellons hash → uniform mix
- Color: single hash → linear interpolation between dark/light pairs

### Fix 2: Skylight constant-array penalty (shader/include/skylight.glsl)

Metal/MoltenVK handles large const struct arrays with loop-based indexing extremely poorly (~25-50x penalty). The skylight TIME_KEYFRAMES[11] and VIEW_KEYFRAMES[7] with loop interpolation + srgb_to_linear (pow) calls cost 26ms alone. Replaced with pre-linearized vec3 constants and unrolled if-else chain. Base GPU overhead dropped from 48ms to <1ms.

### Fix 3: Leaves shadow FBM wind (shader/foliage/leaves_shadow.vert)

The leaves shadow pass drew tree leaf instances into a 1024x1024 shadow map using the same expensive `get_wind()` function (3× FBM noise, 4 octaves each). Cost: 33ms/frame. Replaced with the same hash-based sway from flora_lod.vert. Cost reduced to <1ms.

### Fix 4: gfx_depth_tex and gfx_output_tex STORAGE→SAMPLED

Changed both render pass attachments from `imageLoad`/`image2D` (requires STORAGE) to `texelFetch`/`sampler2D` (requires SAMPLED). Updated composition.comp, god_ray.comp, and lens_flare_sun_visible.comp. While this didn't measurably improve performance (the render pass overhead was from vertex shaders, not attachment formats), it's architecturally correct — SAMPLED is the proper descriptor type for read-only texture access in compute shaders.

### Render pass batching

All flora species (3 types × 2 LOD levels) + tree leaves now draw in a **single Vulkan render pass** via `record_all_flora_and_leaves()`, eliminating 8+ separate render pass begin/end cycles.

### Infrastructure changes (from voxel-world comparison)

- Triple-buffered swapchain (`min_image_count.max(3)`)
- MAX_FRAMES_IN_FLIGHT=2 (was 1)
- `execute_one_time_command` uses fence-based wait instead of `wait_queue_idle`
- Added `--no-flora` and `--no-particles` CLI flags for profiling isolation

### Fix 5: Surface builder blue noise const array (shader/builder/surface/make_surface.comp)

`voxel_hash_blue_noise_packed[2048]` const array with computed-index access — exact same Metal penalty as skylight. Called for every surface voxel in 256³ chunks. Replaced with a Wellons hash returning 2-bit bucket ID. Surface build time: 890ms → 2ms per chunk.

### Fix 6: Flora seeding FNL const arrays (shader/builder/surface/occupancy_to_flora_instances.comp)

FastNoiseLite's `GRADIENTS_2D[]`, `RAND_VECS_2D[]` const arrays triggered the Metal penalty in compute shaders too. The flora seeding uses Perlin noise for density placement and biome selection. Replaced with hash-based value noise. Flora seeding time: 830ms → 1ms per chunk.

### Fix 7: Heightmap FNL const arrays (shader/builder/chunk_writer/chunk_heightmap.comp)

Same FastNoiseLite const-array penalty in the terrain heightmap generator. Four FBM noise calls (3-5 octaves each) for terrain shape, coastline, and island masking. Replaced with hash-based value noise FBM. Chunk init time: 40ms → <5ms per chunk. **Total loading time: ~90s → ~0.4s for 50 chunks.**

### Remaining opportunities

- LOD0 flora still uses full 4-octave FBM wind (only 408 instances — negligible cost)
- Particle passes still use their own render pass (could merge)
- Particle terrain queries still do synchronous GPU roundtrips

