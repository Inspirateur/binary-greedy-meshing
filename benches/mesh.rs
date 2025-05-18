use std::collections::BTreeSet;

use binary_greedy_meshing as bgm;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
const CHUNK_SIZE: usize = 62;
const CS_H: usize = CHUNK_SIZE/2;
const SIZE: usize = 16;
const SIZE2: usize = SIZE.pow(2);

use bgm::MeshDataGeneric as MD;

fn voxel_buffer() -> Box<[u16; MD::<CHUNK_SIZE>::CS_P3]> {
    let mut voxels = Box::new([0; MD::<CHUNK_SIZE>::CS_P3]);
    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                voxels[bgm::pad_linearize_sized::<CHUNK_SIZE>(x, y, z)] = sphere(x, y, z);
            }
        }
    }
    voxels
}

fn sphere(x: usize, y: usize, z: usize) -> u16 {
    if (x - CS_H).pow(2) + (y - CS_H).pow(2) + (z - CS_H).pow(2) < SIZE2 {
        1
    } else {
        0
    }
}

fn bench_opaque(c: &mut Criterion) {
    let voxels = voxel_buffer();
    let mut mesh_data = bgm::MeshDataSized::<CHUNK_SIZE>::new();
    c.bench_function("bench_opaque", |b| b.iter(|| {
        mesh_data.clear();
        bgm::mesh_sized::<CHUNK_SIZE>(
            black_box(voxels.as_slice()), black_box(&mut mesh_data), black_box(BTreeSet::default())
        );
    }));
}

fn bench_transparent(c: &mut Criterion) {
    let voxels = voxel_buffer();
    let mut transparents = BTreeSet::default();
    transparents.insert(2);
    transparents.insert(3);
    let mut mesh_data = bgm::MeshDataSized::<CHUNK_SIZE>::new();
    c.bench_function("bench_transparent", |b| b.iter(|| {
        mesh_data.clear();
        bgm::mesh_sized::<CHUNK_SIZE>(
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