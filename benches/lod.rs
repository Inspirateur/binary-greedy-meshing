use binary_greedy_meshing as bgm;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::{collections::BTreeSet, hint::black_box};
const RADIUS: usize = 32;
const RADIUS_SQUARED: usize = RADIUS.pow(2);
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
    let center = CS / 2;
    if (x - center).pow(2) + (y - center).pow(2) + (z - center).pow(2) < RADIUS_SQUARED {
        1
    } else {
        0
    }
}

fn mesh_opaque_lod<const LOD: usize>(c: &mut Criterion) {
    let voxels = voxel_buffer(opaque_sphere);
    let mut mesher = bgm::Mesher::<CS>::new();
    let transparents = BTreeSet::new();
    c.bench_function(&format!("mesh_opaque_lod {LOD}"), |b| {
        // mesh_lod mutates the buffer, so each iteration needs a fresh copy
        b.iter_batched_ref(
            || voxels.clone(),
            |voxels| {
                mesher.clear();
                mesher.mesh_lod(
                    black_box(voxels.as_mut_slice()),
                    black_box(&transparents),
                    black_box(LOD),
                );
            },
            BatchSize::LargeInput,
        )
    });
}

criterion_group!(
    lod,
    mesh_opaque_lod::<2>,
    mesh_opaque_lod::<4>,
    mesh_opaque_lod::<8>
);
criterion_main!(lod);
