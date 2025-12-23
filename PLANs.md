# PLANs

## Grass System

We already have grasses, as drawn by flora system. However the density of the grasses is not enough. We need to increase the density of the grasses.

Also, the grasses are of the same height, which needs some more variations to make it more realistic.

We have two plans for this:

1. Create different types of grasses model, with different heights, and spawn them randomly.
2. Use a single grass model, but degenerate some of the topped voxels based on the real height, in flora.vert.

Analyze which is better, the first one seems has better perf, but the second one is more flexible.

## Little creatures

Add little creatures like cicadas, bees, and butterflies to the world, it's spring time!

Each is just a single particle, accompanied with a spatial sound source attached to it.

Bees would fly fast, buzzing (audio source not yet introduced, put a placeholder), and their movement should be restricted in a certain area.

Cicadas won't move at all, they just emit sounds.

For now, just implement the bees, and without any spatial sound, add it in later phases.
