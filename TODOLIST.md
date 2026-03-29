# PLANs

## Priorities - From High to Low

- Correct butterfly color palettes

- Correct butterfly fly pattern: inspect the perlin worm, i want to change the noise design to be octave=2, so the broad one is for determine the butterfly's direction, the detail one is for mimicing the random pattern the butterfly often creates.

- Optimize preculling: first we do tracing on terrain to update the depth buffer, so we can avoid most of the work when rendering the fragments (flora...), that are being occuluded by the voxel terrain.

- Correct sky color (more clear looking)

- Little Pond, with SSR reflection so it can reflect on both the terrain and the flora

- Camera shaking design during movement, like other first person adventure games, adjustable movement scale in gui

- Ocean view, more pixelized looking - Need more investigation on this one

- Clouds, pixel vibe

- Randomized rocks, maybe through model import, or using SDF plainly (cleaner, supports random gen, but way harder to look good)

- More flora types
