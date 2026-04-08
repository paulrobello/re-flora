# PLANs

## Priorities - From High to Low

### Bugfixes

### Features

- Terrain editing tool, put the sliders display per voxel type to top right, and change the slider to be progress bars, since it's display only. make the max lim for each voxel type (the storage), to be 0.5x

- For terrain editing, add a particle effect that generates same colored particles from the editing position, flying to the player camera, indicating the voxels are being collected from the terrain to the player's backpack.

- Little Pond, with SSR reflection so it can reflect on both the terrain and the flora

- Ocean view, more pixelized looking - Need more investigation on this one

- Clouds, with pixel vibe

- Randomized rocks, maybe through model import, or using SDF plainly (cleaner, supports random gen, but way harder to look good)

- More flora types

### Optimizes

- Optimize preculling: first we do tracing on terrain to update the depth buffer, so we can avoid most of the work when rendering the fragments (flora...), that are being occuluded by the voxel terrain.
