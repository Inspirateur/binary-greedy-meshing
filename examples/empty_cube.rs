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
use binary_greedy_meshing::{self as bgm, MicroMesher};

pub const ATTRIBUTE_VOXEL_DATA: MeshVertexAttribute =
    MeshVertexAttribute::new("VoxelData", 48757581, VertexFormat::Uint32x2);

const CS: usize = 31;

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
            ..Default::default()
        },
    ));
    commands.spawn((
        Camera3d::default(),
        FreeCamera::default(),
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

    commands.spawn(AmbientLight {
        color: Color::WHITE,
        brightness: light_consts::lux::OVERCAST_DAY,
        ..Default::default()
    });
}

/// Generate 1 mesh per block type for simplicity, in practice we would use a texture array and a custom shader instead
fn generate_mesh() -> Mesh {
    let voxels = voxel_buffer();
    let mut mesher = MicroMesher::new();
    mesher.fast_mesh_from_chunks(&voxels, Default::default(), |_| false);
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for (face_n, quads) in mesher.quads.iter().enumerate() {
        let face: bgm::Face = (face_n as u8).into();
        let n = face.n().map(|v| v as f32);
        for quad in quads {
            let vertices_packed = face.vertices_packed(*quad);
            for vertex in vertices_packed.iter() {
                positions.push(vertex.xyz());
                normals.push(n);
            }
        }
    }
    let indices = MicroMesher::indices(positions.len() / 4);
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

fn voxel_buffer() -> [u8; 29791] {
    let mut voxels = [0; CS * CS * CS];
    for x in 0..CS {
        for y in 0..CS {
            for z in 0..CS {
                voxels[z + x * CS + y * CS * CS] = empty_cube(x, y, z);
            }
        }
    }
    voxels
}

fn empty_cube(x: usize, y: usize, z: usize) -> u8 {
    const MIN: usize = 0;
    const MAX: usize = CS - 1;

    if ((x == MIN || x == MAX) && (MIN..=MAX).contains(&y) && (MIN..=MAX).contains(&z))
        || ((y == MIN || y == MAX) && (MIN..=MAX).contains(&x) && (MIN..=MAX).contains(&z))
        || ((z == MIN || z == MAX) && (MIN..=MAX).contains(&x) && (MIN..=MAX).contains(&y))
    {
        1
    } else {
        0
    }
}
