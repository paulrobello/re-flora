for grasses, we want to have more varieties. currently we have grass instance and grass voxel hashes that can be used to
determine the color to use during rendering. however this is too noisy while successfully adding varieties.
for now, keep this logic but turn the default of the varieties across instances and in a instance (voxel hash), to 0.
since we are going to implement another color patching logic.

the new logic:

we use 2d perlin noise with a ocatave of 3 as default. implement this in vert shader directly so we can fast prototyping
but this step can be later changed to be called during flora generation step to avoid doing in every single frame.

we then hard code a color LUT table in shader side to guide how to interpret the generated float to a color. we can use a color band of 3,
where we just use nearest sampling without interpolation. (toon shading alike)
