const CS: usize = 62;
const CS_P: usize = CS + 2;
const CS_2: usize = CS * CS;
const CS_P2: usize = CS_P * CS_P;
const P_MASK: u64 = !(1 << 63 | 1);

pub struct MeshData {
    // CS_2 * 6
    face_masks: Vec<u64>,
    // CS_P2
    opaque_mask: Vec<u64>,
    // CS_2
    forward_merged: Vec<u8>,
    // CS
    right_merged: Vec<u8>,
    vertices: Vec<usize>,
    vertex_count: usize,
    max_vertices: usize,
    face_vertex_begin: [usize; 6],
    face_vertex_length: [usize; 6],
}

// Passing &mut MeshData instead of returning MeshData allows the caller to reuse buffers
pub fn mesh(voxels: &[u16], mesh_data: &mut MeshData) {
    mesh_data.vertex_count = 0;
    let mut vertex_i: usize = 0;

    let opaque_mask = &mut mesh_data.opaque_mask;
    let face_masks = &mut mesh_data.face_masks;
    let forward_merged = &mut mesh_data.forward_merged;
    let righ_merged = &mut mesh_data.right_merged;

    // Hidden face culling
    for a in 1..(CS_P-1) {
        let a_cs_p = a * CS_P;

        for b in 1..(CS_P-1) {
            let column_bits = opaque_mask[(a * CS_P) + b] & P_MASK;
            let ba_index = (b - 1) + (a - 1) * CS;
            let ab_index = (a - 1) + (b - 1) * CS;

            face_masks[ba_index + 0 * CS_2] = (column_bits & !opaque_mask[a_cs_p + CS_P + b]) >> 1;
            face_masks[ba_index + 1 * CS_2] = (column_bits & !opaque_mask[a_cs_p - CS_P + b]) >> 1;

            face_masks[ab_index + 2 * CS_2] = (column_bits & !opaque_mask[a_cs_p + (b + 1)]) >> 1;
            face_masks[ab_index + 3 * CS_2] = (column_bits & !opaque_mask[a_cs_p + (b - 1)]) >> 1;

            face_masks[ba_index + 4 * CS_2] = column_bits & !(opaque_mask[a_cs_p + b] >> 1);
            face_masks[ba_index + 5 * CS_2] = column_bits & !(opaque_mask[a_cs_p + b] << 1);
        }
    }

    // Greedy meshing faces 0-3
    for face in 0..=3 {
        let axis = face / 2;

        let face_vertex_begin = vertex_i;

        for layer in 0..CS {
            let bits_location = layer * CS + face * CS_2;

            for forward in 0..CS {
                let mut bits_here = face_masks[forward + bits_location];
                if bits_here == 0 { continue; }

                let bits_next = if forward + 1 < CS {
                    face_masks[(forward + 1) + bits_location]
                } else {
                    0
                };

                let mut right_merged = 1;
                while bits_here != 0 {
                    let bit_pos = bits_here.trailing_zeros() as usize;

                    let v_type = voxels[get_axis_index(axis, forward + 1, bit_pos + 1, layer + 1)];

                    if (bits_next >> bit_pos & 1) != 0 && v_type == voxels[get_axis_index(axis, forward + 2, bit_pos + 1, layer + 1)] {
                        forward_merged[bit_pos] += 1;
                        bits_here &= !(1 << bit_pos);
                        continue;
                    }

                    for right in (bit_pos+1)..CS {
                        if (bits_here >> right & 1) == 0 
                            || forward_merged[bit_pos]  != forward_merged[right] 
                            || v_type != voxels[get_axis_index(axis, forward + 1, right + 1, layer + 1)] 
                        {
                            break;
                        }
                        forward_merged[right] = 0;
                        right_merged += 1;
                    }
                    bits_here &= !((1 << (bit_pos + right_merged)) - 1);

                    let mesh_front = forward - forward_merged[bit_pos] as usize;
                    let mesh_left = bit_pos;
                    let mesh_up = layer + (!face & 1);

                    let mesh_width = right_merged;
                    let mesh_length = (forward_merged[bit_pos] + 1) as usize;

                    forward_merged[bit_pos] = 0;
                    right_merged = 1;

                    let v_type = v_type as usize;

                    let quad = match face {
                        0 => get_quad(mesh_front, mesh_up, mesh_left, mesh_length, mesh_width, v_type),
                        1 => get_quad(mesh_front + mesh_length as usize, mesh_up, mesh_left, mesh_length, mesh_width, v_type),
                        2 => get_quad(mesh_up, mesh_front + mesh_length as usize, mesh_left, mesh_length, mesh_width, v_type),
                        3 => get_quad(mesh_up, mesh_front, mesh_left, mesh_length, mesh_width, v_type),
                        _ => unreachable!()
                    };

                    insert_quad(&mut mesh_data.vertices, quad, &mut vertex_i, &mut mesh_data.max_vertices);
                }
            }
        }

        let face_vertex_length = vertex_i - face_vertex_begin;
        mesh_data.face_vertex_begin[face] = face_vertex_begin;
        mesh_data.face_vertex_length[face] =face_vertex_length;
    }

    // Greedy meshing faces 4-5
    for face in 4..6 {
        let axis = face / 2;

        let face_vertex_begin = vertex_i;

        for forward in 0..CS {
            let bits_location = forward * CS + face * CS_2;
            let bits_forward_location = (forward + 1) * CS + face * CS_2;

            for right in 0..CS {
                let mut bits_here = face_masks[right + bits_location];
                if bits_here == 0 {
                    continue;
                }
                
                let bits_forward = if forward < CS - 1 { face_masks[right + bits_forward_location] } else { 0 };
                let bits_right = if right < CS - 1 { face_masks[right + 1 + bits_location] } else { 0 };
                let right_cs = right * CS;

                while bits_here != 0 {
                    let bit_pos = bits_here.trailing_zeros() as usize;

                    bits_here &= !(1 << bit_pos);

                    let v_type = voxels[get_axis_index(axis, right + 1, forward + 1, bit_pos)];
                    let forward_merge_i = right_cs + (bit_pos - 1);
                    let right_merged_ref = &mut righ_merged[bit_pos - 1];

                    if *right_merged_ref == 0 && (bits_forward >> bit_pos & 1) != 0 && v_type == voxels[get_axis_index(axis, right + 1, forward + 2, bit_pos)] {
                        forward_merged[forward_merge_i] += 1;
                        continue;
                    }

                    if (bits_right >> bit_pos & 1) != 0 
                        && forward_merged[forward_merge_i] == forward_merged[(right_cs + CS) + (bit_pos - 1)] 
                        && v_type == voxels[get_axis_index(axis, right + 2, forward + 1, bit_pos)] 
                    {
                        forward_merged[forward_merge_i] = 0;
                        *right_merged_ref += 1;
                        continue;
                    }

                    let mesh_left = right - *right_merged_ref as usize;
                    let mesh_front = forward - forward_merged[forward_merge_i] as usize;
                    let mesh_up = bit_pos - 1 + (!face & 1);

                    let mesh_width = 1 + *right_merged_ref;
                    let mesh_length = 1 + forward_merged[forward_merge_i];

                    forward_merged[forward_merge_i] = 0;
                    *right_merged_ref = 0;

                    let quad = get_quad(
                        mesh_left + (if face == 4 { mesh_width } else { 0 }) as usize, 
                        mesh_front, 
                        mesh_up, 
                        mesh_width as usize, 
                        mesh_length as usize, 
                        v_type as usize
                    );

                    insert_quad(&mut mesh_data.vertices, quad, &mut vertex_i, &mut mesh_data.max_vertices);
                }
            }
        }

        let face_vertex_length = vertex_i - face_vertex_begin;
        mesh_data.face_vertex_begin[face] = face_vertex_begin;
        mesh_data.face_vertex_length[face] = face_vertex_length;
    }
    mesh_data.vertex_count = vertex_i + 1;
}

#[inline]
fn get_axis_index(axis: usize, a: usize, b: usize, c: usize) -> usize {
    match axis {
        0 => b + (a * CS_P) + (c * CS_P2),
        1 => b + (c * CS_P) + (a * CS_P2),
        _ => c + (a * CS_P) + (b * CS_P2)
    }
}

#[inline]
fn insert_quad(vertices: &mut Vec<usize>, quad: usize, vertex_i: &mut usize, max_vertices: &mut usize) {
    if *vertex_i >= *max_vertices - 6 {
      vertices.resize(*max_vertices * 2, 0);
      *max_vertices *= 2;
    }
  
    vertices[*vertex_i] = quad;
  
    *vertex_i += 1;
}
  
#[inline]
fn get_quad(x: usize, y: usize, z: usize, w: usize, h: usize, v_type: usize) -> usize {
    (v_type << 32) | (h << 24) | (w << 18) | (z << 12) | (y << 6) | x
}