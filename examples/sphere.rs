use std::collections::BTreeSet;
#[path = "../utils/mesh_utils.rs"]
mod mesh_utils;
#[path = "../utils/model_utils.rs"]
mod model_utils;
use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
    render::{
        RenderPlugin,
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
    },
};
use binary_greedy_meshing as bgm;

use crate::{mesh_utils::spawn_quads, model_utils::opaque_sphere};

const CS: usize = 62;

fn main() {
    App::new()
        .init_resource::<WireframeConfig>()
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..Default::default()
                })),
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
        Transform::from_translation(Vec3::new(45.0, 50.0, 70.0))
            .looking_at(Vec3::new(31.0, 31.0, 31.0), Vec3::Y),
    ));
    let voxels = opaque_sphere::<CS>(CS / 2);
    let mut mesher = bgm::Mesher::<CS>::new();
    let opaque_mask = bgm::compute_opaque_mask::<CS>(&voxels, &BTreeSet::new());
    let trans_mask = vec![0; bgm::Mesher::<CS>::CS_P2].into_boxed_slice();
    mesher.fast_mesh(&voxels, &opaque_mask, &trans_mask);
    spawn_quads(
        &mut commands,
        &mut materials,
        &mut meshes,
        &mesher.quads,
        Vec3::ZERO,
    );
}
