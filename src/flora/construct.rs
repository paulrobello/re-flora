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
    const STEM_HEIGHT: i32 = 6;
    const BLOOM_HEIGHT: i32 = 3;
    const BASE_LEAF_LAYERS: i32 = 2;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let total_height = (STEM_HEIGHT + BLOOM_HEIGHT + BASE_LEAF_LAYERS) as f32;

    // three stems arranged in an L-shape so the plant feels asymmetric
    let stem_offsets = [
        IVec3::new(0, 0, 0),
        IVec3::new(1, 0, 0),
        IVec3::new(0, 0, 1),
    ];

    for (stem_idx, stem_offset) in stem_offsets.iter().enumerate() {
        for y in 0..STEM_HEIGHT {
            let vertex_offset = vertices.len() as u32;
            let base_pos = *stem_offset + IVec3::new(0, y, 0);
            let color_gradient = (y as f32 / total_height).min(1.0);
            let wind_gradient = (y as f32 / total_height).min(1.0);

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

        for y in 0..BLOOM_HEIGHT {
            let vertex_offset = vertices.len() as u32;
            let height = STEM_HEIGHT + y;
            let base_pos = *stem_offset + IVec3::new(0, height, 0);
            let mut color_gradient = 0.7 + (y as f32 / BLOOM_HEIGHT as f32) * 0.3;
            color_gradient += stem_idx as f32 * 0.02;
            color_gradient = color_gradient.min(1.0);
            let wind_gradient = (height as f32 / total_height).min(1.0);

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

        // add petals radiating from the hottest bloom cube
        let bloom_top = *stem_offset + IVec3::new(0, STEM_HEIGHT + BLOOM_HEIGHT - 1, 0);
        let petal_offsets = [
            IVec3::new(1, 0, 0),
            IVec3::new(-1, 0, 0),
            IVec3::new(0, 0, 1),
            IVec3::new(0, 0, -1),
        ];

        for petal in petal_offsets {
            let vertex_offset = vertices.len() as u32;
            let base_pos = bloom_top + petal;
            let color_gradient = 0.85;
            let wind_gradient = ((bloom_top.y as f32) / total_height).clamp(0.0, 1.0);

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

    // layered leaves close to the ground
    for layer in 0..BASE_LEAF_LAYERS {
        let spread = 2 + layer;
        let base_height = layer;
        let color_gradient = 0.15 + (layer as f32 / (BASE_LEAF_LAYERS as f32 * 3.0));
        let wind_gradient = (base_height as f32 / total_height).min(1.0);

        let ring_offsets = [
            IVec3::new(spread, 0, 0),
            IVec3::new(-spread, 0, 0),
            IVec3::new(0, 0, spread),
            IVec3::new(0, 0, -spread),
            IVec3::new(spread, 0, spread),
            IVec3::new(-spread, 0, spread),
            IVec3::new(spread, 0, -spread),
            IVec3::new(-spread, 0, -spread),
        ];

        for offset in ring_offsets {
            let vertex_offset = vertices.len() as u32;
            let base_pos = offset + IVec3::new(0, base_height, 0);

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

    Ok((vertices, indices))
}
