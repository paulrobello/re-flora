## Grass Variety Plan

1. Rename the current normal grass model to `tall grass` and keep its existing 8-voxel height.
2. Add a new normal grass variant named `short grass` with a 4-voxel height.
3. Update normal grass placement so only 30% of the current grass instances remain `tall grass`.
4. Fill the remaining normal grass coverage with `short grass`, generated at a much higher density than the current layout.
5. Verify the new distribution increases overall grass density, improves visual variety, and ideally reduces total rendered vertices despite the denser field.
