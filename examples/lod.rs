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

use crate::{mesh_utils::spawn_quads, model_utils::natural_terrain};

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
        Transform::from_translation(Vec3::new(0.0, 100.0, 50.0)),
        PointLight {
            range: 500.0,
            intensity: 100_000_000.0,
            ..Default::default()
        },
    ));
    commands.spawn((
        Camera3d::default(),
        Msaa::Sample4,
        Transform::from_translation(Vec3::new(0., CS as f32 * 0.8, CS as f32 * 2.5))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
    let mut mesher = bgm::Mesher::<CS>::new();
    let mut voxels = natural_terrain::<CS>();
    mesher.mesh(&voxels, &BTreeSet::new());
    spawn_quads(
        &mut commands,
        &mut materials,
        &mut meshes,
        &mesher.quads,
        Vec3::new(-(CS as f32) - 5., 0., 0.),
    );
    mesher.clear();
    mesher.mesh_lod(&mut voxels, &BTreeSet::new(), 2);
    spawn_quads(
        &mut commands,
        &mut materials,
        &mut meshes,
        &mesher.quads,
        Vec3::new(5., 0., 0.),
    );
}
