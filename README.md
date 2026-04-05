# Re: Flora

> A meditative voxel-based gardening game. Cultivate your own island ecosystem.

![Re: Flora](./demo/img/splash.png)

---

## Requirements

- **Rust** (latest stable)
- **Vulkan SDK** — [LunarG](https://vulkan.lunarg.com/)
- A Vulkan-capable GPU with up-to-date drivers, non-RTX cards are also first class supported.

---

## Getting Started

### 1. Install Rust

**Linux**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Windows**

1. Download and run [rustup-init.exe](https://win.rustup.rs/)
2. Follow the on-screen prompts
3. Restart your terminal

---

### 2. Install Vulkan SDK

**Linux**

```bash
# Ubuntu / Debian
sudo apt install libvulkan-dev vulkan-tools

# Arch Linux
sudo pacman -S vulkan-headers vulkan-validation-layers
```

**Windows**

1. Download the SDK from [LunarG](https://vulkan.lunarg.com/sdk/home#vulkansdk)
2. Run the installer — environment variables are set automatically
3. Verify the install with `vulkaninfo`

---

### 3. Build & Run

```bash
cargo run --release
```

> **Note:** The first build may take several minutes due to shader compilation.

---

## Controls

| Action       | Input       |
| ------------ | ----------- |
| Move         | WASD        |
| Look         | Mouse       |
| Sprint       | Hold Shift  |
| Place Plant  | Left Click  |
| Remove Plant | Right Click |
| Open GUI     | Tab         |

---

## Configuration

Runtime-editable parameters (sky, audio, particles, etc.) live in `config/gui.toml`. Many settings can also be tweaked live via the in-game GUI (Tab key).

---

## Tech Stack

| Domain    | Library / Tool                                       |
| --------- | ---------------------------------------------------- |
| Rendering | Vulkan (`ash`) with ray tracing extensions           |
| UI        | `egui`                                               |
| Audio     | `petalsonic`                                         |
| Terrain   | Procedural generation via `fastnoise-lite` + `noise` |

---

## macOS / Metal Support

Re: Flora runs on macOS via [MoltenVK](https://github.com/KhronosGroup/MoltenVK), a Vulkan-to-Metal translation layer. The `macos` branch contains targeted shader and pipeline optimizations for Metal's execution model:

- All large `const` array lookups replaced with hash-based alternatives (Metal penalizes computed-index const arrays 25-500x)
- FBM noise in vertex/compute shaders replaced with cheap hash approximations
- `uint64_t` removed from all shaders (no native Metal support)
- Render passes batched to minimize Metal command encoder overhead

**macOS build prerequisites:**

```bash
brew install vulkan-headers vulkan-loader molten-vk shaderc
make deps        # install all brew dependencies
make run-windowed-novalidation   # build and run
```

See the [CHANGELOG](./CHANGELOG.md) for the full list of changes.

---

## Resources & References

### Vulkan

- [Descriptor Sets — Vulkan Guide](https://vkguide.dev/docs/chapter-4/descriptors/#binding-descriptors)
- [Descriptor Sets — NVIDIA Guide](https://developer.nvidia.com/vulkan-shader-resource-binding)
- [Vulkan Synchronization Explained](https://themaister.net/blog/2019/08/14/yet-another-blog-explaining-vulkan-synchronization/)

### Ray Tracing

- [Ray Tracing in Vulkan — Khronos](https://www.khronos.org/blog/ray-tracing-in-vulkan/)
- [GLSL_EXT_ray_query Shading Documentation](https://github.com/KhronosGroup/GLSL/blob/main/extensions/ext/GLSL_EXT_ray_query.txt/)
- [Ray Tracing Pipeline vs. Ray Query Performance](https://tellusim.com/rt-perf/)
- [NVIDIA RTX Best Practices (1)](https://developer.nvidia.com/blog/rtx-best-practices/)
- [NVIDIA RTX Best Practices (2 — Updated)](https://developer.nvidia.com/blog/best-practices-for-using-nvidia-rtx-ray-tracing-updated/)
- [Fast Voxel Ray Tracing using Sparse 64-trees](https://dubiousconst282.github.io/2024/10/03/voxel-ray-tracing/) — ([GitHub](https://github.com/dubiousconst282/VoxelRT) · [Reddit](https://www.reddit.com/r/VoxelGameDev/comments/1fzimke/a_guide_to_fast_voxel_ray_tracing_using_sparse/))
- [Ray-AABB Intersection Algorithm](https://medium.com/@bromanz/another-view-on-the-classic-ray-aabb-intersection-algorithm-for-bvh-traversal-41125138b525)
- [BRDF and PDF for Sampling](https://computergraphics.stackexchange.com/questions/8578/how-to-set-equivalent-pdfs-for-cosine-weighted-and-uniform-sampled-hemispheres)

### Papers

- [ReSTIR GI: Path Resampling for Real-Time Path Tracing](https://research.nvidia.com/publication/2021-06_restir-gi-path-resampling-real-time-path-tracing)

### Inspirational Tech & Art

**Procedural Generation**

- [Procedural Island Generator in Blender](https://blenderartists.org/t/procedural-island-generator-illustration-using-blenders-geometry-nodes/1483314)

**Voxel Worlds**

- [Exploring an Infinite Voxel Forest](https://www.youtube.com/watch?v=1wufuXY3l1o)
- [Animated Voxel Trees — Detail Enhancement](https://www.youtube.com/watch?v=BObFTsNeeGc)

**Physics & Simulation**

- [Voxel Water Physics](https://www.youtube.com/watch?v=1R5WFZDXaEXTOI)
- [Rigid Body Physics](https://www.youtube.com/watch?v=byP6cA71Cgw)

**Audio**

- [Ray Traced Reverb, Wind and Sound Occlusion](https://www.youtube.com/watch?v=UHzeQZD9t2s)
- [Ray Tracing Sound in a Voxel World](https://www.youtube.com/watch?v=of3HwxfAoQU)

**Particles & Effects**

- [Animated Voxel Grass](https://www.youtube.com/watch?v=dGZDXaEXTOI)
- [How I added particles! (Grass)](https://www.youtube.com/watch?v=rf9Piwp91pE)

**General Graphics & Optimization**

- [Other Optimization Techniques](https://www.youtube.com/watch?v=PYu1iwjAxWM)
- [Scratchapixel CG Tutorials](https://www.scratchapixel.com/)

**Post Processing**

- [The Art of Dithering and Retro Shading](https://blog.maximeheckel.com/posts/the-art-of-dithering-and-retro-shading-web/)

### Assets

- [Sound Effects — Pixabay](https://pixabay.com/sound-effects/)

---

## Special Thanks

- **[adrien-ben/egui-ash-renderer](https://github.com/adrien-ben/egui-ash-renderer)** — `ash` + `egui` integration
- **TheMaister** — [Vulkan Synchronization Tutorial](https://themaister.net/blog/2019/08/14/yet-another-blog-explaining-vulkan-synchronization/)
- **Khronos Group** — [Synchronization Examples](https://github.com/KhronosGroup/Vulkan-Docs/wiki/Synchronization-Examples) and [Command Buffer Lifecycle](https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#commandbuffers-lifecycle) docs
- **Cambridge in Colour** — [Gamma Correction Tutorial](https://www.cambridgeincolour.com/tutorials/gamma-correction.htm)
- **Sébastien Piquemal** — [Interactive Gamma Correction & sRGB](https://observablehq.com/@sebastien/srgb-rgb-gamma)

---

## License

This project uses a dual license:

- **Source code** — [GNU General Public License v3.0](./LICENSE)
  - You may use, modify, and distribute the code under GPLv3 terms.
- **All other assets** (art, audio, images, config, etc.) — [CC BY-NC-SA 4.0](./LICENSE-ASSETS)
  - Attribution required. Non-commercial use only. Derivatives must share under the same license.
