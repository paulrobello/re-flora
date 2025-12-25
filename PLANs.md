# PLANs

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
