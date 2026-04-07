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
- D32_SFLOAT with STORAGE_BIT works on MoltenVK for read-only access (imageLoad)
- `GL_ARB_gpu_shader_int64` is NOT natively supported on Metal/MoltenVK — uint64_t in shaders causes software emulation

## Architecture

- Build: `build.rs` compiles GLSL shaders via shaderc, extracts struct layouts via SPIR-V reflection, generates Rust code
- Render pipeline: Compute ray tracer + graphics flora/particle passes + denoiser + composition
- Contree: Quadtree-like voxel octree traversal (up to 1024 iterations per ray)
- Frame sync: Double-buffered (MAX_FRAMES_IN_FLIGHT=2), triple-buffered swapchain, CPU waits on per-image fence
- Scaling: Renders at 0.5x window resolution, upscaled in post-processing
- Shadow map: 1024x1024 D32_SFLOAT, fixed resolution

## Performance (M4 Max, MoltenVK)

**Root cause found and fixed: Flora LOD1 vertex shaders doing expensive FBM noise per-vertex on 5.57M vertices/frame.**

The original hypothesis (MoltenVK presentation pipeline) and second hypothesis (tracer compute shader) were both wrong. Systematic profiling isolated the real bottleneck to the flora/leaves graphics vertex shaders.

### Profiling results (progressive isolation)

| Config | img_fence | FPS | Finding |
|--------|-----------|-----|---------|
| All passes disabled | 0ms | 4100 | Baseline overhead |
| Tracer only (no flora) | 46ms | 21 | Tracer compute = 46ms |
| Flora enabled, tracer disabled | ~1350ms | 0.7 | **Flora = ~1300ms** |
| Flora + LOD1 optimizations | ~66ms | 14.7 | LOD1 cost reduced to ~20ms |
| Full pipeline (all passes) | ~91ms | 10.9 | **15.6x improvement** |

### What was fixed (shader/foliage/flora_lod.vert)

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

### Render pass batching

All flora species (3 types × 2 LOD levels) + tree leaves now draw in a **single Vulkan render pass** via `record_all_flora_and_leaves()`, eliminating 8+ separate render pass begin/end cycles. (This alone didn't measurably help — the vertex shader cost dominated.)

### Infrastructure changes (from voxel-world comparison)

- Triple-buffered swapchain (`min_image_count.max(3)`)
- MAX_FRAMES_IN_FLIGHT=2 (was 1)
- `execute_one_time_command` uses fence-based wait instead of `wait_queue_idle`
- Added `--no-flora` and `--no-particles` CLI flags for profiling isolation

### Remaining opportunities

- LOD0 flora still uses full 4-octave FBM wind (only 408 instances — negligible cost)
- Particle passes still use their own render pass (could merge)
- Leaves shadow LOD pass still separate (could merge)
- Particle terrain queries still do synchronous GPU roundtrips

