# PLANs

## Priorities - From High to Low

- Correct butterfly color palettes

- Correct butterfly fly pattern: inspect the perlin worm, i want to change the noise design to be octave=2, so the broad one is for determine the butterfly's direction, the detail one is for mimicing the random pattern the butterfly often creates.

- Optimize preculling: first we do tracing on terrain to update the depth buffer, so we can avoid most of the work when rendering the fragments (flora...), that are being occuluded by the voxel terrain.

- Correct sky color (more clean looking)

- Pond, with SSR reflection

- Camera shaking during movement

- Ocean view, more pixelized looking - Need more thoughts on this one

- Clouds

- Randomized rocks

- More flora types
