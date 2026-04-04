use crate::app::world_edits::{BuildEdit, VoxelEdit, WorldBuildBackend, WorldEditPlan};
use crate::builder::{
    ContreeBuilder, PlainBuilder, SceneAccelBuilder, SurfaceBuilder, VOXEL_TYPE_CHERRY_WOOD,
};
use crate::geom::UAabb3;
use crate::util::BENCH;
use anyhow::Result;
use glam::{UVec3, Vec3};
use std::time::Instant;

pub(crate) struct FloraSphereEdit {
    pub(crate) center: Vec3,
    pub(crate) radius: f32,
    pub(crate) tick: u32,
}

#[allow(dead_code)]
struct BuilderOnlyWorldBackend<'a> {
    plain_builder: &'a mut PlainBuilder,
    surface_builder: &'a mut SurfaceBuilder,
    contree_builder: &'a mut ContreeBuilder,
    scene_accel_builder: &'a mut SceneAccelBuilder,
    voxel_dim_per_chunk: UVec3,
}

impl WorldBuildBackend for BuilderOnlyWorldBackend<'_> {
    fn apply_voxel_edit(&mut self, edit: VoxelEdit) -> Result<()> {
        apply_voxel_edit(self.plain_builder, edit)
    }

    fn apply_build_edit(&mut self, edit: BuildEdit) -> Result<()> {
        apply_build_edit(
            self.surface_builder,
            self.contree_builder,
            self.scene_accel_builder,
            self.voxel_dim_per_chunk,
            edit,
        )
    }
}

pub(crate) fn apply_voxel_edit(plain_builder: &mut PlainBuilder, edit: VoxelEdit) -> Result<()> {
    match edit {
        VoxelEdit::ClearVoxelRegion(edit) => plain_builder.chunk_init(edit.offset, edit.dim),
        VoxelEdit::StampRoundCones {
            bvh_nodes,
            round_cones,
            voxel_type,
        } => {
            if voxel_type == VOXEL_TYPE_CHERRY_WOOD {
                plain_builder.chunk_modify(&bvh_nodes, &round_cones)
            } else {
                plain_builder.chunk_modify_with_voxel_type(&bvh_nodes, &round_cones, voxel_type)
            }
        }
        VoxelEdit::StampCuboids {
            bvh_nodes,
            cuboids,
            voxel_type,
        } => {
            if voxel_type == VOXEL_TYPE_CHERRY_WOOD {
                plain_builder.chunk_modify_cuboids(&bvh_nodes, &cuboids)
            } else {
                plain_builder.chunk_modify_cuboids_with_voxel_type(&bvh_nodes, &cuboids, voxel_type)
            }
        }
        VoxelEdit::StampSurfaceSpheres {
            bvh_nodes,
            spheres,
            voxel_type,
        } => plain_builder
            .chunk_modify_surface_spheres_with_voxel_type(&bvh_nodes, &spheres, voxel_type, None)
            .map(|_| ()),
    }
}

pub(crate) fn apply_build_edit(
    surface_builder: &mut SurfaceBuilder,
    contree_builder: &mut ContreeBuilder,
    scene_accel_builder: &mut SceneAccelBuilder,
    voxel_dim_per_chunk: UVec3,
    edit: BuildEdit,
) -> Result<()> {
    match edit {
        BuildEdit::RebuildMesh(bound) => mesh_generate(
            surface_builder,
            contree_builder,
            scene_accel_builder,
            voxel_dim_per_chunk,
            bound,
        ),
    }
}

#[allow(dead_code)]
pub(crate) fn execute_edit_plan_on_builders(
    plain_builder: &mut PlainBuilder,
    surface_builder: &mut SurfaceBuilder,
    contree_builder: &mut ContreeBuilder,
    scene_accel_builder: &mut SceneAccelBuilder,
    voxel_dim_per_chunk: UVec3,
    plan: WorldEditPlan,
) -> Result<()> {
    let mut backend = BuilderOnlyWorldBackend {
        plain_builder,
        surface_builder,
        contree_builder,
        scene_accel_builder,
        voxel_dim_per_chunk,
    };
    execute_edit_plan_on_backend(&mut backend, plan)
}

pub(crate) fn execute_edit_plan_on_backend<B: WorldBuildBackend>(
    backend: &mut B,
    plan: WorldEditPlan,
) -> Result<()> {
    for edit in plan.voxel_edits {
        backend.apply_voxel_edit(edit)?;
    }

    for edit in plan.build_edits {
        backend.apply_build_edit(edit)?;
    }

    Ok(())
}

#[allow(dead_code)]
pub(crate) fn init_chunk_by_chunk(
    plain_builder: &mut PlainBuilder,
    surface_builder: &mut SurfaceBuilder,
    contree_builder: &mut ContreeBuilder,
    scene_accel_builder: &mut SceneAccelBuilder,
    voxel_dim_per_chunk: UVec3,
    world_dim: UVec3,
) -> Result<()> {
    let world_bound = UAabb3::new(UVec3::ZERO, world_dim - UVec3::ONE);
    let chunk_indices =
        get_affected_chunk_indices(world_bound.min(), world_bound.max(), voxel_dim_per_chunk);

    let total = chunk_indices.len();
    for (i, chunk_id) in chunk_indices.into_iter().enumerate() {
        let atlas_offset = chunk_id * voxel_dim_per_chunk;

        log::info!(
            "[INIT] chunk {}/{}/{chunk_id:?} generating terrain...",
            i + 1,
            total
        );
        let now = Instant::now();
        plain_builder.chunk_init(atlas_offset, voxel_dim_per_chunk)?;
        BENCH.lock().unwrap().record("chunk_init", now.elapsed());

        log::info!(
            "[INIT] chunk {}/{}/{chunk_id:?} build_surface...",
            i + 1,
            total
        );
        let now = Instant::now();
        let res = surface_builder.build_surface(chunk_id, true);
        if let Err(e) = res {
            log::error!("Failed to build surface for chunk {chunk_id:?}: {e}");
            continue;
        }
        BENCH.lock().unwrap().record("build_surface", now.elapsed());

        log::info!(
            "[INIT] chunk {}/{}/{chunk_id:?} build_and_alloc...",
            i + 1,
            total
        );
        let now = Instant::now();
        let res = contree_builder.build_and_alloc(atlas_offset).unwrap();
        BENCH
            .lock()
            .unwrap()
            .record("build_and_alloc", now.elapsed());

        if let Some((node_buffer_offset, leaf_buffer_offset)) = res {
            scene_accel_builder.update_scene_tex(
                chunk_id,
                node_buffer_offset,
                leaf_buffer_offset,
            )?;
        }

        log::info!("[INIT] chunk {}/{}/{chunk_id:?} done.", i + 1, total);
    }

    Ok(())
}

pub(crate) fn mesh_generate(
    surface_builder: &mut SurfaceBuilder,
    contree_builder: &mut ContreeBuilder,
    scene_accel_builder: &mut SceneAccelBuilder,
    voxel_dim_per_chunk: UVec3,
    bound: UAabb3,
) -> Result<()> {
    let affected_chunk_indices =
        get_affected_chunk_indices(bound.min(), bound.max(), voxel_dim_per_chunk);

    let total_chunks = affected_chunk_indices.len();
    for (i, chunk_id) in affected_chunk_indices.into_iter().enumerate() {
        let atlas_offset = chunk_id * voxel_dim_per_chunk;

        log::debug!(
            "[MESH] chunk {}/{} id={} build_surface...",
            i + 1,
            total_chunks,
            chunk_id
        );
        let now = Instant::now();
        let res = surface_builder.build_surface(chunk_id, true);
        if let Err(e) = res {
            log::error!("Failed to build surface for chunk {}: {}", chunk_id, e);
            continue;
        }
        BENCH.lock().unwrap().record("build_surface", now.elapsed());

        let now = Instant::now();
        let res = contree_builder.build_and_alloc(atlas_offset).unwrap();
        BENCH
            .lock()
            .unwrap()
            .record("build_and_alloc", now.elapsed());

        if let Some((node_buffer_offset, leaf_buffer_offset)) = res {
            scene_accel_builder.update_scene_tex(
                chunk_id,
                node_buffer_offset,
                leaf_buffer_offset,
            )?;
        }
        log::debug!(
            "[MESH] chunk {}/{} id={} done.",
            i + 1,
            total_chunks,
            chunk_id
        );
    }

    Ok(())
}

pub(crate) fn mesh_generate_preserve_flora_for_sphere_edit(
    surface_builder: &mut SurfaceBuilder,
    contree_builder: &mut ContreeBuilder,
    scene_accel_builder: &mut SceneAccelBuilder,
    voxel_dim_per_chunk: UVec3,
    bound: UAabb3,
    flora_edit: FloraSphereEdit,
) -> Result<()> {
    let affected_chunk_indices =
        get_affected_chunk_indices(bound.min(), bound.max(), voxel_dim_per_chunk);

    for chunk_id in affected_chunk_indices {
        let atlas_offset = chunk_id * voxel_dim_per_chunk;

        // Submit surface build without waiting — GPU queue ordering ensures
        // it completes before subsequent same-queue submissions (flora edit,
        // contree build). This eliminates one wait_queue_idle per chunk.
        if let Err(e) = surface_builder.build_surface_submit_only(chunk_id) {
            log::error!("Failed to build surface for chunk {}: {}", chunk_id, e);
            continue;
        }

        surface_builder.edit_flora_instances(
            chunk_id,
            flora_edit.center,
            flora_edit.radius,
            flora_edit.tick,
        )?;

        let res = contree_builder.build_and_alloc(atlas_offset).unwrap();

        if let Some((node_buffer_offset, leaf_buffer_offset)) = res {
            scene_accel_builder.update_scene_tex(
                chunk_id,
                node_buffer_offset,
                leaf_buffer_offset,
            )?;
        }
    }

    Ok(())
}

pub(crate) fn mesh_regenerate_flora_for_sphere_edit(
    surface_builder: &mut SurfaceBuilder,
    voxel_dim_per_chunk: UVec3,
    bound: UAabb3,
    flora_edit: FloraSphereEdit,
) -> Result<()> {
    let affected_chunk_indices =
        get_affected_chunk_indices(bound.min(), bound.max(), voxel_dim_per_chunk);

    for chunk_id in affected_chunk_indices {
        let now = Instant::now();
        let res = surface_builder.build_surface(chunk_id, false);
        if let Err(e) = res {
            log::error!("Failed to build surface for chunk {}: {}", chunk_id, e);
            continue;
        }
        BENCH.lock().unwrap().record("build_surface", now.elapsed());

        let _regen_stats = surface_builder.regenerate_flora_instances(
            chunk_id,
            flora_edit.center,
            flora_edit.radius,
            flora_edit.tick,
        )?;
    }

    Ok(())
}

pub(crate) fn mesh_trim_flora_for_sphere_edit(
    surface_builder: &mut SurfaceBuilder,
    voxel_dim_per_chunk: UVec3,
    bound: UAabb3,
    flora_edit: FloraSphereEdit,
    target_age: u32,
) -> Result<()> {
    let affected_chunk_indices =
        get_affected_chunk_indices(bound.min(), bound.max(), voxel_dim_per_chunk);

    for chunk_id in affected_chunk_indices {
        let now = Instant::now();
        let res = surface_builder.build_surface(chunk_id, false);
        if let Err(e) = res {
            log::error!("Failed to build surface for chunk {}: {}", chunk_id, e);
            continue;
        }
        BENCH.lock().unwrap().record("build_surface", now.elapsed());

        let _regen_stats = surface_builder.trim_flora_instances(
            chunk_id,
            flora_edit.center,
            flora_edit.radius,
            flora_edit.tick,
            target_age,
        )?;
    }

    Ok(())
}

pub(crate) fn get_affected_chunk_indices(
    min_bound: UVec3,
    max_bound: UVec3,
    voxel_dim_per_chunk: UVec3,
) -> Vec<UVec3> {
    let min_chunk_idx = min_bound / voxel_dim_per_chunk;
    let max_chunk_idx = max_bound / voxel_dim_per_chunk;

    let mut affected = Vec::new();
    for x in min_chunk_idx.x..=max_chunk_idx.x {
        for y in min_chunk_idx.y..=max_chunk_idx.y {
            for z in min_chunk_idx.z..=max_chunk_idx.z {
                affected.push(UVec3::new(x, y, z));
            }
        }
    }
    affected
}
