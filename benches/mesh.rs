use std::{collections::BTreeSet, hint::black_box};
#[path = "../utils/model_utils.rs"]
mod model_utils;
use binary_greedy_meshing as bgm;
use criterion::{Criterion, criterion_group, criterion_main};
use model_utils::*;
const CS: usize = 62;

fn fast_mesh_opaque_sphere(c: &mut Criterion) {
    let voxels = opaque_sphere::<CS>(CS / 2);
    let mut mesher = bgm::Mesher::<CS>::new();
    let opaque_mask = bgm::compute_opaque_mask::<CS>(&voxels, &BTreeSet::new());
    let trans_mask = vec![0; bgm::Mesher::<CS>::CS_P2].into_boxed_slice();
    c.bench_function("fast_mesh_opaque_sphere", |b| {
        b.iter(|| {
            mesher.clear();
            mesher.fast_mesh(
                black_box(&voxels),
                black_box(&opaque_mask),
                black_box(&trans_mask),
            );
        })
    });
}

fn mesh_opaque_sphere(c: &mut Criterion) {
    let voxels = opaque_sphere::<CS>(CS / 2);
    let mut mesher = bgm::Mesher::<CS>::new();
    let transparents = BTreeSet::new();
    c.bench_function("mesh_opaque_sphere", |b| {
        b.iter(|| {
            mesher.clear();
            mesher.mesh(black_box(&voxels), black_box(&transparents));
        })
    });
}

fn fast_mesh_transparent_sphere(c: &mut Criterion) {
    let voxels = half_transparent_sphere::<CS>(CS / 2);
    let mut mesher = bgm::Mesher::<CS>::new();
    let transparent_blocks = BTreeSet::from([2]);
    let opaque_mask = bgm::compute_opaque_mask::<CS>(&voxels, &BTreeSet::new());
    let trans_mask = bgm::compute_transparent_mask::<CS>(&voxels, &transparent_blocks);
    c.bench_function("fast_mesh_transparent_sphere", |b| {
        b.iter(|| {
            mesher.clear();
            mesher.fast_mesh(
                black_box(&voxels),
                black_box(&opaque_mask),
                black_box(&trans_mask),
            );
        })
    });
}

fn mesh_transparent_sphere(c: &mut Criterion) {
    let voxels = half_transparent_sphere::<CS>(CS / 2);
    let mut mesher = bgm::Mesher::<CS>::new();
    let transparent_blocks = BTreeSet::from([2]);
    c.bench_function("mesh_transparent_sphere", |b| {
        b.iter(|| {
            mesher.clear();
            mesher.mesh(black_box(&voxels), black_box(&transparent_blocks));
        })
    });
}

fn fast_mesh_random_noise(c: &mut Criterion) {
    let voxels = random_noise::<CS>(0.8);
    let mut mesher = bgm::Mesher::<CS>::new();
    let opaque_mask = bgm::compute_opaque_mask::<CS>(&voxels, &BTreeSet::new());
    let trans_mask = vec![0; bgm::Mesher::<CS>::CS_P2].into_boxed_slice();
    c.bench_function("fast_mesh_random_noise", |b| {
        b.iter(|| {
            mesher.clear();
            mesher.fast_mesh(
                black_box(&voxels),
                black_box(&opaque_mask),
                black_box(&trans_mask),
            );
        })
    });
}

fn mesh_random_noise(c: &mut Criterion) {
    let voxels = random_noise::<CS>(0.8);
    let mut mesher = bgm::Mesher::<CS>::new();
    let transparents = BTreeSet::new();
    c.bench_function("mesh_random_noise", |b| {
        b.iter(|| {
            mesher.clear();
            mesher.mesh(black_box(&voxels), black_box(&transparents));
        })
    });
}

criterion_group!(
    mesh,
    fast_mesh_opaque_sphere,
    mesh_opaque_sphere,
    fast_mesh_transparent_sphere,
    mesh_transparent_sphere,
    fast_mesh_random_noise,
    mesh_random_noise
);
criterion_main!(mesh);
