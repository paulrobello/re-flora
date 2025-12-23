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
    const LEAF_BALL_RADIUS: f32 = 1.5;
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

use std::f32::consts::PI;

pub fn gen_ember_bloom(is_lod_used: bool) -> Result<(Vec<Vertex>, Vec<u32>)> {
    const HEIGHT: i32 = 15;
    // Width Configuration: How wide the plant swells
    const MAX_RADIUS: f32 = 2.5;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for y in 0..HEIGHT {
        // Normalized height (0.0 at bottom, 1.0 at top)
        let t = y as f32 / HEIGHT as f32;

        // Vertical Profile:
        // Uses a sine wave to create a soft bulb shape.
        // It starts small, swells wide in the middle, and tapers at the top.
        // We add 0.5 base radius so it doesn't disappear completely at the very bottom.
        let vertical_swell = (t * PI).sin();
        let base_radius = 0.5 + (vertical_swell * MAX_RADIUS);

        // Define search area for this layer
        let search_radius = (base_radius + 1.5).ceil() as i32;

        for x in -search_radius..=search_radius {
            for z in -search_radius..=search_radius {
                // Calculate distance from center (0,0)
                let dist_sq = (x * x + z * z) as f32;
                let dist = dist_sq.sqrt();

                // Calculate Angle for the "Wavy" texture
                let angle = (z as f32).atan2(x as f32);

                // Wavy/Leafy Logic:
                // We use cos(angle * 6.0) to create 6 gentle lobes (leaves) wrapping around.
                // 'lobe_depth' controls how deep the ridges are.
                let lobe_depth = 0.6;
                let wave_modifier = (angle * 6.0).cos() * lobe_depth;

                // The effective radius limit at this specific angle
                let radius_limit = base_radius + wave_modifier;

                // Solid Fill Logic:
                // We fill everything inside the calculated radius.
                // This guarantees the shape is symmetrical and has no holes.
                if dist <= radius_limit {
                    let vertex_offset = vertices.len() as u32;

                    // No stem sway, just straight up for symmetry
                    let pos = IVec3::new(x, y, z);

                    // Color Logic:
                    // 0.0 at bottom -> 1.0 at top.
                    // We add a slight highlight to the "ridges" of the waves to give it depth.
                    let ridge_highlight = wave_modifier * 0.1;
                    let color_gradient = (t + ridge_highlight).clamp(0.0, 1.0);

                    // Wind Logic:
                    // The top moves more than the bottom.
                    let wind_gradient = pos.y as f32 / HEIGHT as f32;

                    append_indexed_cube_data(
                        &mut vertices,
                        &mut indices,
                        pos,
                        vertex_offset,
                        color_gradient,
                        wind_gradient,
                        is_lod_used,
                    )?;
                }
            }
        }
    }

    Ok((vertices, indices))
}
