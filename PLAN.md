# Coding Plans

## Particle System (CPU-first)

currently, we are passing Vec3 for each partical position to GPU, but we are clamping its position anyway inside the partical.vert shader, so my thoughts are, to only pass integer
  position into the GPU, for lowering the bandwidth consumption. see in_instance_pos for more reference, we are using uvec3 position there. we can utilize the same thing too for our
  particle system.
