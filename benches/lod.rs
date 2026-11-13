use binary_greedy_meshing as bgm;
#[path = "../utils/model_utils.rs"]
mod model_utils;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use model_utils::*;
use std::{collections::BTreeSet, hint::black_box};
const CS: usize = 62;

fn terrain_lod<const LOD: usize>(c: &mut Criterion) {
    let voxels = natural_terrain::<CS>();
    let mut mesher = bgm::Mesher::<CS>::new();
    let transparents = BTreeSet::new();
    c.bench_function(&format!("terrain_lod {LOD}"), |b| {
        // mesh_lod mutates the buffer, so each iteration needs a fresh copy
        b.iter_batched_ref(
            || voxels.clone(),
            |voxels| {
                mesher.clear();
                mesher.mesh_lod(
                    black_box(&mut voxels[..]),
                    black_box(&transparents),
                    black_box(LOD),
                );
            },
            BatchSize::LargeInput,
        )
    });
}

criterion_group!(lod, terrain_lod::<2>, terrain_lod::<4>, terrain_lod::<8>);
criterion_main!(lod);
