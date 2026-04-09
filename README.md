# Re: Flora

> A meditative voxel-based gardening game. Cultivate your own island ecosystem.

![Re: Flora](./demo/img/splash.png)

## What is this?

Re: Flora is a cozy voxel game prototype focused on terrain shaping, planting flora, and building a calm island atmosphere.

## Quick Start

1. Install Rust (latest stable): <https://rustup.rs/>
2. Install Vulkan SDK/tools:
   - Linux: `libvulkan-dev` + `vulkan-tools` (or distro equivalent)
   - Windows: <https://vulkan.lunarg.com/sdk/home#vulkansdk>
3. Build and run:

```bash
cargo run --release
```

The first build may take a while due to shader compilation.

## Requirements

- Rust (latest stable)
- Vulkan SDK or development packages
- Vulkan-capable GPU with up-to-date drivers (non-RTX GPUs are supported)

## Configuration

- Runtime settings live in `config/gui.toml`
- Many values can also be tuned live in-game via the Tab menu

## Tech Stack

| Domain    | Library / Tool                                       |
| --------- | ---------------------------------------------------- |
| Rendering | Vulkan (`ash`) with ray tracing extensions           |
| UI        | `egui`                                               |
| Audio     | `petalsonic`                                         |
| Terrain   | Procedural generation via `fastnoise-lite` + `noise` |

## Docs

- References and technical reading: `docs/references.md`
- Inspirations and art direction links: `docs/inspirations.md`

## Special Thanks

- [adrien-ben/egui-ash-renderer](https://github.com/adrien-ben/egui-ash-renderer) for `ash` + `egui` integration
- TheMaister, Khronos Group, and graphics education authors for Vulkan and rendering guidance

## License

This project uses a dual license:

- Source code: [GNU General Public License v3.0](./LICENSE)
- Non-code assets (art, audio, images, config, etc.): [CC BY-NC-SA 4.0](./LICENSE-ASSETS)
