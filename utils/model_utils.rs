//! Utility functions to get various voxel data (for benchmarking and examples).
//! Not part of the library: included by benches/examples with `#[path]`.
#![allow(dead_code)]
use binary_greedy_meshing as bgm;

#[repr(u16)]
pub enum VoxelType {
    Air = 0,
    Rock = 1,
    Dirt = 2,
    Grass = 3,
    Wood = 4,
    Leaves = 5,
    Glass = 6,
}

pub fn opaque_sphere<const CS: usize>(radius: usize) -> Box<[u16]> {
    let mut voxels = vec![0; bgm::Mesher::<CS>::CS_P3].into_boxed_slice();
    let center = (CS / 2) as isize;
    let radius_squared = (radius * radius) as isize;
    for x in 0..CS {
        for y in 0..CS {
            for z in 0..CS {
                let dist = (x as isize - center).pow(2)
                    + (y as isize - center).pow(2)
                    + (z as isize - center).pow(2);
                voxels[bgm::pad_linearize::<CS>(x, y, z)] = if dist < radius_squared {
                    VoxelType::Rock as u16
                } else {
                    VoxelType::Air as u16
                };
            }
        }
    }
    voxels
}

pub fn half_transparent_sphere<const CS: usize>(radius: usize) -> Box<[u16]> {
    let mut voxels = vec![0; bgm::Mesher::<CS>::CS_P3].into_boxed_slice();
    let center = (CS / 2) as isize;
    let radius_squared = (radius * radius) as isize;
    for x in 0..CS {
        for y in 0..CS {
            for z in 0..CS {
                let dist = (x as isize - center).pow(2)
                    + (y as isize - center).pow(2)
                    + (z as isize - center).pow(2);
                voxels[bgm::pad_linearize::<CS>(x, y, z)] = if dist < radius_squared {
                    if y < center as usize {
                        VoxelType::Rock as u16
                    } else {
                        VoxelType::Glass as u16
                    }
                } else {
                    VoxelType::Air as u16
                };
            }
        }
    }
    voxels
}

pub fn random_noise<const CS: usize>(density: f64) -> Box<[u16]> {
    (0..bgm::Mesher::<CS>::CS_P3)
        .map(|_| {
            if rand::random::<f64>() < density {
                VoxelType::Rock as u16
            } else {
                VoxelType::Air as u16
            }
        })
        .collect()
}

pub fn natural_terrain<const CS: usize>() -> Box<[u16]> {
    let mut voxels = vec![0; bgm::Mesher::<CS>::CS_P3].into_boxed_slice();
    let cs_fract = |fract| (fract * CS as f32) as usize;
    let trees = [
        (cs_fract(0.2), cs_fract(0.1)),
        (cs_fract(0.5), cs_fract(0.7)),
        (cs_fract(0.6), cs_fract(0.2)),
        (cs_fract(0.7), cs_fract(0.9)),
    ];
    // Create a heightmap from large, medium, and small terrain features.
    let base_height = CS / 3;
    let mut heightmap = vec![base_height; CS * CS];
    let frequency = std::f32::consts::TAU / CS as f32;
    for x in 0..CS {
        for z in 0..CS {
            let xf = x as f32 * frequency;
            let zf = z as f32 * frequency;
            let height_offset = (CS as f32 * 0.06) * (xf + zf * 0.5).sin()
                + (CS as f32 * 0.03) * (xf * 2. + zf * 1. + 1.3).sin()
                + (CS as f32 * 0.01) * (xf * 4. - zf * 2. + 2.6).sin();
            heightmap[x * CS + z] = (base_height as f32 + height_offset)
                .round()
                .clamp(1., (CS - 1) as f32) as usize;
        }
    }
    // Fill the voxel grid based on the heightmap
    for x in 0..CS {
        for z in 0..CS {
            let height = heightmap[x * CS + z];
            for y in 0..CS {
                voxels[bgm::pad_linearize::<CS>(x, y, z)] = if y < height {
                    VoxelType::Dirt as u16
                } else if y == height {
                    VoxelType::Grass as u16
                } else {
                    VoxelType::Air as u16
                };
            }
        }
    }
    // Add simple trees at the chosen coordinates
    for (tree_x, tree_z) in trees {
        let tree_y = heightmap[tree_x * CS + tree_z];
        let trunk_end = (tree_y + CS / 4).min(CS);
        for y in tree_y..trunk_end {
            voxels[bgm::pad_linearize::<CS>(tree_x, y, tree_z)] = VoxelType::Wood as u16;
        }
        for x in tree_x.saturating_sub(4)..=(tree_x + 4).min(CS - 1) {
            for z in tree_z.saturating_sub(4)..=(tree_z + 4).min(CS - 1) {
                for y in trunk_end..(trunk_end + 2).min(CS) {
                    voxels[bgm::pad_linearize::<CS>(x, y, z)] = VoxelType::Leaves as u16;
                }
            }
        }
    }

    voxels
}
