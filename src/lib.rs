#![no_std]

#[macro_use]
extern crate alloc;

mod face;

use alloc::{boxed::Box, collections::btree_set::BTreeSet, vec::Vec};
pub use face::*;
pub const CS: usize = 62;
const CS_2: usize = CS * CS;
pub const CS_P: usize = CS + 2;
pub const CS_P2: usize = CS_P * CS_P;
pub const CS_P3: usize = CS_P * CS_P * CS_P;


#[derive(Debug)]
pub struct MeshData {
    // Output
    pub quads: [Vec<u64>; 6],
    // Internal buffers
    /// CS_2 * 6
    face_masks: Box<[u64]>,
    /// CS_2
    forward_merged: Box<[u8]>,
    /// CS
    right_merged: Box<[u8]>,
}

impl MeshData {
    pub fn new() -> Self {
        Self { 
            face_masks: vec![0; CS_2*6].into_boxed_slice(), 
            forward_merged: vec![0; CS_2].into_boxed_slice(), 
            right_merged: vec![0; CS].into_boxed_slice(), 
            quads: core::array::from_fn(|_| Vec::new()), 
        }
    }

    pub fn clear(&mut self) {
        self.face_masks.fill(0);
        self.forward_merged.fill(0);
        self.right_merged.fill(0);
        for i in 0..self.quads.len() {
            self.quads[i].clear();
        }
    }
}

#[inline]
/// v1 is not AIR
fn face_value(v1: u16, v2: u16, transparents: &BTreeSet<u16>) -> u64 {
    (v2 == 0 || (v1 != v2 && transparents.contains(&v2))) as u64
}

// Passing &mut MeshData instead of returning MeshData allows the caller to reuse buffers
pub fn mesh(voxels: &[u16], mesh_data: &mut MeshData, transparents: BTreeSet<u16>) {
    // Hidden face culling
    for a in 1..(CS_P-1) {
        let a_cs_p = a * CS_P;

        for b in 1..(CS_P-1) {
            let ab = (a_cs_p + b) * CS_P;
            let ba_index = (b - 1) + (a - 1) * CS;
            let ab_index = (a - 1) + (b - 1) * CS;

            for c in 1..(CS_P-1) {
                let abc = ab + c;
                let v1 = voxels[abc];
                if v1 == 0 {
                    continue;
                }
                mesh_data.face_masks[ba_index + 0 * CS_2] |= face_value(v1, voxels[abc + CS_P2], &transparents) << (c-1);
                mesh_data.face_masks[ba_index + 1 * CS_2] |= face_value(v1, voxels[abc - CS_P2], &transparents) << (c-1);
                
                mesh_data.face_masks[ab_index + 2 * CS_2] |= face_value(v1, voxels[abc + CS_P], &transparents) << (c-1);
                mesh_data.face_masks[ab_index + 3 * CS_2] |= face_value(v1, voxels[abc - CS_P], &transparents) << (c-1);
    
                mesh_data.face_masks[ba_index + 4 * CS_2] |= face_value(v1, voxels[abc + 1], &transparents) << c;
                mesh_data.face_masks[ba_index + 5 * CS_2] |= face_value(v1, voxels[abc - 1], &transparents) << c;
            }
        }
    }

    // Greedy meshing faces 0-3
    for face in 0..=3 {
        let axis = face / 2;

        for layer in 0..CS {
            let bits_location = layer * CS + face * CS_2;

            for forward in 0..CS {
                let mut bits_here = mesh_data.face_masks[forward + bits_location];
                if bits_here == 0 { continue; }

                let bits_next = if forward + 1 < CS {
                    mesh_data.face_masks[(forward + 1) + bits_location]
                } else {
                    0
                };

                let mut right_merged = 1;
                while bits_here != 0 {
                    let bit_pos = bits_here.trailing_zeros() as usize;

                    let v_type = voxels[get_axis_index(axis, forward + 1, bit_pos + 1, layer + 1)];

                    if (bits_next >> bit_pos & 1) != 0 && v_type == voxels[get_axis_index(axis, forward + 2, bit_pos + 1, layer + 1)] {
                        mesh_data.forward_merged[bit_pos] += 1;
                        bits_here &= !(1 << bit_pos);
                        continue;
                    }

                    for right in (bit_pos+1)..CS {
                        if (bits_here >> right & 1) == 0 
                            || mesh_data.forward_merged[bit_pos]  != mesh_data.forward_merged[right] 
                            || v_type != voxels[get_axis_index(axis, forward + 1, right + 1, layer + 1)] 
                        {
                            break;
                        }
                        mesh_data.forward_merged[right] = 0;
                        right_merged += 1;
                    }
                    bits_here &= !((1 << (bit_pos + right_merged)) - 1);

                    let mesh_front = forward - mesh_data.forward_merged[bit_pos] as usize;
                    let mesh_left = bit_pos;
                    let mesh_up = layer + (!face & 1);

                    let mesh_width = right_merged;
                    let mesh_length = (mesh_data.forward_merged[bit_pos] + 1) as usize;

                    mesh_data.forward_merged[bit_pos] = 0;
                    right_merged = 1;

                    let v_type = v_type as usize;

                    let quad = match face {
                        0 => get_quad(mesh_front, mesh_up, mesh_left, mesh_length, mesh_width, v_type),
                        1 => get_quad(mesh_front + mesh_length as usize, mesh_up, mesh_left, mesh_length, mesh_width, v_type),
                        2 => get_quad(mesh_up, mesh_front + mesh_length as usize, mesh_left, mesh_length, mesh_width, v_type),
                        3 => get_quad(mesh_up, mesh_front, mesh_left, mesh_length, mesh_width, v_type),
                        _ => unreachable!()
                    };
                    mesh_data.quads[face].push(quad);
                }
            }
        }
    }

    // Greedy meshing faces 4-5
    for face in 4..6 {
        let axis = face / 2;

        for forward in 0..CS {
            let bits_location = forward * CS + face * CS_2;
            let bits_forward_location = (forward + 1) * CS + face * CS_2;

            for right in 0..CS {
                let mut bits_here = mesh_data.face_masks[right + bits_location];
                if bits_here == 0 {
                    continue;
                }
                
                let bits_forward = if forward < CS - 1 { mesh_data.face_masks[right + bits_forward_location] } else { 0 };
                let bits_right = if right < CS - 1 { mesh_data.face_masks[right + 1 + bits_location] } else { 0 };
                let right_cs = right * CS;

                while bits_here != 0 {
                    let bit_pos = bits_here.trailing_zeros() as usize;

                    bits_here &= !(1 << bit_pos);

                    let v_type = voxels[get_axis_index(axis, right + 1, forward + 1, bit_pos)];
                    let forward_merge_i = right_cs + (bit_pos - 1);
                    let right_merged_ref = &mut mesh_data.right_merged[bit_pos - 1];

                    if *right_merged_ref == 0 && (bits_forward >> bit_pos & 1) != 0 && v_type == voxels[get_axis_index(axis, right + 1, forward + 2, bit_pos)] {
                        mesh_data.forward_merged[forward_merge_i] += 1;
                        continue;
                    }

                    if (bits_right >> bit_pos & 1) != 0 
                        && mesh_data.forward_merged[forward_merge_i] == mesh_data.forward_merged[(right_cs + CS) + (bit_pos - 1)] 
                        && v_type == voxels[get_axis_index(axis, right + 2, forward + 1, bit_pos)] 
                    {
                        mesh_data.forward_merged[forward_merge_i] = 0;
                        *right_merged_ref += 1;
                        continue;
                    }

                    let mesh_left = right - *right_merged_ref as usize;
                    let mesh_front = forward - mesh_data.forward_merged[forward_merge_i] as usize;
                    let mesh_up = bit_pos - 1 + (!face & 1);

                    let mesh_width = 1 + *right_merged_ref;
                    let mesh_length = 1 + mesh_data.forward_merged[forward_merge_i];

                    mesh_data.forward_merged[forward_merge_i] = 0;
                    *right_merged_ref = 0;

                    let quad = get_quad(
                        mesh_left + (if face == 4 { mesh_width } else { 0 }) as usize, 
                        mesh_front, 
                        mesh_up, 
                        mesh_width as usize, 
                        mesh_length as usize, 
                        v_type as usize
                    );
                    mesh_data.quads[face].push(quad);
                }
            }
        }
    }
}

#[inline]
fn get_axis_index(axis: usize, a: usize, b: usize, c: usize) -> usize {
    // TODO: figure out how to shuffle this around to make it work with YZX
    match axis {
        0 => b + (a * CS_P) + (c * CS_P2),
        1 => b + (c * CS_P) + (a * CS_P2),
        _ => c + (a * CS_P) + (b * CS_P2)
    }
}

#[inline]
fn get_quad(x: usize, y: usize, z: usize, w: usize, h: usize, v_type: usize) -> u64 {
    ((v_type << 32) | (h << 24) | (w << 18) | (z << 12) | (y << 6) | x) as u64
}


pub fn indices(num_quads: usize) -> Vec<u32> {
    // Each quads is made of 2 triangles which require 6 indices
    // The indices are the same regardless of the face
    let mut res = Vec::with_capacity(num_quads*6);
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

pub fn pad_linearize(x: usize, y: usize, z: usize) -> usize {
    z + 1 + (x + 1)*CS_P + (y + 1)*CS_P2
}

#[cfg(test)]
mod tests {
    use alloc::collections::btree_set::BTreeSet;
    use crate as bgm;
    const MASK6: u64 = 0b111_111;

    #[derive(Debug)]
    struct Quad {
        x: u64,
        y: u64,
        z: u64,
        w: u64,
        h: u64,
        v_type: u64
    }

    impl From<u64> for Quad {
        fn from(value: u64) -> Self {
            Self {
                x: value & MASK6,
                y: (value >> 6) & MASK6,
                z: (value >> 12) & MASK6,
                w: (value >> 18) & MASK6,
                h: (value >> 24) & MASK6,
                v_type: value >> 32,
            }
        }
    }
    
    #[test]
    fn doesnt_crash() {
        let mut voxels = [0; bgm::CS_P3];
        voxels[bgm::pad_linearize(0, 0, 0)] = 1;
        voxels[bgm::pad_linearize(0, 1, 0)] = 1;
    
        let mut mesh_data = bgm::MeshData::new();

        bgm::mesh(&voxels, &mut mesh_data, BTreeSet::default());
        // mesh_data.quads is the output
        /*/
        for (i, quads) in mesh_data.quads.iter().enumerate() {
            println!("--- Face {i} ---");
            for quad in quads {
                println!("{:?}", Quad::from(*quad));
            }
        }
        */
    }
}