# Product Roadmap

This roadmap captures planned improvements for gameplay, visuals, and performance.
Items are ordered by priority and intended implementation sequence.

## 1) Critical Fixes

- No critical bugfixes are currently tracked.

## 2) High-Priority Features

- **Trading system**
  - Add a docked ship that enables resource exchange. (Draw the docked ship, the port, with primitives during world init time
  - Support buy/sell interactions for player inventory.

- **Terrain harvesting feedback**
  - Add particle effects at terrain-edit positions.
  - Emit particles matching voxel color.
  - Animate particles toward the player camera to indicate collection into backpack storage.

- macOS adaption
  - Make sure everything works properly in a reasonable framerate in macOS
  - Make sure bootstrap_macos.sh work properly

## 3) Visual & World Expansion

- **Reflective pond biome element**
  - Add a small pond with SSR reflections for terrain and flora.

- **Ocean presentation pass**
  - Create a more pixelized ocean look.
  - Continue visual research and prototyping.

- **Procedural rock variants**
  - Evaluate model import vs. SDF-based generation.
  - Favor SDF for cleaner procedural control, while validating visual quality.

- **Additional flora types**
  - Expand flora variety to improve biome richness.

- **Stylized cloud system**
  - Add clouds with a strong pixel-art aesthetic.

## 4) Performance Work

- **Pre-culling optimization**
  - Trace terrain first to update depth buffer early.
  - Skip shading work for flora/fragments occluded by voxel terrain.

- **Semaphore and Fences checks**
  - Check if semaphores and fences are setup properly across each frame, for the highest possible performance
