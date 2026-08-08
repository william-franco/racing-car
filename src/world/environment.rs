//! Céu, iluminação e terreno em volta do circuito.

use avian3d::prelude::*;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use crate::core::physics::GameLayer;
use crate::core::settings::DisplayQuality;
use crate::core::state::{GameState, SurfaceGrip};
use crate::world::textures;
use crate::world::track::{GRIP_TERRAIN, mesh::GROUND_Y};

/// Raio da esfera invertida que faz as vezes de céu.
const SKY_RADIUS: f32 = 2500.0;

/// Lado do plano de terreno.
const TERRAIN_SIZE: f32 = 5000.0;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Racing), spawn_environment);
    }
}

fn spawn_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    quality: Res<DisplayQuality>,
) {
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.75, 0.82, 1.0),
        brightness: 260.0,
        ..default()
    });

    commands.spawn((
        DespawnOnExit(GameState::Racing),
        DirectionalLight {
            illuminance: 11_000.0,
            shadow_maps_enabled: quality.shadows_enabled(),
            ..default()
        },
        Transform::default().looking_to(Vec3::new(-0.55, -0.72, -0.42), Vec3::Y),
    ));

    // Céu: esfera com escala negativa, então enxergamos a face interna.
    commands.spawn((
        DespawnOnExit(GameState::Racing),
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.33, 0.6, 0.95),
            unlit: true,
            ..default()
        })),
        Transform::from_scale(Vec3::splat(-SKY_RADIUS)),
        NotShadowCaster,
    ));

    let grass_texture = images.add(textures::grass());
    let mut terrain: Mesh = Plane3d::default().mesh().size(1.0, 1.0).into();
    // A UV acompanha o tamanho do plano para a grama não virar um borrão.
    let repeats = TERRAIN_SIZE / 6.0;
    terrain.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![
            [repeats, 0.0],
            [0.0, 0.0],
            [0.0, repeats],
            [repeats, repeats],
        ],
    );

    commands.spawn((
        DespawnOnExit(GameState::Racing),
        Mesh3d(meshes.add(terrain)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.55, 0.72, 0.45),
            base_color_texture: Some(grass_texture),
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::from_xyz(0.0, GROUND_Y, 0.0).with_scale(Vec3::splat(TERRAIN_SIZE)),
    ));

    // O colisor do terreno é um cuboide grosso logo abaixo da grama: bem mais
    // barato que uma malha e impossível de atravessar em velocidade alta.
    commands.spawn((
        DespawnOnExit(GameState::Racing),
        Transform::from_xyz(0.0, GROUND_Y - 2.0, 0.0),
        RigidBody::Static,
        Collider::cuboid(TERRAIN_SIZE, 4.0, TERRAIN_SIZE),
        GameLayer::ground(),
        SurfaceGrip(GRIP_TERRAIN),
        Friction::new(0.6),
    ));
}
