use anyhow::Result;
use glam::{IVec3, UVec3};

use crate::tracer::{
    voxel_geometry::{CUBE_INDICES, CUBE_INDICES_LOD, VOXEL_VERTICES, VOXEL_VERTICES_LOD},
    Vertex,
};

const BIT_PER_POS: u32 = 7;
const BIT_PER_OFFSET: u32 = 1;
const BIT_PER_ORIGIN: u32 = 7;

const POS_BITS: u32 = BIT_PER_POS * 3;
const OFFSET_BITS: u32 = BIT_PER_OFFSET * 3;
const ORIGIN_BITS: u32 = BIT_PER_ORIGIN * 3;
const MAX_LENGTH_BITS: u32 = 32 - ORIGIN_BITS;

const MAX_LENGTH_MASK: u32 = (1 << MAX_LENGTH_BITS) - 1;

/// Encodes a position into BIT_PER_POS * 3 bits.
fn encode_pos(pos: IVec3) -> Result<u32> {
    encode_vec3_with_bits(pos, BIT_PER_POS, "local position")
}

/// Encodes a voxel offset (within a unit cube) into BIT_PER_OFFSET bits.
fn encode_voxel_offset(base_vert: UVec3) -> Result<u32> {
    const UPPER_BOUND: u32 = (1 << BIT_PER_OFFSET) - 1;
    if base_vert.x > UPPER_BOUND || base_vert.y > UPPER_BOUND || base_vert.z > UPPER_BOUND {
        return Err(anyhow::anyhow!("Invalid base vert"));
    }
    let encoded =
        base_vert.x | (base_vert.y << BIT_PER_OFFSET) | (base_vert.z << (BIT_PER_OFFSET * 2));
    Ok(encoded)
}

fn encode_origin(origin: IVec3) -> Result<u32> {
    encode_vec3_with_bits(origin, BIT_PER_ORIGIN, "origin")
}

fn encode_max_length(max_length: u32) -> Result<u32> {
    if max_length == 0 {
        return Err(anyhow::anyhow!("max_length must be greater than zero"));
    }
    if max_length > MAX_LENGTH_MASK {
        return Err(anyhow::anyhow!(
            "max_length {} exceeds encodable range (max {})",
            max_length,
            MAX_LENGTH_MASK
        ));
    }
    Ok(max_length)
}

fn make_value_from_parts(
    encoded_pos: u32,
    encoded_offset: u32,
    encoded_origin: u32,
    encoded_max_length: u32,
) -> [u32; 2] {
    let packed_pos_and_offset = encoded_pos | (encoded_offset << POS_BITS);
    let packed_origin_and_length = encoded_origin | (encoded_max_length << ORIGIN_BITS);
    [packed_pos_and_offset, packed_origin_and_length]
}

fn encode_vec3_with_bits(pos: IVec3, bits: u32, label: &str) -> Result<u32> {
    let offset: i32 = 1 << (bits - 1);
    let pos = pos + IVec3::splat(offset);

    let lower_bound: i32 = 0;
    let upper_bound: i32 = (1 << bits) - 1;
    if pos.x < lower_bound
        || pos.x > upper_bound
        || pos.y < lower_bound
        || pos.y > upper_bound
        || pos.z < lower_bound
        || pos.z > upper_bound
    {
        return Err(anyhow::anyhow!(
            "Invalid {} {:?}",
            label,
            pos - IVec3::splat(offset)
        ));
    }
    let pos = pos.as_uvec3(); // this is safe now
    let encoded = pos.x | (pos.y << bits) | (pos.z << (bits * 2));
    Ok(encoded)
}

/// Appends 8 vertices and 36 indices for a single cube to the provided lists.
pub fn append_indexed_cube_data(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    pos: IVec3,
    vertex_offset: u32,
    origin: IVec3,
    max_length: u32,
    is_lod_used: bool,
) -> Result<()> {
    const LOWER_BOUND: i32 = -(1 << (BIT_PER_POS - 1));
    const UPPER_BOUND: i32 = (1 << (BIT_PER_POS - 1)) - 1;
    if pos.x < LOWER_BOUND
        || pos.x > UPPER_BOUND
        || pos.y < LOWER_BOUND
        || pos.y > UPPER_BOUND
        || pos.z < LOWER_BOUND
        || pos.z > UPPER_BOUND
    {
        return Err(anyhow::anyhow!("Invalid local position"));
    }

    let encoded_pos = encode_pos(pos)?;
    let encoded_origin = encode_origin(origin)?;
    let encoded_max_length = encode_max_length(max_length)?;

    let voxel_verts: Vec<UVec3> = if is_lod_used {
        VOXEL_VERTICES_LOD.to_vec()
    } else {
        VOXEL_VERTICES.to_vec()
    };
    let base_indices = if is_lod_used {
        CUBE_INDICES_LOD.to_vec()
    } else {
        CUBE_INDICES.to_vec()
    };

    for voxel_vert in voxel_verts {
        let encoded_offset = encode_voxel_offset(voxel_vert)?;
        let packed_data = make_value_from_parts(
            encoded_pos,
            encoded_offset,
            encoded_origin,
            encoded_max_length,
        );
        vertices.push(Vertex { packed_data });
    }
    for index in base_indices {
        indices.push(vertex_offset + index);
    }

    Ok(())
}
