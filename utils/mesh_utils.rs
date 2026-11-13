use std::collections::HashMap;
mod model_utils;

use bevy::{
    asset::RenderAssetUsages,
    ecs::system::Commands,
    mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
};
use binary_greedy_meshing as bgm;

use model_utils::VoxelType;

#[derive(Debug, Default)]
struct MeshAttributes {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
}

pub fn spawn_quads(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    quads: &[Vec<bgm::Quad>; 6],
    translation: Vec3,
) {
    let mut meshes_attrs: HashMap<u16, MeshAttributes> = HashMap::new();
    for (face_n, quads) in quads.iter().enumerate() {
        let face: bgm::Face = (face_n as u8).into();
        let n = face.n().map(|v| v as f32);
        for &quad in quads {
            let voxel_i = quad.voxel_id() as u16;
            let vertices_packed = face.vertices_packed(quad);
            for &vertex in vertices_packed.iter() {
                let [x, y, z] = vertex.xyz();
                meshes_attrs
                    .entry(voxel_i)
                    .or_insert_with(|| MeshAttributes::default())
                    .positions
                    .push([x as f32, y as f32, z as f32]);
                meshes_attrs
                    .entry(voxel_i)
                    .or_insert_with(|| MeshAttributes::default())
                    .normals
                    .push(n.clone());
            }
        }
    }
    for (v, attrs) in meshes_attrs.into_iter() {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            VertexAttributeValues::Float32x3(attrs.positions.clone()),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            VertexAttributeValues::Float32x3(attrs.normals),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_UV_0,
            VertexAttributeValues::Float32x2(vec![[0.0; 2]; attrs.positions.len()]),
        );
        mesh.insert_indices(Indices::U32(bgm::indices(attrs.positions.len() / 4)));
        let voxel_type = unsafe { std::mem::transmute::<_, VoxelType>(v) };
        let material = match voxel_type {
            VoxelType::Air => unreachable!(),
            VoxelType::Rock => MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::linear_rgba(0.8, 0.8, 0.8, 1.0),
                alpha_mode: AlphaMode::Opaque,
                ..Default::default()
            })),
            VoxelType::Dirt => MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::linear_rgba(0.8, 0.6, 0.4, 1.0),
                alpha_mode: AlphaMode::Opaque,
                ..Default::default()
            })),
            VoxelType::Grass => MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::linear_rgba(0.5, 0.9, 0.4, 1.0),
                alpha_mode: AlphaMode::Opaque,
                ..Default::default()
            })),
            VoxelType::Wood => MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::linear_rgba(0.9, 0.7, 0.5, 1.0),
                alpha_mode: AlphaMode::Opaque,
                ..Default::default()
            })),
            VoxelType::Leaves => MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::linear_rgba(0.3, 0.7, 0.3, 1.0),
                alpha_mode: AlphaMode::Opaque,
                ..Default::default()
            })),
            VoxelType::Glass => MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::linear_rgba(0.8, 0.85, 0.8, 0.5),
                alpha_mode: AlphaMode::Blend,
                ..Default::default()
            })),
        };
        let mesh3d = Mesh3d(meshes.add(mesh));
        commands.spawn((mesh3d, material, Transform::from_translation(translation)));
    }
}
