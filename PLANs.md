# PLANs

## Some bugfixes and enhancements

in in_packed_data, we have encoded o_color_gradient and o_wind_gradient in it, however, i think this is not flexible enough to encode these from CPU side, instead, just follow KISS principle, store the origin of the geometry, with a ivec3, and a max_length with a uint. therefore we can calculate for the gradients by using the vox_local_pos, origin, to calculate the distance from the vox to the center d, then d / max_length is the gradient we are going to use (normally), in the case of the grass, where we are having various lengths, we are calculating the gradient in the same way (where the denom is still the max_length, instead of current grass blade's actual length)

after changing the structure, a single uint may not be enough. feel free to extend the structure.

## Little creatures

Add little creatures like cicadas, bees, and butterflies to the world, it's spring time!

Each is just a single particle, accompanied with a spatial sound source attached to it.

Bees would fly fast, buzzing (audio source not yet introduced, put a placeholder), and their movement should be restricted in a certain area.

Cicadas won't move at all, they just emit sounds.

For now, just implement the bees, and without any spatial sound, add it in later phases.
