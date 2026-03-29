# PLANs

## Priorities - From High to Low

### Bugfixes

### Features

- Add tool to add dirt

- Add a backpack to show the collected voxels count, (count each voxel type separately), when removing dirt, add to the backpack, when adding dirt to terrain, decrease from backpack, so total voxel counts can remain

- Little Pond, with SSR reflection so it can reflect on both the terrain and the flora

- Ocean view, more pixelized looking - Need more investigation on this one

- Clouds, with pixel vibe

- Randomized rocks, maybe through model import, or using SDF plainly (cleaner, supports random gen, but way harder to look good)

- More flora types

### Optimizes

- Optimize preculling: first we do tracing on terrain to update the depth buffer, so we can avoid most of the work when rendering the fragments (flora...), that are being occuluded by the voxel terrain.
