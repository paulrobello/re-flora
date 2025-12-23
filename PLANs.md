# PLANs

## Seed based flora color variation

For the little floras, add a technique to apply a variation on color for different instances, remove the gui control for the lavender color, instead, just use a random color selected from a predefined color palette hardcoded in shader, and map the seed to a color in the palette, using bucket sampling or whatever suits best.

We are currently having both the tip color and the bottom color for a given flora, we need to alter this data structure, introduce two seed fields: bottom_color_seed and tip_color_seed, and use them to sample the color from the color palette.

Right now, only the tip color variation is required, but we can easily extend it to the bottom color as well later on. So just keep in mind.

Oh, for the leaves generated, they also belong to the flora system, we should add some variations for their colors as well!

In later phases, only a color palette is not enough, we would add a technique to offset their color picked from color palette in HSV field, to make them look more distinct and vivid. But not in this turn.

## Little creatures

Add little creatures like cicadas, bees, and butterflies to the world, it's spring time!

Each is just a single particle, accompanied with a spatial sound source attached to it.

Bees would fly fast, buzzing (audio source not yet introduced, put a placeholder), and their movement should be restricted in a certain area.

Cicadas won't move at all, they just emit sounds.

For now, just implement the bees, and without any spatial sound, add it in later phases.

## Grass System

We already have grasses, as drawn by flora system. However the density of the grasses is not enough. We need to increase the density of the grasses.

Also, the grasses are of the same height, which needs some more variations to make it more realistic.

We have two plans for this:

1. Create different types of grasses model, with different heights, and spawn them randomly.
2. Use a single grass model, but degenerate some of the topped voxels based on the real height, in flora.vert.

Analyze which is better, the first one seems has better perf, but the second one is more flexible.
