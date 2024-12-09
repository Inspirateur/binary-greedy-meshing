use std::collections::BTreeSet;

use binary_greedy_meshing as bgm;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
const SIZE: usize = 16;
const SIZE2: usize = SIZE.pow(2);

fn voxel_buffer() -> Box<[u16; bgm::CS_P3]> {
    let mut voxels = Box::new([0; bgm::CS_P3]);
    for x in 0..bgm::CS {
        for y in 0..bgm::CS {
            for z in 0..bgm::CS {
                voxels[bgm::pad_linearize(x, y, z)] = sphere(x, y, z);
            }
        }
    }
    voxels
}

fn sphere(x: usize, y: usize, z: usize) -> u16 {
    if (x as i32-31).pow(2) + (y as i32-31).pow(2) + (z as i32-31).pow(2) < SIZE2 as i32 {
        1
    } else {
        0
    }
}

fn bench_opaque(c: &mut Criterion) {
    let voxels = voxel_buffer();
    let mut mesh_data = bgm::MeshData::new();
    c.bench_function("bench_opaque", |b| b.iter(|| {
        mesh_data.clear();
        bgm::mesh(
            black_box(voxels.as_slice()), black_box(&mut mesh_data), black_box(BTreeSet::default())
        );
    }));
}

fn bench_transparent(c: &mut Criterion) {
    let voxels = voxel_buffer();
    let mut transparents = BTreeSet::default();
    transparents.insert(2);
    transparents.insert(3);
    let mut mesh_data = bgm::MeshData::new();
    c.bench_function("bench_transparent", |b| b.iter(|| {
        mesh_data.clear();
        bgm::mesh(
            black_box(voxels.as_slice()), black_box(&mut mesh_data), black_box(BTreeSet::default())
        );
    }));
}

criterion_group!(
    mesh, 
    bench_opaque, 
    bench_transparent
);
criterion_main!(mesh);