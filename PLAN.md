# PLAN

Currently, we are using a u8 chunk for occupancy data for the floras, where each stores either 0 or 1 indicating if there are flora on the given position.

However, our flora instances have a term storing the plant tick in a u32, so in the vert/frag stages we can use this data to determine the active height of the plant.

There's a need for us to update the occupancy stages, so it support carring that information.

The plan:

1. Use u32 for the occupancy data instead, where you use 0x00000000 as the special one that denotes there's no flora presented.

2. For storing a valid flora in this 3d grid, you add the tick number by 1 to avoid collapsing with the special 0 as noted in point 1.

3. For flora editing taken in place, just make sure that:

   a. In removal stages, wipe relevant region of data to 0, regardless of what is originally stored inside.

   b. In addition stages, in affacted range, if the data is not 0, do not modify, otherwise use the current tick (plus 1, as noted in 2).

DO IT LATER:

Add a third tool to the bottom toolbox, that functions as a trim tool for the flora.

This tool is used to trim affacted region to a flora age, which is effectivelly setting the tick stored in occupancy data to: current_tick - target_age

This gives a target_age to all affacted regions
