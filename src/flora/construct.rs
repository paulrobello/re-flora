use crate::tracer::voxel_encoding::append_indexed_cube_data;
use crate::tracer::Vertex;
use anyhow::Result;
use glam::IVec3;

pub fn gen_grass(is_lod_used: bool) -> Result<(Vec<Vertex>, Vec<u32>)> {
    const VOXEL_COUNT: u32 = 8;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for i in 0..VOXEL_COUNT {
        let vertex_offset = vertices.len() as u32;
        let base_pos = IVec3::new(0, i as i32, 0);

        // calculate color gradient: 0.0 for bottom (i=0), 1.0 for tip (i=voxel_count-1)
        let gradient = if VOXEL_COUNT > 1 {
            i as f32 / (VOXEL_COUNT - 1) as f32
        } else {
            0.0
        };

        append_indexed_cube_data(
            &mut vertices,
            &mut indices,
            base_pos,
            vertex_offset,
            gradient,
            gradient,
            is_lod_used,
        )?;
    }

    Ok((vertices, indices))
}

pub fn gen_lavender(is_lod_used: bool) -> Result<(Vec<Vertex>, Vec<u32>)> {
    const STEM_VOXEL_COUNT: u32 = 8;
    const LEAF_BALL_RADIUS: f32 = 2.0;
    const LEAF_BALL_BOUNDARY: i32 = LEAF_BALL_RADIUS as i32;
    const TOTAL_HEIGHT: u32 = STEM_VOXEL_COUNT + (LEAF_BALL_BOUNDARY * 2 + 1) as u32;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // draw the stem
    let total_stem_voxel_count = STEM_VOXEL_COUNT - LEAF_BALL_BOUNDARY as u32;
    for i in 0..total_stem_voxel_count {
        let vertex_offset = vertices.len() as u32;
        let base_pos = IVec3::new(0, i as i32, 0);

        // it never reaches 1, because 1 means the leaf ball, and we only need the shadow underneath it
        let mut color_gradient = i as f32 / total_stem_voxel_count as f32;
        color_gradient = color_gradient.powf(5.0);

        let wind_gradient = base_pos.y as f32 / TOTAL_HEIGHT as f32;

        append_indexed_cube_data(
            &mut vertices,
            &mut indices,
            base_pos,
            vertex_offset,
            color_gradient,
            wind_gradient,
            is_lod_used,
        )?;
    }

    // draw the leaf ball at the top of the stem
    for i in -LEAF_BALL_BOUNDARY..=LEAF_BALL_BOUNDARY {
        for j in -LEAF_BALL_BOUNDARY..=LEAF_BALL_BOUNDARY {
            for k in -LEAF_BALL_BOUNDARY..=LEAF_BALL_BOUNDARY {
                if i * i + j * j + k * k > LEAF_BALL_BOUNDARY * LEAF_BALL_BOUNDARY {
                    continue;
                }

                let vertex_offset = vertices.len() as u32;
                let base_pos = IVec3::new(i, j, k) + IVec3::new(0, STEM_VOXEL_COUNT as i32, 0);

                const COLOR_GRADIENT: f32 = 1.0;
                let wind_gradient = base_pos.y as f32 / TOTAL_HEIGHT as f32;
                append_indexed_cube_data(
                    &mut vertices,
                    &mut indices,
                    base_pos,
                    vertex_offset,
                    COLOR_GRADIENT,
                    wind_gradient,
                    is_lod_used,
                )?;
            }
        }
    }

    Ok((vertices, indices))
}

pub fn gen_ember_bloom(is_lod_used: bool) -> Result<(Vec<Vertex>, Vec<u32>)> {
    // Dimensions to keep it comparable to Lavender
    const STEM_HEIGHT: i32 = 6;
    // The bloom sits on top of the stem
    const BLOOM_RADIUS: i32 = 1;
    const BLOOM_HEIGHT: i32 = 4;

    let total_height = (STEM_HEIGHT + BLOOM_HEIGHT) as f32;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // 1. Generate the Stem
    // A simple vertical stalk, slightly darker at the bottom
    for y in 0..STEM_HEIGHT {
        let vertex_offset = vertices.len() as u32;
        let base_pos = IVec3::new(0, y, 0);

        // Stem Gradient: 0.0 (base) to 0.2 (top of stem).
        // Keeps it dark/ashy compared to the bright bloom.
        let color_gradient = (y as f32 / STEM_HEIGHT as f32) * 0.2;

        // Wind affects the top more
        let wind_gradient = base_pos.y as f32 / total_height;

        append_indexed_cube_data(
            &mut vertices,
            &mut indices,
            base_pos,
            vertex_offset,
            color_gradient,
            wind_gradient,
            is_lod_used,
        )?;
    }

    // 2. Generate the "Ember" Bloom
    // We create a symmetric bulb shape using Manhattan distance or radius checks
    let bloom_start_y = STEM_HEIGHT;

    for y in 0..BLOOM_HEIGHT {
        for x in -BLOOM_RADIUS..=BLOOM_RADIUS {
            for z in -BLOOM_RADIUS..=BLOOM_RADIUS {
                let local_y = y;

                // Shaping logic: Create a "Lantern" or "Bud" shape
                // The middle of the bloom (y=1 or 2) is widest.
                // The top and bottom are narrow.
                let is_center = x == 0 && z == 0;
                let is_corner = x.abs() == BLOOM_RADIUS && z.abs() == BLOOM_RADIUS;

                // Skip corners on the bottom and top layers to round it off
                if (local_y == 0 || local_y == BLOOM_HEIGHT - 1) && is_corner {
                    continue;
                }

                // Skip outer ring entirely on the very tip to make a point
                if local_y == BLOOM_HEIGHT - 1 && !is_center {
                    continue;
                }

                let vertex_offset = vertices.len() as u32;
                let base_pos = IVec3::new(x, bloom_start_y + y, z);
                let wind_gradient = base_pos.y as f32 / total_height;

                // 3. Artistic Coloring (Heat Gradient)
                // Logic: The center is the "core" (hottest/brightest).
                // The outside is the "crust" (cooler/darker).
                let color_gradient = if is_center {
                    // The core gets brighter as it goes up, like a flame
                    0.8 + (local_y as f32 / BLOOM_HEIGHT as f32) * 0.2
                } else {
                    // Outer petals are cooler (magma red/orange)
                    0.4 + (local_y as f32 / BLOOM_HEIGHT as f32) * 0.2
                };

                append_indexed_cube_data(
                    &mut vertices,
                    &mut indices,
                    base_pos,
                    vertex_offset,
                    color_gradient,
                    wind_gradient,
                    is_lod_used,
                )?;
            }
        }
    }

    // 3. Add small "Sepals" or thorns at the base of the bloom for detail
    // Small spikes sticking out just below the bloom
    let sepal_offsets = [
        IVec3::new(1, -1, 0),
        IVec3::new(-1, -1, 0),
        IVec3::new(0, -1, 1),
        IVec3::new(0, -1, -1),
    ];

    for offset in sepal_offsets {
        let vertex_offset = vertices.len() as u32;
        let base_pos = IVec3::new(0, bloom_start_y, 0) + offset;

        // Darker color for the protective leaves
        let color_gradient = 0.15;
        let wind_gradient = base_pos.y as f32 / total_height;

        append_indexed_cube_data(
            &mut vertices,
            &mut indices,
            base_pos,
            vertex_offset,
            color_gradient,
            wind_gradient,
            is_lod_used,
        )?;
    }

    Ok((vertices, indices))
}
