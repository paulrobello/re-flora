use crate::flora::construct::{gen_grass, gen_lavender};
use crate::tracer::Vertex;
use anyhow::Result;

pub const MAX_FLORA_SPECIES: usize = 2;

pub type MeshGeneratorFn = fn(bool) -> Result<(Vec<Vertex>, Vec<u32>)>;

#[derive(Clone, Copy)]
pub struct FloraSpeciesDesc {
    pub key: &'static str,
    #[allow(dead_code)]
    pub display_name: &'static str,
    pub default_bottom_color: [u8; 3],
    pub default_tip_color: [u8; 3],
    pub mesh_generator: MeshGeneratorFn,
}

impl FloraSpeciesDesc {
    pub const fn new(
        key: &'static str,
        display_name: &'static str,
        default_bottom_color: [u8; 3],
        default_tip_color: [u8; 3],
        mesh_generator: MeshGeneratorFn,
    ) -> Self {
        Self {
            key,
            display_name,
            default_bottom_color,
            default_tip_color,
            mesh_generator,
        }
    }
}

pub const FLORA_SPECIES: &[FloraSpeciesDesc] = &[
    FloraSpeciesDesc::new("grass", "Grass", [61, 163, 59], [168, 227, 0], gen_grass),
    FloraSpeciesDesc::new(
        "lavender",
        "Lavender",
        [74, 165, 0],
        [85, 0, 207],
        gen_lavender,
    ),
];

pub fn species() -> &'static [FloraSpeciesDesc] {
    FLORA_SPECIES
}

pub fn species_count() -> usize {
    FLORA_SPECIES.len()
}

pub fn assert_species_limit() {
    assert!(
        species_count() <= MAX_FLORA_SPECIES,
        "Defined {} flora species but MAX_FLORA_SPECIES is {}",
        species_count(),
        MAX_FLORA_SPECIES
    );
}
