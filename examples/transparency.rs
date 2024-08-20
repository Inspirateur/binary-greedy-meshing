use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin}, 
    prelude::*, 
    render::{
        mesh::{Indices, MeshVertexAttribute, PrimitiveTopology, VertexAttributeValues}, 
        render_asset::RenderAssetUsages, 
        render_resource::VertexFormat, 
        settings::{RenderCreation, WgpuFeatures, WgpuSettings}, 
        RenderPlugin
    }};
use binary_greedy_meshing as bgm;

pub const ATTRIBUTE_VOXEL_DATA: MeshVertexAttribute =
    MeshVertexAttribute::new("VoxelData", 48757581, VertexFormat::Uint32x2);

const LAYER_W: usize = 3;
const SIZE: usize = 10;
const MASK6: u32 = 0b111_111;

fn main() {
    App::new()
    .init_resource::<WireframeConfig>()
    .insert_resource(Msaa::Sample4)
    .add_plugins((
        DefaultPlugins.set(RenderPlugin {
            render_creation: RenderCreation::Automatic(WgpuSettings {
                features: WgpuFeatures::POLYGON_MODE_LINE,
                ..Default::default()
            }),
            ..default()
        }),
        WireframePlugin,
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

    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(50.0, 100.0, 50.0)),
        point_light: PointLight {
            range: 200.0,
            //intensity: 8000.0,
            ..Default::default()
        },
        ..Default::default()
    });
    commands.spawn(Camera3dBundle {
        transform: Transform::from_translation(Vec3::new(40.0, 20.0, 50.0))
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..Default::default()
    });
    let [solid_mesh, transp_mesh1, transp_mesh2] = generate_meshes();
    let solid_mesh = meshes.add(solid_mesh);
    let transp_mesh1 = meshes.add(transp_mesh1);
    let transp_mesh2 = meshes.add(transp_mesh2);

    commands.spawn(PbrBundle {
        mesh: solid_mesh,
        material: materials.add(StandardMaterial {
            base_color: Color::linear_rgba(0., 0., 0., 1.0),
            ..Default::default()
        }),
        ..Default::default()
    });

    commands.spawn(PbrBundle {
        mesh: transp_mesh1,
        material: materials.add(StandardMaterial {
            base_color: Color::linear_rgba(1., 0.5, 0.5, 0.3),
            ..Default::default()
        }),
        ..Default::default()
    });
    
    commands.spawn(PbrBundle {
        mesh: transp_mesh2,
        material: materials.add(StandardMaterial {
            base_color: Color::linear_rgba(0.5, 1., 0.5, 0.3),
            ..Default::default()
        }),
        ..Default::default()
    });

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: light_consts::lux::OVERCAST_DAY,
    });
}

/// Generate 1 mesh per block type for simplicity, in practice we would use a texture array and a custom shader instead 
fn generate_meshes() -> [Mesh; 3] {
    let voxels = voxel_buffer();
    let mut mesh_data = bgm::MeshData::new();
    // Fill the opacity mask
    for (i, voxel) in voxels.iter().enumerate() {
        // Transpancy is not handled yet so we treat every block except Air as opaque
        if *voxel == 0 {
            continue;
        }
        let (q, r) = (i/bgm::CS_P, i%bgm::CS_P);
        mesh_data.opaque_mask[q] |= 1 << r;
    }
    bgm::mesh(&voxels, &mut mesh_data);
    let mut positions: [_; 3] = core::array::from_fn(|_| Vec::new());
    let mut normals: [_; 3] = core::array::from_fn(|_| Vec::new());
    let mut indices: [_; 3] = core::array::from_fn(|_| Vec::new());
    for (face_n, (start, num_quads)) in mesh_data.face_vertex_begin.iter().zip(mesh_data.face_vertex_length).enumerate() {
        let face: bgm::Face = (face_n as u8).into();
        let n = face.n();
        for quad in mesh_data.quads[*start..(*start+num_quads)].iter() {
            let voxel_i = (quad >> 32) as usize -1;
            let vertices_packed = face.vertices_packed(*quad);
            for vertex_packed in vertices_packed.iter() {
                let x = *vertex_packed & MASK6;
                let y = (*vertex_packed >> 6) & MASK6;
                let z = (*vertex_packed >> 12) & MASK6;
                positions[voxel_i].push([x as f32, y as f32, z as f32]);
                normals[voxel_i].push(n.clone());
            }
        }
    }
    for i in 0..positions.len() {
        indices[i] = bgm::indices(positions[i].len());
    }
    core::array::from_fn(|i| {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
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

fn voxel_buffer() -> [u16; bgm::CS_P3] {
    let mut voxels = [0; bgm::CS_P3];
    for x in 0..bgm::CS {
        for y in 0..bgm::CS {
            for z in 0..bgm::CS {
                voxels[bgm::pad_linearize(x, y, z)] = transparent_sandwich(x, y, z);
            }
        }
    }
    voxels
}

/// 0 = Air, 1 = Solid, 2 = transparent, 3 = other transparent
/// This returns a "sandwich" with 1 solid layer and 2 transparent layers
fn transparent_sandwich(x: usize, y: usize, z: usize) -> u16 {
    if y > SIZE || z > SIZE {
        return 0;
    }
    if x < LAYER_W {
        return 1;
    } else if x < LAYER_W*2 {
        return 2;
    } else if x < LAYER_W*3 {
        return 3
    };
    0
}
