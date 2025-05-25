use std::collections::BTreeSet;

use binary_greedy_meshing as bgm;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
const SIZE: usize = 16;
const SIZE2: usize = SIZE.pow(2);

fn voxel_buffer<F>(f: F) -> Box<[u16; bgm::CS_P3]> 
    where F: Fn(usize, usize, usize) -> u16,
{
    let mut voxels = Box::new([0; bgm::CS_P3]);
    for x in 0..bgm::CS {
        for y in 0..bgm::CS {
            for z in 0..bgm::CS {
                voxels[bgm::pad_linearize(x, y, z)] = f(x, y, z);
            }
        }
    }
    voxels
}

fn opaque_sphere(x: usize, y: usize, z: usize) -> u16 {
    if (x as i32-31).pow(2) + (y as i32-31).pow(2) + (z as i32-31).pow(2) < SIZE2 as i32 {
        1
    } else {
        0
    }
}

fn transparent_sphere(x: usize, y: usize, z: usize) -> u16 {
    if x == SIZE/2 {
        2
    } else if (x as i32-31).pow(2) + (y as i32-31).pow(2) + (z as i32-31).pow(2) < SIZE2 as i32 {
        1
    } else {
        0
    }
}

fn bench_fast_opaque(c: &mut Criterion) {
    let voxels = voxel_buffer(opaque_sphere);
    let mut mesher = bgm::Mesher::new();
    let opaque_mask = bgm::compute_opaque_mask(voxels.as_slice(), &BTreeSet::new());
    let trans_mask = vec![0; bgm::CS_P2].into_boxed_slice();
    c.bench_function("bench_fast_opaque", |b| b.iter(|| {
        mesher.clear();
        mesher.fast_mesh(black_box(voxels.as_slice()), black_box(&opaque_mask), black_box(&trans_mask));
    }));
}

fn bench_opaque(c: &mut Criterion) {
    let voxels = voxel_buffer(opaque_sphere);
    let mut mesher = bgm::Mesher::new();
    let transparents = BTreeSet::new();
    c.bench_function("bench_opaque", |b| b.iter(|| {
        mesher.clear();
        mesher.mesh(black_box(voxels.as_slice()), black_box(&transparents));
    }));
}

fn bench_fast_transparent(c: &mut Criterion) {
    let voxels = voxel_buffer(transparent_sphere);
    let mut mesher = bgm::Mesher::new();
    let transparent_blocks = BTreeSet::from([2]);
    let opaque_mask = bgm::compute_opaque_mask(voxels.as_slice(), &BTreeSet::new());
    let trans_mask = bgm::compute_trans_mask(voxels.as_slice(), &transparent_blocks);
    c.bench_function("bench_fast_transparent", |b| b.iter(|| {
        mesher.clear();
        mesher.fast_mesh(black_box(voxels.as_slice()), black_box(&opaque_mask), black_box(&trans_mask));
    }));
}

fn bench_transparent(c: &mut Criterion) {
    let voxels = voxel_buffer(transparent_sphere);
    let mut mesher = bgm::Mesher::new();
    let transparent_blocks = BTreeSet::from([2]);
    c.bench_function("bench_transparent", |b| b.iter(|| {
        mesher.clear();
        mesher.mesh(black_box(voxels.as_slice()), black_box(&transparent_blocks));
    }));
}

criterion_group!(
    mesh, 
    bench_fast_opaque, 
    bench_opaque,
    bench_fast_transparent,
    bench_transparent
);
criterion_main!(mesh);