#![no_std]
#![cfg_attr(feature = "simd", feature(portable_simd))]

#[macro_use]
extern crate alloc;

mod face;
mod material;
mod quad;

use alloc::{boxed::Box, vec::Vec};

use core::simd::Simd;
#[cfg(feature = "simd")]
use core::simd::{u64x4, usizex4};

pub use face::*;
pub use material::*;
pub use quad::*;

// The `CS` (chunk size) constants for each mesher type are bounded by the bit-width
// of the coordinate fields in the corresponding quad structs.
//
// For default types we use Max CS:
// Max CS = 2^(Quad coordinate bitness (for example 5)) - 1
//
// But you can use only usable section of chunk:
// Usable CS = Max CS - 1
//
// So for example, final chunk size where you can place any blocks is 2^5 - 1 - 1 = 30
pub type RichMesher = Mesher<u32, RichQuad, 63>;
pub type MiniMesher = Mesher<u8, MiniQuad, 63>;
pub type MicroMesher = Mesher<u8, MicroQuad, 31>;

#[derive(Debug)]
pub struct Mesher<M: Material, Q: Quad<M>, const CS: usize> {
    // Output
    pub quads: [Vec<Q>; 6],
    // Internal buffers
    /// CS_2 * 6
    face_masks: Box<[u64]>,
    /// CS_2
    forward_merged: Box<[u8]>,
    /// CS
    right_merged: Box<[u8]>,
    p: core::marker::PhantomData<M>,
}

impl<M: Material, Q: Quad<M>, const CS: usize> Default for Mesher<M, Q, CS> {
    fn default() -> Self {
        Self {
            face_masks: vec![0; Self::CS_2 * 6].into_boxed_slice(),
            forward_merged: vec![0; Self::CS_2].into_boxed_slice(),
            right_merged: vec![0; CS].into_boxed_slice(),
            quads: core::array::from_fn(|_| Vec::new()),
            p: Default::default(),
        }
    }
}

impl<M: Material, Q: Quad<M>, const CS: usize> Mesher<M, Q, CS> {
    pub const CS_2: usize = CS * CS;
    pub const CS_P: usize = CS + 2;
    pub const CS_P2: usize = Self::CS_P * Self::CS_P;
    pub const CS_P3: usize = Self::CS_P * Self::CS_P * Self::CS_P;
    pub const P_MASK: u64 = !(1 << (Self::CS_P - 1) | 1);

    /// Creates a mesher object, allocates necessary buffers
    pub fn new() -> Self {
        Self::default()
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

    pub fn assemble_padded(center: &[M], neighbors: [Option<&[M]>; 6]) -> Box<[M]>
    where
        M: Default,
    {
        let mut padded = vec![M::default(); Self::CS_P3].into_boxed_slice();

        // Center
        for y in 0..CS {
            for x in 0..CS {
                for z in 0..CS {
                    padded[Self::pad_linearize(x, y, z)] = center[z + x * CS + y * CS * CS];
                }
            }
        }

        // neg_x (left)
        if let Some(neg_x_data) = neighbors[0] {
            for y in 0..CS {
                for z in 0..CS {
                    padded[Self::pad_linearize(0, y, z)] =
                        neg_x_data[z + (CS - 1) * CS + y * CS * CS];
                }
            }
        }
        // pos_x (right)
        if let Some(pos_x_data) = neighbors[1] {
            for y in 0..CS {
                for z in 0..CS {
                    padded[Self::pad_linearize(CS + 1, y, z)] = pos_x_data[z + y * CS * CS];
                }
            }
        }
        // neg_y (bottom)
        if let Some(neg_y_data) = neighbors[2] {
            for x in 0..CS {
                for z in 0..CS {
                    padded[Self::pad_linearize(x, 0, z)] =
                        neg_y_data[z + x * CS + (CS - 1) * CS * CS];
                }
            }
        }
        // pos_y (top)
        if let Some(pos_y_data) = neighbors[3] {
            for x in 0..CS {
                for z in 0..CS {
                    padded[Self::pad_linearize(x, CS + 1, z)] = pos_y_data[z + x * CS];
                }
            }
        }
        // neg_z (back)
        if let Some(neg_z_data) = neighbors[4] {
            for y in 0..CS {
                for x in 0..CS {
                    padded[Self::pad_linearize(x, y, 0)] =
                        neg_z_data[(CS - 1) + x * CS + y * CS * CS];
                }
            }
        }
        // pos_z (front)
        if let Some(pos_z_data) = neighbors[5] {
            for y in 0..CS {
                for x in 0..CS {
                    padded[Self::pad_linearize(x, y, CS + 1)] = pos_z_data[x * CS + y * CS * CS];
                }
            }
        }

        padded
    }

    fn face_culling(&mut self, voxels: &[M], is_transparent: impl Fn(M) -> bool) {
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
                    if v1.is_air() {
                        continue;
                    }
                    self.face_masks[ba_index] |=
                        Self::face_value(v1, voxels[abc + Self::CS_P2], &is_transparent) << (c - 1);
                    self.face_masks[ba_index + Self::CS_2] |=
                        Self::face_value(v1, voxels[abc - Self::CS_P2], &is_transparent) << (c - 1);

                    self.face_masks[ab_index + 2 * Self::CS_2] |=
                        Self::face_value(v1, voxels[abc + Self::CS_P], &is_transparent) << (c - 1);
                    self.face_masks[ab_index + 3 * Self::CS_2] |=
                        Self::face_value(v1, voxels[abc - Self::CS_P], &is_transparent) << (c - 1);

                    self.face_masks[ba_index + 4 * Self::CS_2] |=
                        Self::face_value(v1, voxels[abc + 1], &is_transparent) << c;
                    self.face_masks[ba_index + 5 * Self::CS_2] |=
                        Self::face_value(v1, voxels[abc - 1], &is_transparent) << c;
                }
            }
        }
    }

    fn fast_face_culling(&mut self, voxels: &[M], opaque_mask: &[u64], trans_mask: &[u64]) {
        #[cfg(not(feature = "simd"))]
        {
            for a in 1..(Self::CS_P - 1) {
                let a_ = a * Self::CS_P;
                for b in 1..(Self::CS_P - 1) {
                    self.fast_face_culling_scalar((a, a_), b, voxels, opaque_mask, trans_mask);
                }
            }
        }

        #[cfg(feature = "simd")]
        {
            let p_mask_v = u64x4::splat(Self::P_MASK);
            for a in 1..(Self::CS_P - 1) {
                let a_ = a * Self::CS_P;
                let mut b = 1;
                let b_end = Self::CS_P - 1; // exclusive, loop runs for b in 1..CS_P-1

                while b < b_end {
                    // Process 4 columns at once if possible
                    if b + 4 <= b_end {
                        self.fast_face_culling_simd(
                            (a, a_),
                            b,
                            voxels,
                            opaque_mask,
                            trans_mask,
                            p_mask_v,
                        );
                        b += 4;
                    } else {
                        // Fallback to scalar for a single column (identical to original loop body)
                        self.fast_face_culling_scalar((a, a_), b, voxels, opaque_mask, trans_mask);
                        b += 1;
                    }
                }
            }
        }
    }

    fn fast_face_culling_scalar(
        &mut self,
        (a, a_): (usize, usize),
        b: usize,
        voxels: &[M],
        opaque_mask: &[u64],
        trans_mask: &[u64],
    ) {
        let ab = a_ + b;
        let opaque_col = opaque_mask[ab] & Self::P_MASK;
        let unpadded_opaque_col = opaque_col >> 1;
        let ba_index = (b - 1) + (a - 1) * CS;
        let ab_index = (a - 1) + (b - 1) * CS;
        let up_faces = ba_index;
        let down_faces = ba_index + Self::CS_2;
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
            return;
        }

        // Block-wise transparent step
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

    #[cfg(feature = "simd")]
    fn fast_face_culling_simd(
        &mut self,
        (a, a_): (usize, usize),
        b: usize,
        voxels: &[M],
        opaque_mask: &[u64],
        trans_mask: &[u64],
        p_mask_v: Simd<u64, 4>,
    ) {
        let raw_opaque = u64x4::from_slice(&opaque_mask[a_ + b..][..4]);
        let opaque = raw_opaque & p_mask_v;
        let unpadded = opaque >> 1;

        let not_front = !u64x4::from_slice(&opaque_mask[a_ + b + Self::CS_P..][..4]) >> 1;
        let not_back = !u64x4::from_slice(&opaque_mask[a_ + b - Self::CS_P..][..4]) >> 1;
        let not_right = !u64x4::from_slice(&opaque_mask[a_ + b + 1..][..4]) >> 1;
        let not_left = !u64x4::from_slice(&opaque_mask[a_ + b - 1..][..4]) >> 1;
        let not_up = !(opaque >> 1);
        let not_down = !(opaque << 1);

        let up_vals = unpadded & not_front;
        let down_vals = unpadded & not_back;
        let right_vals = unpadded & not_right;
        let left_vals = unpadded & not_left;
        let front_vals = opaque & not_up;
        let back_vals = opaque & not_down;

        let up_start = (a - 1) * CS + (b - 1);
        self.face_masks[up_start..up_start + 4].copy_from_slice(&up_vals.to_array());
        self.face_masks[up_start + Self::CS_2..up_start + Self::CS_2 + 4]
            .copy_from_slice(&down_vals.to_array());
        self.face_masks[up_start + 4 * Self::CS_2..up_start + 4 * Self::CS_2 + 4]
            .copy_from_slice(&front_vals.to_array());
        self.face_masks[up_start + 5 * Self::CS_2..up_start + 5 * Self::CS_2 + 4]
            .copy_from_slice(&back_vals.to_array());

        let ab_indices: [usize; 4] = core::array::from_fn(|i| (a - 1) + (b + i - 1) * CS);
        let base_idx = usizex4::from_array(ab_indices);
        let right_idxs = base_idx + usizex4::splat(2 * Self::CS_2);
        let left_idxs = base_idx + usizex4::splat(3 * Self::CS_2);
        right_vals.scatter(&mut self.face_masks, right_idxs);
        left_vals.scatter(&mut self.face_masks, left_idxs);

        for col in 0..4 {
            let b_cur = b + col;
            let ab = a_ + b_cur;
            let mut bits_here = trans_mask[ab] & Self::P_MASK;
            if bits_here == 0 {
                continue;
            }

            let not_front_col = !opaque_mask[ab + Self::CS_P] >> 1;
            let not_back_col = !opaque_mask[ab - Self::CS_P] >> 1;
            let not_right_col = !opaque_mask[ab + 1] >> 1;
            let not_left_col = !opaque_mask[ab - 1] >> 1;
            let not_col_up = !(opaque_mask[ab] >> 1);
            let not_col_down = !(opaque_mask[ab] << 1);

            let ab_ = ab * Self::CS_P;
            let ba_index = (b_cur - 1) + (a - 1) * CS;
            let ab_index = (a - 1) + (b_cur - 1) * CS;
            let up_faces = ba_index;
            let down_faces = ba_index + Self::CS_2;
            let right_faces = ab_index + 2 * Self::CS_2;
            let left_faces = ab_index + 3 * Self::CS_2;
            let front_faces = ba_index + 4 * Self::CS_2;
            let back_faces = ba_index + 5 * Self::CS_2;

            while bits_here != 0 {
                let c = bits_here.trailing_zeros() as usize;
                let c_mask = 1 << c;
                let unpadded_c_mask = c_mask >> 1;
                bits_here &= !c_mask;
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

    fn face_merging(&mut self, voxels: &[M]) {
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
                            voxels[Self::get_axis_index(axis, forward + 1, bit_pos + 1, layer + 1)];

                        if (bits_next >> bit_pos & 1) != 0
                            && v_type
                                == voxels[Self::get_axis_index(
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
                                    != voxels[Self::get_axis_index(
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

                        let (x, y, z) = match face {
                            0 => (mesh_front, mesh_up, mesh_left),
                            1 => (mesh_front + mesh_length, mesh_up, mesh_left),
                            2 => (mesh_up, mesh_front + mesh_length, mesh_left),
                            3 => (mesh_up, mesh_front, mesh_left),
                            _ => unreachable!(),
                        };

                        self.quads[face].push(Q::new(
                            x as u8,
                            y as u8,
                            z as u8,
                            mesh_length as u8,
                            mesh_width as u8,
                            v_type,
                        ));
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
                            voxels[Self::get_axis_index(axis, right + 1, forward + 1, bit_pos)];
                        let forward_merge_i = right_cs + (bit_pos - 1);
                        let right_merged_ref = &mut self.right_merged[bit_pos - 1];

                        if *right_merged_ref == 0
                            && (bits_forward >> bit_pos & 1) != 0
                            && v_type
                                == voxels
                                    [Self::get_axis_index(axis, right + 1, forward + 2, bit_pos)]
                        {
                            self.forward_merged[forward_merge_i] += 1;
                            continue;
                        }

                        if (bits_right >> bit_pos & 1) != 0
                            && self.forward_merged[forward_merge_i]
                                == self.forward_merged[(right_cs + CS) + (bit_pos - 1)]
                            && v_type
                                == voxels
                                    [Self::get_axis_index(axis, right + 2, forward + 1, bit_pos)]
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

                        self.quads[face].push(Q::new(
                            mesh_left as u8 + (if face == 4 { mesh_width } else { 0 }),
                            mesh_front as u8,
                            mesh_up as u8,
                            mesh_width,
                            mesh_length,
                            v_type,
                        ));
                    }
                }
            }
        }
    }

    /// Meshes a voxel buffer representing a chunk, using an opaque and transparent mask with 1 u64 per column with 1 bit per voxel in the column,
    /// signaling if the voxel is opaque or transparent.
    /// This is ~4x faster than the regular mesh method but requires maintaining 2 masks for each chunk.
    /// See https://github.com/Inspirateur/binary-greedy-meshing?tab=readme-ov-file#what-to-do-with-mesh_dataquads for using the output
    pub fn fast_mesh(&mut self, voxels: &[M], opaque_mask: &[u64], trans_mask: &[u64]) {
        self.fast_face_culling(voxels, opaque_mask, trans_mask);
        self.face_merging(voxels);
    }

    /// Meshes a voxel buffer representing a chunk, using a BTreeSet signaling which voxel values are transparent.
    /// This is ~4x slower than the fast_mesh method but does not require maintaining 2 masks for each chunk.
    /// See https://github.com/Inspirateur/binary-greedy-meshing?tab=readme-ov-file#what-to-do-with-mesh_dataquads for using the output
    pub fn mesh(&mut self, voxels: &[M], is_transparent: impl Fn(M) -> bool) {
        self.face_culling(voxels, is_transparent);
        self.face_merging(voxels);
    }

    pub fn fast_mesh_from_chunks(
        &mut self,
        center: &[M],
        neighbors: [Option<&[M]>; 6],
        is_transparent: impl Fn(M) -> bool,
    ) where
        M: Default,
    {
        let padded = Self::assemble_padded(center, neighbors);
        let opaque_mask = Self::compute_opaque_mask(&padded, &is_transparent);
        let trans_mask = Self::compute_transparent_mask(&padded, &is_transparent);
        self.fast_mesh(&padded, &opaque_mask, &trans_mask);
    }

    pub fn mesh_from_chunks(
        &mut self,
        center: &[M],
        neighbors: [Option<&[M]>; 6],
        is_transparent: impl Fn(M) -> bool,
    ) where
        M: Default,
    {
        let padded = Self::assemble_padded(center, neighbors);
        self.mesh(&padded, is_transparent);
    }

    #[inline]
    /// v1 is not AIR
    fn face_value(v1: M, v2: M, is_transparent: impl Fn(M) -> bool) -> u64 {
        (v2.is_air() || (v1 != v2 && is_transparent(v2))) as u64
    }

    #[inline]
    fn get_axis_index(axis: usize, a: usize, b: usize, c: usize) -> usize {
        // TODO: figure out how to shuffle this around to make it work with YZX
        let (csp, csp2) = (Self::CS_P, Self::CS_P2);
        match axis {
            0 => b + (a * csp) + (c * csp2),
            1 => b + (c * csp) + (a * csp2),
            _ => c + (a * csp) + (b * csp2),
        }
    }

    /// Compute Mesh indices for a given amount of quads
    pub fn indices(num_quads: usize) -> Vec<u32> {
        // Each quads is made of 2 triangles which require 6 indices
        // The indices are the same regardless of the face
        let mut res = Vec::with_capacity(num_quads * 6);
        for i in 0..num_quads as u32 {
            res.push((i << 2) | 2);
            res.push(i << 2);
            res.push((i << 2) | 1);
            res.push((i << 2) | 1);
            res.push((i << 2) | 3);
            res.push((i << 2) | 2);
        }
        res
    }

    pub fn pad_linearize(x: usize, y: usize, z: usize) -> usize {
        z + 1 + (x + 1) * Self::CS_P + (y + 1) * Self::CS_P2
    }

    /// Compute an opacity mask from a voxel buffer and a BTreeSet specifying which voxel values are transparent
    pub fn compute_opaque_mask(voxels: &[M], is_transparent: impl Fn(M) -> bool) -> Box<[u64]> {
        let mut opaque_mask = vec![0; Self::CS_P2].into_boxed_slice();
        // Fill the opacity mask
        for (i, voxel) in voxels.iter().enumerate() {
            // If the voxel is transparent we skip it
            if voxel.is_air() || is_transparent(*voxel) {
                continue;
            }
            let (r, q) = (i / Self::CS_P, i % Self::CS_P);
            opaque_mask[r] |= 1 << q;
        }
        opaque_mask
    }

    /// Compute a transparent mask from a voxel buffer and a BTreeSet specifying which voxel values are transparent
    pub fn compute_transparent_mask(
        voxels: &[M],
        is_transparent: impl Fn(M) -> bool,
    ) -> Box<[u64]> {
        let mut trans_mask = vec![0; Self::CS_P2].into_boxed_slice();
        // Fill the opacity mask
        for (i, voxel) in voxels.iter().enumerate() {
            // If the voxel is opaque we skip it
            if voxel.is_air() || !is_transparent(*voxel) {
                continue;
            }
            let (r, q) = (i / Self::CS_P, i % Self::CS_P);
            trans_mask[r] |= 1 << q;
        }
        trans_mask
    }
}

#[cfg(test)]
mod tests {
    use crate::RichMesher;
    use alloc::{boxed::Box, collections::btree_set::BTreeSet};

    pub const CS: usize = 62;

    /// Show quad output on a simple 2 voxels case
    #[test]
    fn test_output() {
        extern crate std;
        let mut voxels = [0; RichMesher::CS_P3];
        voxels[RichMesher::pad_linearize(0, 0, 0)] = 1;
        voxels[RichMesher::pad_linearize(0, 1, 0)] = 1;

        let mut mesher = RichMesher::new();
        let opaque_mask = RichMesher::compute_opaque_mask(&voxels, |_| false);
        let trans_mask = vec![0; RichMesher::CS_P2].into_boxed_slice();
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
        let opaque_mask = RichMesher::compute_opaque_mask(voxels.as_slice(), |_| false);
        let trans_mask = RichMesher::compute_transparent_mask(voxels.as_slice(), |v| {
            transparent_blocks.contains(&v)
        });
        let mut mesher1 = RichMesher::new();
        mesher1.mesh(voxels.as_slice(), |v| transparent_blocks.contains(&v));
        let mut mesher2 = RichMesher::new();
        mesher2.fast_mesh(voxels.as_slice(), &opaque_mask, &trans_mask);
        assert_eq!(mesher1.quads, mesher2.quads);
    }

    fn test_buffer() -> Box<[u32; RichMesher::CS_P3]> {
        let mut voxels = Box::new([0; RichMesher::CS_P3]);
        for x in 0..CS {
            for y in 0..CS {
                for z in 0..CS {
                    voxels[RichMesher::pad_linearize(x, y, z)] = transparent_sphere(x, y, z);
                }
            }
        }
        voxels
    }

    fn transparent_sphere(x: usize, y: usize, z: usize) -> u32 {
        if x == 8 {
            2
        } else if (x as i32 - 31).pow(2) + (y as i32 - 31).pow(2) + (z as i32 - 31).pow(2) < 16 {
            1
        } else {
            0
        }
    }
}
