# Product Roadmap

This roadmap captures planned improvements for gameplay, visuals, and performance.
Items are ordered by priority and intended implementation sequence.

## 1) Critical Fixes

- No critical bugfixes are currently tracked.

## 2) High-Priority Features

- **Terrain Editing UI refresh**
  - Move voxel-type display sliders to the top-right panel.
  - Replace display-only sliders with progress bars.
  - Set per-voxel storage cap to `0.5x` current maximum.

- **Flora construction system**
  - Implement a build/placement workflow for planting flora.

- **Port trading loop**
  - Add a docked ship that enables resource exchange.
  - Support buy/sell interactions for player inventory.

- **Terrain harvesting feedback**
  - Add particle effects at terrain-edit positions.
  - Emit particles matching voxel color.
  - Animate particles toward the player camera to indicate collection into backpack storage.

## 3) Visual & World Expansion

- **Reflective pond biome element**
  - Add a small pond with SSR reflections for terrain and flora.

- **Ocean presentation pass**
  - Create a more pixelized ocean look.
  - Continue visual research and prototyping.

- **Stylized cloud system**
  - Add clouds with a strong pixel-art aesthetic.

- **Procedural rock variants**
  - Evaluate model import vs. SDF-based generation.
  - Favor SDF for cleaner procedural control, while validating visual quality.

- **Additional flora types**
  - Expand flora variety to improve biome richness.

## 4) Performance Work

- **Pre-culling optimization**
  - Trace terrain first to update depth buffer early.
  - Skip shading work for flora/fragments occluded by voxel terrain.
