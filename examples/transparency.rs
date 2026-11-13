use std::collections::BTreeSet;
#[path = "../utils/mesh_utils.rs"]
mod mesh_utils;
#[path = "../utils/model_utils.rs"]
mod model_utils;
use bevy::{
    prelude::*,
    render::{
        RenderPlugin,
        settings::{RenderCreation, WgpuSettings},
    },
};
use binary_greedy_meshing as bgm;

use crate::{
    mesh_utils::spawn_quads,
    model_utils::{VoxelType, half_transparent_sphere},
};

const CS: usize = 62;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins.set(RenderPlugin {
            render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                ..Default::default()
            })),
            ..default()
        }),))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn((
        Transform::from_translation(Vec3::new(50.0, 100.0, 50.0)),
        PointLight {
            range: 500.0,
            intensity: 100_000_000.0,
            ..Default::default()
        },
    ));
    commands.insert_resource(GlobalAmbientLight {
        brightness: 500.0,
        ..Default::default()
    });
    let center = CS as f32 / 2.0;
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(100.0, 50.0, 80.0))
            .looking_at(Vec3::splat(center), Vec3::Y),
    ));
    let voxels = half_transparent_sphere::<CS>(CS / 2);
    let mut mesher = bgm::Mesher::<CS>::new();
    let mut transparent_blocks = BTreeSet::new();
    transparent_blocks.insert(VoxelType::Glass as u16);
    let opaque_mask = bgm::compute_opaque_mask::<CS>(&voxels, &transparent_blocks);
    let trans_mask = bgm::compute_transparent_mask::<CS>(&voxels, &transparent_blocks);
    mesher.fast_mesh(&voxels, &opaque_mask, &trans_mask);
    spawn_quads(
        &mut commands,
        &mut materials,
        &mut meshes,
        &mesher.quads,
        Vec3::ZERO,
    );
}
