use bevy::{
    asset::RenderAssetUsages,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
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

const SIZE: usize = 16;
const SIZE2: usize = SIZE.pow(2);
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

fn setup(
    mut commands: Commands,
    mut wireframe_config: ResMut<WireframeConfig>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
) {
    wireframe_config.global = true;

    commands.spawn((
        Transform::from_translation(Vec3::new(50.0, 100.0, 50.0)),
        DirectionalLight {
            illuminance: light_consts::lux::HALLWAY,
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

    let texture = asset_server.load_with_settings("atlas.png", |s: &mut _| {
        *s = ImageLoaderSettings {
            sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..default()
            }),
            ..default()
        }
    });

    commands.spawn((
        mesh,
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(texture),
            ..Default::default()
        })),
    ));

    commands.spawn(AmbientLight {
        color: Color::WHITE,
        brightness: light_consts::lux::OVERCAST_DAY,
        ..Default::default()
    });
}

fn generate_mesh() -> Mesh {
    let voxels = voxel_buffer();
    let mut mesher = MiniMesher::new();
    let opaque_mask = MiniMesher::compute_opaque_mask(&voxels, |_| false);
    let trans_mask = vec![0; MiniMesher::CS_P2].into_boxed_slice();
    mesher.fast_mesh(&voxels, &opaque_mask, &trans_mask);

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();

    for (face_n, quads) in mesher.quads.iter().enumerate() {
        let face: bgm::Face = (face_n as u8).into();
        let n = face.n().map(|v| v as f32);

        for quad in quads {
            let material = quad.m();
            let vertices_packed = face.vertices_packed(*quad);

            for vertex in vertices_packed.iter() {
                positions.push(vertex.xyz());
                normals.push(n);
                let mut v_tex = vertex.v() as f32 * 0.5;

                if material == 2 {
                    v_tex += 0.5;
                }

                uvs.push([vertex.u() as f32, v_tex]);
            }
        }
    }

    let indices = MiniMesher::indices(positions.len() / 4);
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float32x3(positions),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        VertexAttributeValues::Float32x3(normals),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::Float32x2(uvs));
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn voxel_buffer() -> [u8; MiniMesher::CS_P3] {
    let mut voxels = [0; MiniMesher::CS_P3];
    for x in 0..CS {
        for y in 0..CS {
            for z in 0..CS {
                voxels[MiniMesher::pad_linearize(x, y, z)] = sphere(x, y, z);
            }
        }
    }
    voxels
}

fn sphere(x: usize, y: usize, z: usize) -> u8 {
    let dist2 = (x as i32 - 31).pow(2) + (y as i32 - 31).pow(2) + (z as i32 - 31).pow(2);
    if dist2 < SIZE2 as i32 {
        let index = x * CS * CS + y * CS + z;
        if index.is_multiple_of(2) { 2 } else { 1 }
    } else {
        0
    }
}
