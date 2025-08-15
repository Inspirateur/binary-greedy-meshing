#![no_std]

#[macro_use]
extern crate alloc;

mod face;
mod quad;

use alloc::{boxed::Box, collections::btree_set::BTreeSet, vec::Vec};

pub use face::*;
pub use quad::*;

#[derive(Debug)]
pub struct Mesher<const CS: usize> {
    // Output
    pub quads: [Vec<Quad>; 6],
    // Internal buffers
    /// CS_2 * 6
    face_masks: Box<[u64]>,
    /// CS_2
    forward_merged: Box<[u8]>,
    /// CS
    right_merged: Box<[u8]>,
}

impl<const CS: usize> Mesher<CS> {
    pub const CS_2: usize = CS * CS;
    pub const CS_P: usize = CS + 2;
    pub const CS_P2: usize = Self::CS_P * Self::CS_P;
    pub const CS_P3: usize = Self::CS_P * Self::CS_P * Self::CS_P;
    pub const P_MASK: u64 = !(1 << (Self::CS_P - 1) | 1);

    /// Creates a mesher object, allocates necessary buffers
    pub fn new() -> Self {
        Self {
            face_masks: vec![0; Self::CS_2 * 6].into_boxed_slice(),
            forward_merged: vec![0; Self::CS_2].into_boxed_slice(),
            right_merged: vec![0; CS].into_boxed_slice(),
            quads: core::array::from_fn(|_| Vec::new()),
        }
    }

    /// Call this between each meshing call to reset the buffers without reallocating them
    pub fn clear(&mut self) {
        self.face_masks.fill(0);
        self.forward_merged.fill(0);
        self.right_merged.fill(0);
        for i in 0..self.quads.len() {
            self.quads[i].clear();
        }
    }

    fn face_culling(&mut self, voxels: &[u16], transparents: &BTreeSet<u16>) {
        // Hidden face culling
        for a in 1..(Self::CS_P - 1) {
            let a_cs_p = a * Self::CS_P;

            for b in 1..(Self::CS_P - 1) {
                let ab = (a_cs_p + b) * Self::CS_P;
                let ba_index = (b - 1) + (a - 1) * CS;
                let ab_index = (a - 1) + (b - 1) * CS;

                for c in 1..(Self::CS_P - 1) {
                    let abc = ab + c;
                    let v1 = voxels[abc];
                    if v1 == 0 {
                        continue;
                    }
                    self.face_masks[ba_index + 0 * Self::CS_2] |=
                        face_value(v1, voxels[abc + Self::CS_P2], &transparents) << (c - 1);
                    self.face_masks[ba_index + 1 * Self::CS_2] |=
                        face_value(v1, voxels[abc - Self::CS_P2], &transparents) << (c - 1);

                    self.face_masks[ab_index + 2 * Self::CS_2] |=
                        face_value(v1, voxels[abc + Self::CS_P], &transparents) << (c - 1);
                    self.face_masks[ab_index + 3 * Self::CS_2] |=
                        face_value(v1, voxels[abc - Self::CS_P], &transparents) << (c - 1);

                    self.face_masks[ba_index + 4 * Self::CS_2] |=
                        face_value(v1, voxels[abc + 1], &transparents) << c;
                    self.face_masks[ba_index + 5 * Self::CS_2] |=
                        face_value(v1, voxels[abc - 1], &transparents) << c;
                }
            }
        }
    }

    fn fast_face_culling(&mut self, voxels: &[u16], opaque_mask: &[u64], trans_mask: &[u64]) {
        // Hidden face culling
        for a in 1..(Self::CS_P - 1) {
            let a_ = a * Self::CS_P;

            for b in 1..(Self::CS_P - 1) {
                // Column-wise opaque step
                let ab = a_ + b;
                let opaque_col = opaque_mask[ab] & Self::P_MASK;
                let unpadded_opaque_col = opaque_col >> 1;
                let ba_index = (b - 1) + (a - 1) * CS;
                let ab_index = (a - 1) + (b - 1) * CS;
                let up_faces = ba_index + 0 * Self::CS_2;
                let down_faces = ba_index + 1 * Self::CS_2;
                let right_faces = ab_index + 2 * Self::CS_2;
                let left_faces = ab_index + 3 * Self::CS_2;
                let front_faces = ba_index + 4 * Self::CS_2;
                let back_faces = ba_index + 5 * Self::CS_2;
                let not_front_col = !opaque_mask[ab + Self::CS_P] >> 1;
                let not_back_col = !opaque_mask[ab - Self::CS_P] >> 1;
                let not_right_col = !opaque_mask[ab + 1] >> 1;
                let not_left_col = !opaque_mask[ab - 1] >> 1;
                let not_col_up = !(opaque_mask[ab] >> 1);
                let not_col_down = !(opaque_mask[ab] << 1);
                self.face_masks[up_faces] = unpadded_opaque_col & not_front_col;
                self.face_masks[down_faces] = unpadded_opaque_col & not_back_col;

                self.face_masks[right_faces] = unpadded_opaque_col & not_right_col;
                self.face_masks[left_faces] = unpadded_opaque_col & not_left_col;

                self.face_masks[front_faces] = opaque_col & not_col_up;
                self.face_masks[back_faces] = opaque_col & not_col_down;

                // check if there's transparent blocks in this column
                let mut bits_here = trans_mask[ab] & Self::P_MASK;
                if bits_here == 0 {
                    continue;
                }
                // Block-wise transparent step
                // The transparent step is slower than the opaque step
                // because we need to check if neighboring transparent blocks are differents (we don't care about that for opaque blocks)
                let ab_ = ab * Self::CS_P;
                while bits_here != 0 {
                    let c = bits_here.trailing_zeros() as usize;
                    let c_mask = 1 << c;
                    let unpadded_c_mask = c_mask >> 1;
                    bits_here &= !(c_mask);
                    let abc = ab_ + c;
                    let v1 = voxels[abc];
                    self.face_masks[up_faces] |= not_front_col
                        & unpadded_c_mask
                        & ((v1 != voxels[abc + Self::CS_P2]) as u64) << (c - 1);
                    self.face_masks[down_faces] |= not_back_col
                        & unpadded_c_mask
                        & ((v1 != voxels[abc - Self::CS_P2]) as u64) << (c - 1);

                    self.face_masks[right_faces] |= not_right_col
                        & unpadded_c_mask
                        & ((v1 != voxels[abc + Self::CS_P]) as u64) << (c - 1);
                    self.face_masks[left_faces] |= not_left_col
                        & unpadded_c_mask
                        & ((v1 != voxels[abc - Self::CS_P]) as u64) << (c - 1);

                    self.face_masks[front_faces] |=
                        not_col_up & c_mask & ((v1 != voxels[abc + 1]) as u64) << c;
                    self.face_masks[back_faces] |=
                        not_col_down & c_mask & ((v1 != voxels[abc - 1]) as u64) << c;
                }
            }
        }
    }

    fn face_merging(&mut self, voxels: &[u16]) {
        // Greedy meshing faces 0-3
        for face in 0..=3 {
            let axis = face / 2;

            for layer in 0..CS {
                let bits_location = layer * CS + face * Self::CS_2;

                for forward in 0..CS {
                    let mut bits_here = self.face_masks[forward + bits_location];
                    if bits_here == 0 {
                        continue;
                    }

                    let bits_next = if forward + 1 < CS {
                        self.face_masks[(forward + 1) + bits_location]
                    } else {
                        0
                    };

                    let mut right_merged = 1;
                    while bits_here != 0 {
                        let bit_pos = bits_here.trailing_zeros() as usize;

                        let v_type =
                            voxels[get_axis_index::<CS>(axis, forward + 1, bit_pos + 1, layer + 1)];

                        if (bits_next >> bit_pos & 1) != 0
                            && v_type
                                == voxels[get_axis_index::<CS>(
                                    axis,
                                    forward + 2,
                                    bit_pos + 1,
                                    layer + 1,
                                )]
                        {
                            self.forward_merged[bit_pos] += 1;
                            bits_here &= !(1 << bit_pos);
                            continue;
                        }

                        for right in (bit_pos + 1)..CS {
                            if (bits_here >> right & 1) == 0
                                || self.forward_merged[bit_pos] != self.forward_merged[right]
                                || v_type
                                    != voxels[get_axis_index::<CS>(
                                        axis,
                                        forward + 1,
                                        right + 1,
                                        layer + 1,
                                    )]
                            {
                                break;
                            }
                            self.forward_merged[right] = 0;
                            right_merged += 1;
                        }
                        bits_here &= !((1 << (bit_pos + right_merged)) - 1);

                        let mesh_front = forward - self.forward_merged[bit_pos] as usize;
                        let mesh_left = bit_pos;
                        let mesh_up = layer + (!face & 1);

                        let mesh_width = right_merged;
                        let mesh_length = (self.forward_merged[bit_pos] + 1) as usize;

                        self.forward_merged[bit_pos] = 0;
                        right_merged = 1;

                        let v_type = v_type as usize;

                        let quad = match face {
                            0 => Quad::pack(
                                mesh_front,
                                mesh_up,
                                mesh_left,
                                mesh_length,
                                mesh_width,
                                v_type,
                            ),
                            1 => Quad::pack(
                                mesh_front + mesh_length as usize,
                                mesh_up,
                                mesh_left,
                                mesh_length,
                                mesh_width,
                                v_type,
                            ),
                            2 => Quad::pack(
                                mesh_up,
                                mesh_front + mesh_length as usize,
                                mesh_left,
                                mesh_length,
                                mesh_width,
                                v_type,
                            ),
                            3 => Quad::pack(
                                mesh_up,
                                mesh_front,
                                mesh_left,
                                mesh_length,
                                mesh_width,
                                v_type,
                            ),
                            _ => unreachable!(),
                        };
                        self.quads[face].push(quad);
                    }
                }
            }
        }

        // Greedy meshing faces 4-5
        for face in 4..6 {
            let axis = face / 2;

            for forward in 0..CS {
                let bits_location = forward * CS + face * Self::CS_2;
                let bits_forward_location = (forward + 1) * CS + face * Self::CS_2;

                for right in 0..CS {
                    let mut bits_here = self.face_masks[right + bits_location];
                    if bits_here == 0 {
                        continue;
                    }

                    let bits_forward = if forward < CS - 1 {
                        self.face_masks[right + bits_forward_location]
                    } else {
                        0
                    };
                    let bits_right = if right < CS - 1 {
                        self.face_masks[right + 1 + bits_location]
                    } else {
                        0
                    };
                    let right_cs = right * CS;

                    while bits_here != 0 {
                        let bit_pos = bits_here.trailing_zeros() as usize;

                        bits_here &= !(1 << bit_pos);

                        let v_type =
                            voxels[get_axis_index::<CS>(axis, right + 1, forward + 1, bit_pos)];
                        let forward_merge_i = right_cs + (bit_pos - 1);
                        let right_merged_ref = &mut self.right_merged[bit_pos - 1];

                        if *right_merged_ref == 0
                            && (bits_forward >> bit_pos & 1) != 0
                            && v_type
                                == voxels
                                    [get_axis_index::<CS>(axis, right + 1, forward + 2, bit_pos)]
                        {
                            self.forward_merged[forward_merge_i] += 1;
                            continue;
                        }

                        if (bits_right >> bit_pos & 1) != 0
                            && self.forward_merged[forward_merge_i]
                                == self.forward_merged[(right_cs + CS) + (bit_pos - 1)]
                            && v_type
                                == voxels
                                    [get_axis_index::<CS>(axis, right + 2, forward + 1, bit_pos)]
                        {
                            self.forward_merged[forward_merge_i] = 0;
                            *right_merged_ref += 1;
                            continue;
                        }

                        let mesh_left = right - *right_merged_ref as usize;
                        let mesh_front = forward - self.forward_merged[forward_merge_i] as usize;
                        let mesh_up = bit_pos - 1 + (!face & 1);

                        let mesh_width = 1 + *right_merged_ref;
                        let mesh_length = 1 + self.forward_merged[forward_merge_i];

                        self.forward_merged[forward_merge_i] = 0;
                        *right_merged_ref = 0;

                        let quad = Quad::pack(
                            mesh_left + (if face == 4 { mesh_width } else { 0 }) as usize,
                            mesh_front,
                            mesh_up,
                            mesh_width as usize,
                            mesh_length as usize,
                            v_type as usize,
                        );
                        self.quads[face].push(quad);
                    }
                }
            }
        }
    }

    /// Meshes a voxel buffer representing a chunk, using an opaque and transparent mask with 1 u64 per column with 1 bit per voxel in the column,
    /// signaling if the voxel is opaque or transparent.
    /// This is ~4x faster than the regular mesh method but requires maintaining 2 masks for each chunk.
    /// See https://github.com/Inspirateur/binary-greedy-meshing?tab=readme-ov-file#what-to-do-with-mesh_dataquads for using the output
    pub fn fast_mesh(&mut self, voxels: &[u16], opaque_mask: &[u64], trans_mask: &[u64]) {
        self.fast_face_culling(voxels, opaque_mask, trans_mask);
        self.face_merging(voxels);
    }

    /// Meshes a voxel buffer representing a chunk, using a BTreeSet signaling which voxel values are transparent.
    /// This is ~4x slower than the fast_mesh method but does not require maintaining 2 masks for each chunk.
    /// See https://github.com/Inspirateur/binary-greedy-meshing?tab=readme-ov-file#what-to-do-with-mesh_dataquads for using the output
    pub fn mesh(&mut self, voxels: &[u16], transparents: &BTreeSet<u16>) {
        self.face_culling(voxels, transparents);
        self.face_merging(voxels);
    }
}

#[inline]
/// v1 is not AIR
fn face_value(v1: u16, v2: u16, transparents: &BTreeSet<u16>) -> u64 {
    (v2 == 0 || (v1 != v2 && transparents.contains(&v2))) as u64
}

#[inline]
fn get_axis_index<const CS: usize>(axis: usize, a: usize, b: usize, c: usize) -> usize {
    // TODO: figure out how to shuffle this around to make it work with YZX
    match axis {
        0 => b + (a * Mesher::<CS>::CS_P) + (c * Mesher::<CS>::CS_P2),
        1 => b + (c * Mesher::<CS>::CS_P) + (a * Mesher::<CS>::CS_P2),
        _ => c + (a * Mesher::<CS>::CS_P) + (b * Mesher::<CS>::CS_P2),
    }
}

/// Compute Mesh indices for a given amount of quads
pub fn indices(num_quads: usize) -> Vec<u32> {
    // Each quads is made of 2 triangles which require 6 indices
    // The indices are the same regardless of the face
    let mut res = Vec::with_capacity(num_quads * 6);
    for i in 0..num_quads as u32 {
        res.push((i << 2) | 2);
        res.push((i << 2) | 0);
        res.push((i << 2) | 1);
        res.push((i << 2) | 1);
        res.push((i << 2) | 3);
        res.push((i << 2) | 2);
    }
    res
}

pub fn pad_linearize<const CS: usize>(x: usize, y: usize, z: usize) -> usize {
    z + 1 + (x + 1) * Mesher::<CS>::CS_P + (y + 1) * Mesher::<CS>::CS_P2
}

/// Compute an opacity mask from a voxel buffer and a BTreeSet specifying which voxel values are transparent
pub fn compute_opaque_mask<const CS: usize>(
    voxels: &[u16],
    transparents: &BTreeSet<u16>,
) -> Box<[u64]> {
    let mut opaque_mask = vec![0; Mesher::<CS>::CS_P2].into_boxed_slice();
    // Fill the opacity mask
    for (i, voxel) in voxels.iter().enumerate() {
        // If the voxel is transparent we skip it
        if *voxel == 0 || transparents.contains(voxel) {
            continue;
        }
        let (r, q) = (i / Mesher::<CS>::CS_P, i % Mesher::<CS>::CS_P);
        opaque_mask[r] |= 1 << q;
    }
    opaque_mask
}

/// Compute a transparent mask from a voxel buffer and a BTreeSet specifying which voxel values are transparent
pub fn compute_transparent_mask<const CS: usize>(
    voxels: &[u16],
    transparents: &BTreeSet<u16>,
) -> Box<[u64]> {
    let mut trans_mask = vec![0; Mesher::<CS>::CS_P2].into_boxed_slice();
    // Fill the opacity mask
    for (i, voxel) in voxels.iter().enumerate() {
        // If the voxel is opaque we skip it
        if *voxel == 0 || !transparents.contains(voxel) {
            continue;
        }
        let (r, q) = (i / Mesher::<CS>::CS_P, i % Mesher::<CS>::CS_P);
        trans_mask[r] |= 1 << q;
    }
    trans_mask
}

#[cfg(test)]
mod tests {
    use crate::{self as bgm};
    use alloc::{boxed::Box, collections::btree_set::BTreeSet};

    pub const CS: usize = 62;

    /// Show quad output on a simple 2 voxels case
    #[test]
    fn test_output() {
        extern crate std;
        let mut voxels = [0; bgm::Mesher::<CS>::CS_P3];
        voxels[bgm::pad_linearize::<CS>(0, 0, 0)] = 1;
        voxels[bgm::pad_linearize::<CS>(0, 1, 0)] = 1;

        let mut mesher = bgm::Mesher::<CS>::new();
        let opaque_mask = bgm::compute_opaque_mask::<CS>(&voxels, &BTreeSet::new());
        let trans_mask = vec![0; bgm::Mesher::<CS>::CS_P2].into_boxed_slice();
        mesher.fast_mesh(&voxels, &opaque_mask, &trans_mask);
        // self.quads is the output
        for (i, quads) in mesher.quads.iter().enumerate() {
            std::println!("--- Face {i} ---");
            for &quad in quads {
                std::println!("{:?}", quad);
            }
        }
    }

    /// Ensures that mesh and fast_mesh return the same results
    #[test]
    fn same_results() {
        let voxels = test_buffer();
        let transparent_blocks = BTreeSet::from([2]);
        let opaque_mask = bgm::compute_opaque_mask::<CS>(voxels.as_slice(), &BTreeSet::new());
        let trans_mask =
            bgm::compute_transparent_mask::<CS>(voxels.as_slice(), &transparent_blocks);
        let mut mesher1 = bgm::Mesher::<CS>::new();
        mesher1.mesh(voxels.as_slice(), &transparent_blocks);
        let mut mesher2 = bgm::Mesher::<CS>::new();
        mesher2.fast_mesh(voxels.as_slice(), &opaque_mask, &trans_mask);
        assert_eq!(mesher1.quads, mesher2.quads);
    }

    fn test_buffer() -> Box<[u16; bgm::Mesher::<CS>::CS_P3]> {
        let mut voxels = Box::new([0; bgm::Mesher::<CS>::CS_P3]);
        for x in 0..CS {
            for y in 0..CS {
                for z in 0..CS {
                    voxels[bgm::pad_linearize::<CS>(x, y, z)] = transparent_sphere(x, y, z);
                }
            }
        }
        voxels
    }

    fn transparent_sphere(x: usize, y: usize, z: usize) -> u16 {
        if x == 8 {
            2
        } else if (x as i32 - 31).pow(2) + (y as i32 - 31).pow(2) + (z as i32 - 31).pow(2)
            < 16 as i32
        {
            1
        } else {
            0
        }
    }
}
