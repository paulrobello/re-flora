# PLANs

## Varying Colors

Phase 1

For floras, each instance has a unique seed, the goal is, to use the seed to determine a offset of the colors applied to the instance, in HSV color space (refer to color.glsl for existing color conversion code). The maximum amount of offset in H, S, V should be configurable separately. This offset serves as a global offset of color to each voxel in the instance.

After you are done, commit all for a checkpoint.

Phase 2

Use the same color offset technique to offset the color for each voxel within each instance too. This time, the seed is just the local position of each voxel combined with the instance's seed. The amount of offset in H, S, V should be configurable separately, and separate this variation config with Phase 1's

After you are done, commit all for a checkpoint.

After the implementation of each phase, do a cargo check, then before the commit, do a tools/format_all.bat

## Flying pedals

Now the flora would spawn particles too, just like the trees.

## Little creatures

Add little creatures like cicadas, bees, and butterflies to the world, it's spring time!

Each is just a single particle, accompanied with a spatial sound source attached to it.

For each creature, you can create a corresponding emitter type, to handle the spawn and update logic. Just somewhat like the fallen leaf emitter. Attach a gui config for each type of the spawner.

Bees would fly fast, buzzing (audio source not yet introduced, put a placeholder), and their movement should be restricted in a certain area.

Cicadas won't move at all, they just emit sounds.

For phase 1, implement the butterflies, they don't sound, they just move.

For phase 2, implement the cicadas, since we already have the audio assets already, they sound, but they don't move.

Even we implement these phases one by one, we need to consider all of them so that the design fits elegantly, remember, follow KISS.
