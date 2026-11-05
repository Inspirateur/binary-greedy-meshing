use std::{collections::BTreeSet, hint::black_box};

use binary_greedy_meshing as bgm;
use criterion::{Criterion, criterion_group, criterion_main};
const SIZE: usize = 16;
const SIZE2: usize = SIZE.pow(2);
const CS: usize = 62;

fn voxel_buffer<F>(f: F) -> Box<[u16; bgm::Mesher::<CS>::CS_P3]>
where
    F: Fn(usize, usize, usize) -> u16,
{
    let mut voxels = Box::new([0; bgm::Mesher::<CS>::CS_P3]);
    for x in 0..CS {
        for y in 0..CS {
            for z in 0..CS {
                voxels[bgm::pad_linearize::<CS>(x, y, z)] = f(x, y, z);
            }
        }
    }
    voxels
}

fn opaque_sphere(x: usize, y: usize, z: usize) -> u16 {
    if (x as i32 - 31).pow(2) + (y as i32 - 31).pow(2) + (z as i32 - 31).pow(2) < SIZE2 as i32 {
        1
    } else {
        0
    }
}

fn transparent_sphere(x: usize, y: usize, z: usize) -> u16 {
    if x == SIZE / 2 {
        2
    } else if (x as i32 - 31).pow(2) + (y as i32 - 31).pow(2) + (z as i32 - 31).pow(2)
        < SIZE2 as i32
    {
        1
    } else {
        0
    }
}

fn mesh_opaque_lod(c: &mut Criterion) {
    let mut voxels = voxel_buffer(opaque_sphere);
    let mut mesher = bgm::Mesher::<CS>::new();
    let transparents = BTreeSet::new();
    c.bench_function("mesh_opaque_lod", |b| {
        b.iter(|| {
            mesher.clear();
            mesher.mesh_lod(
                black_box(voxels.as_mut_slice()),
                black_box(&transparents),
                black_box(4),
                black_box(None),
            );
        })
    });
}

fn mesh_transparent_lod(c: &mut Criterion) {
    let mut voxels = voxel_buffer(transparent_sphere);
    let mut mesher = bgm::Mesher::<CS>::new();
    let transparent_blocks = BTreeSet::from([2]);
    c.bench_function("mesh_transparent_lod", |b| {
        b.iter(|| {
            mesher.clear();
            mesher.mesh_lod(
                black_box(voxels.as_mut_slice()),
                black_box(&transparent_blocks),
                black_box(4),
                black_box(None),
            );
        })
    });
}

criterion_group!(lod, mesh_opaque_lod, mesh_transparent_lod);
criterion_main!(lod);
