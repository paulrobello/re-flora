# PLANs

## Priorities - From High to Low

### Bugfixes

- For saving on gui config, refer to:
something like: 0.05000000074505806
you should round it reasonably so we don't get this kind of precesion issue

### Features

- Camera shaking design during movement, like other first person adventure games, adjustable movement scale in gui

- Correct sky color (more clear looking)

- Little Pond, with SSR reflection so it can reflect on both the terrain and the flora

- Ocean view, more pixelized looking - Need more investigation on this one

- Clouds, pixel vibe

- Randomized rocks, maybe through model import, or using SDF plainly (cleaner, supports random gen, but way harder to look good)

- More flora types

### Optimizes

- Optimize preculling: first we do tracing on terrain to update the depth buffer, so we can avoid most of the work when rendering the fragments (flora...), that are being occuluded by the voxel terrain.
