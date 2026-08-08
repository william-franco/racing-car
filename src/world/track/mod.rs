//! Montagem do circuito: traçado, malhas, colisores e adereços.

pub mod mesh;
pub mod props;
pub mod spline;

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::core::physics::GameLayer;
use crate::core::settings::DisplayQuality;
use crate::core::state::{GameState, SurfaceGrip};
use crate::world::textures;

pub use spline::TrackLayout;

/// Aderência de cada tipo de superfície, usada pelo modelo de pneu.
pub const GRIP_ASPHALT: f32 = 1.0;
pub const GRIP_VERGE: f32 = 0.5;
pub const GRIP_TERRAIN: f32 = 0.32;

pub struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        // O traçado é puro cálculo e não depende de assets, então já fica
        // disponível para o minimapa do menu.
        app.insert_resource(TrackLayout::circuit())
            .add_systems(OnEnter(GameState::Racing), spawn_track);
    }
}

fn spawn_track(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    layout: Res<TrackLayout>,
    quality: Res<DisplayQuality>,
) {
    let asphalt_texture = images.add(textures::asphalt());
    let gravel_texture = images.add(textures::gravel());

    let asphalt_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.62, 0.66),
        base_color_texture: Some(asphalt_texture),
        perceptual_roughness: 0.92,
        ..default()
    });
    let verge_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.78, 0.75, 0.68),
        base_color_texture: Some(gravel_texture),
        perceptual_roughness: 1.0,
        ..default()
    });
    let paint_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.95),
        perceptual_roughness: 0.6,
        ..default()
    });
    let kerb_red = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.15, 0.12),
        perceptual_roughness: 0.7,
        ..default()
    });
    let kerb_white = materials.add(StandardMaterial {
        base_color: Color::srgb(0.93, 0.93, 0.93),
        perceptual_roughness: 0.7,
        ..default()
    });
    let dark_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.06, 0.06, 0.07),
        perceptual_roughness: 0.8,
        ..default()
    });

    // Asfalto: a única superfície com colisor de malha exata, já que é onde a
    // suspensão precisa de precisão.
    let asphalt_mesh = mesh::asphalt(&layout);
    let asphalt_collider = Collider::trimesh_from_mesh(&asphalt_mesh)
        .expect("a malha do asfalto sempre tem triângulos");
    commands.spawn((
        DespawnOnExit(GameState::Racing),
        Mesh3d(meshes.add(asphalt_mesh)),
        MeshMaterial3d(asphalt_material),
        Transform::default(),
        RigidBody::Static,
        asphalt_collider,
        GameLayer::ground(),
        SurfaceGrip(GRIP_ASPHALT),
        Friction::new(1.0),
    ));

    for side in [-1.0f32, 1.0] {
        let verge_mesh = mesh::verge(&layout, side);
        let verge_collider = Collider::trimesh_from_mesh(&verge_mesh)
            .expect("a malha do acostamento sempre tem triângulos");
        commands.spawn((
            DespawnOnExit(GameState::Racing),
            Mesh3d(meshes.add(verge_mesh)),
            MeshMaterial3d(verge_material.clone()),
            Transform::default(),
            RigidBody::Static,
            verge_collider,
            GameLayer::ground(),
            SurfaceGrip(GRIP_VERGE),
            Friction::new(0.7),
        ));

        commands.spawn((
            DespawnOnExit(GameState::Racing),
            Mesh3d(meshes.add(mesh::edge_line(&layout, side))),
            MeshMaterial3d(paint_material.clone()),
            Transform::default(),
        ));

        let (red, white) = mesh::kerbs(&layout, side);
        spawn_decoration(&mut commands, &mut meshes, red, kerb_red.clone());
        spawn_decoration(&mut commands, &mut meshes, white, kerb_white.clone());
    }

    let (dark, light) = mesh::start_line(&layout);
    spawn_decoration(&mut commands, &mut meshes, dark, dark_material);
    spawn_decoration(&mut commands, &mut meshes, light, paint_material);

    let props = props::PropMaterials::new(&mut materials);
    props::spawn_guard_rails(&mut commands, &mut meshes, &props, &layout);
    props::spawn_cones(&mut commands, &mut meshes, &props, &layout, *quality);
    props::spawn_trees(&mut commands, &mut meshes, &props, &layout, *quality);
    props::spawn_start_gantry(&mut commands, &mut meshes, &props, &layout);
    props::spawn_grandstands(&mut commands, &mut meshes, &props, &layout);
}

/// Malhas puramente visuais, sem colisor. Malhas vazias são descartadas.
fn spawn_decoration(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mesh: Mesh,
    material: Handle<StandardMaterial>,
) {
    if mesh.count_vertices() == 0 {
        return;
    }
    commands.spawn((
        DespawnOnExit(GameState::Racing),
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material),
        Transform::default(),
    ));
}
