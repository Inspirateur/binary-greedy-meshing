use binary_greedy_meshing::RichMesher;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
const SIZE: usize = 16;
const SIZE2: usize = SIZE.pow(2);
const CS: usize = 62;

fn voxel_buffer<F>(f: F) -> Box<[u32; RichMesher::CS_P3]>
where
    F: Fn(usize, usize, usize) -> u32,
{
    let mut voxels = Box::new([0; RichMesher::CS_P3]);
    for x in 0..CS {
        for y in 0..CS {
            for z in 0..CS {
                voxels[RichMesher::pad_linearize(x, y, z)] = f(x, y, z);
            }
        }
    }
    voxels
}

fn opaque_sphere(x: usize, y: usize, z: usize) -> u32 {
    if (x as i32 - 31).pow(2) + (y as i32 - 31).pow(2) + (z as i32 - 31).pow(2) < SIZE2 as i32 {
        1
    } else {
        0
    }
}

fn transparent_sphere(x: usize, y: usize, z: usize) -> u32 {
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

fn fast_mesh_opaque(c: &mut Criterion) {
    let voxels = voxel_buffer(opaque_sphere);
    let mut mesher = RichMesher::new();
    let opaque_mask = RichMesher::compute_opaque_mask(voxels.as_slice(), |_| false);
    let trans_mask = vec![0; RichMesher::CS_P2].into_boxed_slice();
    c.bench_function("fast_mesh_opaque", |b| {
        b.iter(|| {
            mesher.clear();
            mesher.fast_mesh(
                black_box(voxels.as_slice()),
                black_box(&opaque_mask),
                black_box(&trans_mask),
            );
        })
    });
}

fn mesh_opaque(c: &mut Criterion) {
    let voxels = voxel_buffer(opaque_sphere);
    let mut mesher = RichMesher::new();
    c.bench_function("mesh_opaque", |b| {
        b.iter(|| {
            mesher.clear();
            mesher.mesh(black_box(voxels.as_slice()), black_box(|_| false));
        })
    });
}

fn fast_mesh_transparent(c: &mut Criterion) {
    let voxels = voxel_buffer(transparent_sphere);
    let mut mesher = RichMesher::new();
    let opaque_mask = RichMesher::compute_opaque_mask(voxels.as_slice(), |_| false);
    let trans_mask = RichMesher::compute_transparent_mask(voxels.as_slice(), |v| v == 2);
    c.bench_function("fast_mesh_transparent", |b| {
        b.iter(|| {
            mesher.clear();
            mesher.fast_mesh(
                black_box(voxels.as_slice()),
                black_box(&opaque_mask),
                black_box(&trans_mask),
            );
        })
    });
}

fn mesh_transparent(c: &mut Criterion) {
    let voxels = voxel_buffer(transparent_sphere);
    let mut mesher = RichMesher::new();
    c.bench_function("mesh_transparent", |b| {
        b.iter(|| {
            mesher.clear();
            mesher.mesh(black_box(voxels.as_slice()), black_box(|v| v == 2));
        })
    });
}

criterion_group!(
    mesh,
    fast_mesh_opaque,
    mesh_opaque,
    fast_mesh_transparent,
    mesh_transparent
);
criterion_main!(mesh);
