# Re: Flora

> A meditative voxel-based gardening game. Cultivate your own island ecosystem.

## Requirements

- **Rust** (latest stable)
- **Vulkan** SDK ([install from LunarG](https://vulkan.lunarg.com/))
- A Vulkan-capable GPU with drivers installed

## Getting Started

### 1. Install Rust

#### Linux

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### Windows

1. Download and run [rustup-init.exe](https://win.rustup.rs/)
2. Follow the on-screen prompts
3. Restart your terminal

### 2. Install Vulkan SDK

Download and install from [LunarG](https://vulkan.lunarg.com/).

#### Linux

```bash
# Ubuntu/Debian
sudo apt install libvulkan-dev vulkan-tools

# Arch Linux
sudo pacman -S vulkan-headers vulkan-validation-layers
```

#### Windows

1. Download the Vulkan SDK from [LunarG](https://vulkan.lunarg.com/sdk/home#vulkansdk)
2. Run the installer and follow the prompts
3. The installer sets up environment variables automatically
4. Verify with `vulkaninfo`

### 3. Build & Run

```bash
cargo run --release
```

> **Note:** First build may take a few minutes due to shader compilation.

## Controls

| Action       | Input       |
| ------------ | ----------- |
| Move         | WASD        |
| Look         | Mouse       |
| Sprint       | Hold Shift  |
| Place Plant  | Left Click  |
| Remove Plant | Right Click |
| Open GUI     | Tab         |

## Configuration

Editable parameters (sky, audio, particles, etc.) are in `config/gui.toml`. Some settings can be tweaked at runtime via the in-game GUI (Tab key).

## Tech Stack

- **Rendering**: Vulkan (via `ash`) with ray tracing extensions
- **UI**: `egui`
- **Audio**: `petalsonic`
- **Terrain**: Procedural generation with `fastnoise-lite` and `noise`

## Resources & References

### Vulkan

- [Descriptor set - Vulkan Guide](https://vkguide.dev/docs/chapter-4/descriptors/#binding-descriptors)
- [Descriptor set - Nvidia's Guide](https://developer.nvidia.com/vulkan-shader-resource-binding)
- [Vulkan Synchronization Explained](https://themaister.net/blog/2019/08/14/yet-another-blog-explaining-vulkan-synchronization/)

### Ray Tracing

- [Official Khronos Guide to Ray Tracing in Vulkan](https://www.khronos.org/blog/ray-tracing-in-vulkan/)
- [GLSL_EXT_ray_query Shading Documentation](https://github.com/KhronosGroup/GLSL/blob/main/extensions/ext/GLSL_EXT_ray_query.txt/)
- [Ray Tracing Pipeline vs. Ray Query Performance](https://tellusim.com/rt-perf/)
- [NVIDIA RTX Best Practices (1)](https://developer.nvidia.com/blog/rtx-best-practices/)
- [NVIDIA RTX Best Practices (2 - Updated)](https://developer.nvidia.com/blog/best-practices-for-using-nvidia-rtx-ray-tracing-updated/)
- [A Guide to Fast Voxel Ray Tracing using Sparse 64-trees](https://dubiousconst282.github.io/2024/10/03/voxel-ray-tracing/)
  - [Associated GitHub Project](https://github.com/dubiousconst282/VoxelRT)
  - [Reddit Discussion](https://www.reddit.com/r/VoxelGameDev/comments/1fzimke/a_guide_to_fast_voxel_ray_tracing_using_sparse/)
- [Another View on the Classic Ray-AABB Intersection Algorithm](https://medium.com/@bromanz/another-view-on-the-classic-ray-aabb-intersection-algorithm-for-bvh-traversal-41125138b525)
- [Understanding BRDF and PDF for Sampling](https://computergraphics.stackexchange.com/questions/8578/how-to-set-equivalent-pdfs-for-cosine-weighted-and-uniform-sampled-hemispheres)

### Papers

- [ReSTIR GI: Path Resampling for Real-Time Path Tracing](https://research.nvidia.com/publication/2021-06_restir-gi-path-resampling-real-time-path-tracing)

### Inspirational Tech & Art

- **Procedural Generation**: [Procedural Island Generator in Blender](https://blenderartists.org/t/procedural-island-generator-illustration-using-blenders-geometry-nodes/1483314)
- **Voxel Worlds**:
  - [Exploring an Infinite Voxel Forest](https://www.youtube.com/watch?v=1wufuXY3l1o)
  - [Animated Voxel Trees - Detail Enhancement](https://www.youtube.com/watch?v=BObFTsNeeGc)
- **Physics & Simulation**:
  - [Voxel Water Physics](https://www.youtube.com/watch?v=1R5WFZDXaEXTOI)
  - [Rigid Body Physics](https://www.youtube.com/watch?v=byP6cA71Cgw)
- **Audio**:
  - [Ray Traced Reverb, Wind and Sound Occlusion](https://www.youtube.com/watch?v=UHzeQZD9t2s)
  - [Ray Tracing Sound in a Voxel World](https://www.youtube.com/watch?v=of3HwxfAoQU)
- **Particles & Effects**:
  - [Animated Voxel Grass](https://www.youtube.com/watch?v=dGZDXaEXTOI)
  - [How I added particles! (Grass)](https://www.youtube.com/watch?v=rf9Piwp91pE)
- **General Graphics & Optimization**:
  - [Other Optimization Techs](https://www.youtube.com/watch?v=PYu1iwjAxWM)
  - [Scratchapixel CG Tutorials](https://www.scratchapixel.com/)
- **Post Processing**:
  - [The Art of Dithering](https://blog.maximeheckel.com/posts/the-art-of-dithering-and-retro-shading-web/)

### Resources

- [Sound Effects](https://pixabay.com/sound-effects/)

---

## Special Thanks To

- **[adrien-ben/egui-ash-renderer](https://github.com/adrien-ben/egui-ash-renderer)** for the implementation of `ash` with `egui`.
- **TheMaister's Blog** for the excellent [Vulkan Synchronization Tutorial](https://themaister.net/blog/2019/08/14/yet-another-blog-explaining-vulkan-synchronization/).
- **Khronos Group** for the official [Vulkan Synchronization Examples](https://github.com/KhronosGroup/Vulkan-Docs/wiki/Synchronization-Examples) and documentation on the [Command Buffer Lifecycle](https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#commandbuffers-lifecycle).
- **Cambridge in Colour** for the clear tutorial on [Gamma Correction](https://www.cambridgeincolour.com/tutorials/gamma-correction.htm).
- **Sébastien Piquemal** for the interactive explanation of [Gamma Correction and sRGB](https://observablehq.com/@sebastien/srgb-rgb-gamma).
