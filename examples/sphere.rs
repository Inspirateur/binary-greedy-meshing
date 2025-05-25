use std::collections::BTreeSet;

use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
    render::{
        mesh::{Indices, MeshVertexAttribute, PrimitiveTopology, VertexAttributeValues},
        render_asset::RenderAssetUsages,
        render_resource::VertexFormat,
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
        RenderPlugin,
    },
};
use binary_greedy_meshing as bgm;

pub const ATTRIBUTE_VOXEL_DATA: MeshVertexAttribute =
    MeshVertexAttribute::new("VoxelData", 48757581, VertexFormat::Uint32x2);

const SIZE: usize = 16;
const SIZE2: usize = SIZE.pow(2);
const MASK6: u32 = 0b111_111;

fn main() {
    App::new()
        .init_resource::<WireframeConfig>()
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..Default::default()
                }),
                ..default()
            }),
            WireframePlugin::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut wireframe_config: ResMut<WireframeConfig>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    wireframe_config.global = true;

    commands.spawn((
        Transform::from_translation(Vec3::new(50.0, 100.0, 50.0)),
        PointLight {
            range: 200.0,
            //intensity: 8000.0,
            ..Default::default()
        },
    ));
    commands.spawn((
        Camera3d::default(),
        Msaa::Sample4,
        Transform::from_translation(Vec3::new(60.0, 60.0, 100.0))
            .looking_at(Vec3::new(31.0, 31.0, 31.0), Vec3::Y),
    ));
    let mesh = Mesh3d(meshes.add(generate_mesh()));

    commands.spawn((
        mesh,
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::linear_rgba(0.1, 0.1, 0.1, 1.0),
            ..Default::default()
        })),
    ));

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: light_consts::lux::OVERCAST_DAY,
        ..Default::default()
    });
}

/// Generate 1 mesh per block type for simplicity, in practice we would use a texture array and a custom shader instead
fn generate_mesh() -> Mesh {
    let voxels = voxel_buffer();
    let mut mesher = bgm::Mesher::new();
    let opaque_mask = bgm::compute_opaque_mask(&voxels, &BTreeSet::new());
    let trans_mask = vec![0; bgm::CS_P2].into_boxed_slice();
    mesher.fast_mesh(&voxels, &opaque_mask, &trans_mask);
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for (face_n, quads) in mesher.quads.iter().enumerate() {
        let face: bgm::Face = (face_n as u8).into();
        let n = face.n();
        for quad in quads {
            let vertices_packed = face.vertices_packed(*quad);
            for vertex_packed in vertices_packed.iter() {
                let x = *vertex_packed & MASK6;
                let y = (*vertex_packed >> 6) & MASK6;
                let z = (*vertex_packed >> 12) & MASK6;
                positions.push([x as f32, y as f32, z as f32]);
                normals.push(n.clone());
            }
        }
    }
    let indices = bgm::indices(positions.len());
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        VertexAttributeValues::Float32x2(vec![[0.0; 2]; positions.len()]),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float32x3(positions),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        VertexAttributeValues::Float32x3(normals),
    );
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn voxel_buffer() -> [u16; bgm::CS_P3] {
    let mut voxels = [0; bgm::CS_P3];
    for x in 0..bgm::CS {
        for y in 0..bgm::CS {
            for z in 0..bgm::CS {
                voxels[bgm::pad_linearize(x, y, z)] = sphere(x, y, z);
            }
        }
    }
    voxels
}

/// This returns an opaque sphere
fn sphere(x: usize, y: usize, z: usize) -> u16 {
    if (x as i32 - 31).pow(2) + (y as i32 - 31).pow(2) + (z as i32 - 31).pow(2) < SIZE2 as i32 {
        1
    } else {
        0
    }
}
