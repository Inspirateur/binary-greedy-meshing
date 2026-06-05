use bevy::{
    asset::RenderAssetUsages,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    mesh::{Indices, MeshVertexAttribute, PrimitiveTopology, VertexAttributeValues},
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
    render::{
        RenderPlugin,
        render_resource::VertexFormat,
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
    },
};
use binary_greedy_meshing::{self as bgm, MiniMesher};

pub const ATTRIBUTE_VOXEL_DATA: MeshVertexAttribute =
    MeshVertexAttribute::new("VoxelData", 48757581, VertexFormat::Uint32x2);

const LAYER_W: usize = 3;
const SIZE: usize = 10;
const CS: usize = 62;

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
            FreeCameraPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

/// 0 = Air, 1 = Solid, 2 = transparent, 3 = other transparent
/// This returns a "sandwich" with 1 solid layer and 2 transparent layers
fn transparent_sandwich(x: usize, y: usize, z: usize) -> u8 {
    if y > SIZE || z > SIZE {
        return 0;
    }
    if x < LAYER_W {
        return 1;
    } else if x < LAYER_W * 2 {
        return 2;
    } else if x < LAYER_W * 3 {
        return 3;
    };
    0
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
        FreeCamera::default(),
        Transform::from_translation(Vec3::new(40.0, 20.0, 50.0))
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));
    let [solid_mesh, transp_mesh1, transp_mesh2] = generate_meshes();
    let solid_mesh = Mesh3d(meshes.add(solid_mesh));
    let transp_mesh1 = Mesh3d(meshes.add(transp_mesh1));
    let transp_mesh2 = Mesh3d(meshes.add(transp_mesh2));
    commands.spawn((
        solid_mesh,
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::linear_rgba(0.8, 0.8, 0.8, 1.0),
            alpha_mode: AlphaMode::AlphaToCoverage,
            ..Default::default()
        })),
    ));
    commands.spawn((
        transp_mesh1,
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::linear_rgba(1., 0.3, 0.3, 0.3),
            alpha_mode: AlphaMode::AlphaToCoverage,
            ..Default::default()
        })),
    ));
    commands.spawn((
        transp_mesh2,
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::linear_rgba(0.3, 1., 0.3, 0.3),
            alpha_mode: AlphaMode::AlphaToCoverage,
            ..Default::default()
        })),
    ));

    commands.spawn(AmbientLight {
        color: Color::WHITE,
        brightness: light_consts::lux::OVERCAST_DAY,
        ..Default::default()
    });
}

/// Generate 1 mesh per block type for simplicity, in practice we would use a texture array and a custom shader instead
fn generate_meshes() -> [Mesh; 3] {
    let voxels = voxel_buffer();
    let mut mesher = MiniMesher::new();
    let opaque_mask = MiniMesher::compute_opaque_mask(&voxels, |v| v == 2 || v == 3);
    let trans_mask = MiniMesher::compute_transparent_mask(&voxels, |v| v == 2 || v == 3);
    mesher.fast_mesh(&voxels, &opaque_mask, &trans_mask);
    let mut positions: [_; 3] = core::array::from_fn(|_| Vec::new());
    let mut normals: [_; 3] = core::array::from_fn(|_| Vec::new());
    let mut indices: [_; 3] = core::array::from_fn(|_| Vec::new());
    for (face_n, quads) in mesher.quads.iter().enumerate() {
        let face: bgm::Face = (face_n as u8).into();
        let n = face.n().map(|v| v as f32);
        for &quad in quads {
            let voxel_i = quad.m() as usize - 1;
            let vertices_packed = face.vertices_packed(quad);
            for &vertex in vertices_packed.iter() {
                positions[voxel_i].push(vertex.xyz());
                normals[voxel_i].push(n);
            }
        }
    }
    for i in 0..positions.len() {
        indices[i] = MiniMesher::indices(positions[i].len() / 4);
    }
    core::array::from_fn(|i| {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            VertexAttributeValues::Float32x3(positions[i].clone()),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            VertexAttributeValues::Float32x3(normals[i].clone()),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_UV_0,
            VertexAttributeValues::Float32x2(vec![[0.0; 2]; positions[i].len()]),
        );
        mesh.insert_indices(Indices::U32(indices[i].clone()));
        mesh
    })
}

fn voxel_buffer() -> [u8; MiniMesher::CS_P3] {
    let mut voxels = [0; MiniMesher::CS_P3];
    for x in 0..CS {
        for y in 0..CS {
            for z in 0..CS {
                voxels[MiniMesher::pad_linearize(x, y, z)] = transparent_sandwich(x, y, z);
            }
        }
    }
    voxels
}
