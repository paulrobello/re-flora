# Changelog

All notable changes to re-flora on the `macos` branch (since diverging from `main`).

## [Unreleased] - macos branch

### Performance (MoltenVK/Metal)

- **10x FPS improvement** on M4 Max (0.7 FPS -> 117 FPS full pipeline)
- Replace FBM noise with hash-based wind sway in flora LOD1 vertex shader (`flora_lod.vert`)
- Replace FBM noise wind in leaves shadow vertex shader (`leaves_shadow.vert`)
- Replace skylight const-array + loop interpolation with pre-linearized constants and unrolled if-else (`skylight.glsl`)
- Replace blue noise const array in surface builder with Wellons hash (`make_surface.comp`)
- Replace FastNoiseLite const arrays in flora seeding with hash-based value noise (`occupancy_to_flora_instances.comp`)
- Replace FastNoiseLite const arrays in heightmap generator with hash-based value noise FBM (`chunk_heightmap.comp`)
- Batch all flora species (3 types x 2 LOD levels) + tree leaves into a single Vulkan render pass
- Change depth and output textures from STORAGE to SAMPLED descriptor type for read-only compute access

### Rendering

- Triple-buffered swapchain (`min_image_count.max(3)`)
- Double-buffered frames in flight (`MAX_FRAMES_IN_FLIGHT=2`, was 1)
- Fence-based wait in `execute_one_time_command` (replaces `wait_queue_idle`)
- Remove `uint64_t` usage from all shaders (not natively supported on Metal, causes software emulation)
- Add rock color variance in voxel color palette
- Fix chunk boundary diagonal line artifacts via two-phase loading (all terrain voxels written before surface normals computed)
- Add multi-octave terrain detail noise for more natural terrain surfaces

### GUI

- Flora draw distance parameter (adjustable in Debug panel)
- Leaves color palette now tints from GUI color settings instead of ignoring them
- Runtime leaves regeneration when density/radius/color params change
- Reset to Defaults button in Debug panel (reloads `config/gui.toml`)
- Fix clicking in debug panel recapturing mouse cursor
- Fix mouse tracking activating before first click in windowed mode (`cursor_engaged` guard)

### CLI

- `--perf` flag to enable per-frame FPS and timing output to console
- `--no-flora` flag to disable flora/leaves graphics passes
- `--no-particles` flag to disable particle simulation
- `--screenshot <path>` to save GPU readback screenshot after rendering starts
- `--screenshot-delay <secs>` to configure screenshot timing (default: 5s)
- `--auto-exit <secs>` to auto-terminate after rendering starts
- `AppOptions` and `RenderFlags` structs for structured CLI parsing and pass control

### Build

- Add `Makefile` and `Makefile.macos` with standard build targets (`build-release`, `run-windowed`, `kill`, `fmt`, `lint`, `checkall`)
- Add `chunk_heightmap.comp` compute shader (extracted from `chunk_init.comp`)

### Dependencies

- egui 0.33.2 -> 0.34.1 (fix deprecated API calls: `is_pointer_over_egui`, `global_style`, `set_global_style`)
- egui-winit 0.33.2 -> 0.34.1
- glam 0.30.0 -> 0.32.1
- comfy-table 7.1.3 -> 7.2.2
- log 0.4.25 -> 0.4.29
- thiserror 2.0.11 -> 2.0.18
- ordered-float 5.0.0 -> 5.3.0
- once_cell 1.21.3 -> 1.21.4
- indexmap 2.9.0 -> 2.13.1
- image 0.25.6 -> 0.25.10
- anyhow 1.0.98 -> 1.0.102
- bytemuck 1.23.1 -> 1.25.0

### Cleanup

- Demote verbose per-chunk contree logs from `info` to `debug`
- Remove `debug_read_scene_tex()` call from post-init
- Gate all `[FPS]` and `[PERF]` log output behind `--perf` flag
- Add CLAUDE.md with project documentation (build instructions, CLI flags, architecture, profiling results)
