# PLANs

## Priorities - From High to Low

### Bugfixes

### Features

- Let our terrain be grounded, sand interacts with the ocean. currently, ocean is below y=0, maybe we use y=0.2 as the basis.
  For generated terrain, ensure the minimum height of the terrain is 2 voxels, that is, 2.0 / 256.0

- Little Pond, with SSR reflection so it can reflect on both the terrain and the flora

- Ocean view, more pixelized looking - Need more investigation on this one

- Clouds, with pixel vibe

- Randomized rocks, maybe through model import, or using SDF plainly (cleaner, supports random gen, but way harder to look good)

- More flora types

### Optimizes

- Optimize preculling: first we do tracing on terrain to update the depth buffer, so we can avoid most of the work when rendering the fragments (flora...), that are being occuluded by the voxel terrain.
