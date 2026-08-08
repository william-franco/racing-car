//! Adereços do circuito: guard-rails, cones, pórtico de largada e bandeiras.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::core::physics::GameLayer;
use crate::core::settings::DisplayQuality;
use crate::core::state::GameState;

use super::mesh::{GROUND_Y, VERGE_WIDTH};
use super::spline::TrackLayout;

/// Distância entre dois postes de guard-rail, em metros.
const RAIL_SPACING: f32 = 10.0;

/// Folga entre a borda do acostamento e o guard-rail.
const RAIL_MARGIN: f32 = 1.5;

const RAIL_HEIGHT: f32 = 0.95;

pub struct PropMaterials {
    pub rail: Handle<StandardMaterial>,
    pub post: Handle<StandardMaterial>,
    pub cone: Handle<StandardMaterial>,
    pub trunk: Handle<StandardMaterial>,
    pub leaves: Handle<StandardMaterial>,
    pub concrete: Handle<StandardMaterial>,
    pub accent: Handle<StandardMaterial>,
    pub flag: Handle<StandardMaterial>,
}

impl PropMaterials {
    pub fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        let mut solid = |color: Color, roughness: f32, metallic: f32| {
            materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: roughness,
                metallic,
                ..default()
            })
        };

        Self {
            rail: solid(Color::srgb(0.72, 0.74, 0.78), 0.35, 0.85),
            post: solid(Color::srgb(0.35, 0.36, 0.39), 0.6, 0.5),
            cone: solid(Color::srgb(0.95, 0.35, 0.1), 0.7, 0.0),
            trunk: solid(Color::srgb(0.31, 0.21, 0.14), 0.9, 0.0),
            leaves: solid(Color::srgb(0.16, 0.42, 0.18), 0.95, 0.0),
            concrete: solid(Color::srgb(0.66, 0.65, 0.62), 0.85, 0.0),
            accent: solid(Color::srgb(0.92, 0.24, 0.16), 0.5, 0.1),
            flag: solid(Color::srgb(0.95, 0.95, 0.95), 0.8, 0.0),
        }
    }
}

/// Guard-rails contínuos dos dois lados, com postes a cada vão.
pub fn spawn_guard_rails(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    props: &PropMaterials,
    layout: &TrackLayout,
) {
    let rail_mesh = meshes.add(Cuboid::new(1.0, 0.34, 1.0));
    let post_mesh = meshes.add(Cuboid::new(0.16, RAIL_HEIGHT, 0.16));

    let steps = (layout.length() / RAIL_SPACING).floor().max(4.0) as usize;
    let step = layout.length() / steps as f32;

    for side in [-1.0f32, 1.0] {
        for i in 0..steps {
            let start = layout.sample_at_distance(i as f32 * step);
            let end = layout.sample_at_distance((i + 1) as f32 * step);

            let lateral = side * (start.half_width + VERGE_WIDTH + RAIL_MARGIN);
            let a = start.center + start.right * lateral;
            let b = end.center + end.right * (side * (end.half_width + VERGE_WIDTH + RAIL_MARGIN));

            let a = a.with_y(GROUND_Y);
            let b = b.with_y(GROUND_Y);
            let span = b - a;
            let length = span.length();
            if length < 0.05 {
                continue;
            }

            let middle = a.midpoint(b) + Vec3::Y * (RAIL_HEIGHT - 0.1);
            let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, span / length);

            // A barra é um cubo esticado no comprimento do vão, o que dá um
            // colisor barato e uma superfície contínua para o carro raspar.
            // O colisor é declarado em tamanho unitário porque o Avian aplica
            // a escala do `Transform` sobre ele — repetir as medidas aqui
            // multiplicaria o vão por ele mesmo.
            commands.spawn((
                DespawnOnExit(GameState::Racing),
                Mesh3d(rail_mesh.clone()),
                MeshMaterial3d(props.rail.clone()),
                Transform::from_translation(middle)
                    .with_rotation(rotation)
                    .with_scale(Vec3::new(0.22, 1.0, length)),
                RigidBody::Static,
                Collider::cuboid(1.0, 0.34, 1.0),
                GameLayer::barrier(),
                Friction::new(0.2),
                Restitution::new(0.1),
            ));

            commands.spawn((
                DespawnOnExit(GameState::Racing),
                Mesh3d(post_mesh.clone()),
                MeshMaterial3d(props.post.clone()),
                Transform::from_translation(a + Vec3::Y * (RAIL_HEIGHT * 0.5)),
            ));
        }
    }
}

/// Cones alinhados nas entradas de curva, apenas decorativos.
pub fn spawn_cones(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    props: &PropMaterials,
    layout: &TrackLayout,
    quality: DisplayQuality,
) {
    let cone = meshes.add(Cone {
        radius: 0.34,
        height: 0.8,
    });

    let spacing = 26.0 / quality.scenery_density().max(0.2);
    let steps = (layout.length() / spacing).floor().max(4.0) as usize;

    for i in 0..steps {
        let sample = layout.sample_at_distance(i as f32 * layout.length() / steps as f32);
        if sample.curvature.abs() < 0.004 {
            continue;
        }

        for side in [-1.0f32, 1.0] {
            let lateral = side * (sample.half_width + 3.2);
            let position = (sample.center + sample.right * lateral).with_y(GROUND_Y + 0.4);
            commands.spawn((
                DespawnOnExit(GameState::Racing),
                Mesh3d(cone.clone()),
                MeshMaterial3d(props.cone.clone()),
                Transform::from_translation(position),
            ));
        }
    }
}

/// Árvores fora do circuito, para dar referência de velocidade.
pub fn spawn_trees(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    props: &PropMaterials,
    layout: &TrackLayout,
    quality: DisplayQuality,
) {
    let trunk = meshes.add(Cylinder::new(0.28, 3.2));
    let leaves = meshes.add(Sphere::new(2.1));

    let spacing = 18.0 / quality.scenery_density().max(0.2);
    let steps = (layout.length() / spacing).floor().max(8.0) as usize;

    for i in 0..steps {
        let distance = i as f32 * layout.length() / steps as f32;
        let sample = layout.sample_at_distance(distance);

        // Uma variação pseudoaleatória estável mantém a floresta irregular
        // sem depender de um gerador com estado.
        let jitter = ((i * 2654435761) % 1000) as f32 / 1000.0;
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        let lateral = side * (sample.half_width + VERGE_WIDTH + 8.0 + jitter * 24.0);
        let base = (sample.center + sample.right * lateral).with_y(GROUND_Y);
        let scale = 0.8 + jitter * 0.9;

        commands.spawn((
            DespawnOnExit(GameState::Racing),
            Mesh3d(trunk.clone()),
            MeshMaterial3d(props.trunk.clone()),
            Transform::from_translation(base + Vec3::Y * 1.6 * scale)
                .with_scale(Vec3::splat(scale)),
        ));
        commands.spawn((
            DespawnOnExit(GameState::Racing),
            Mesh3d(leaves.clone()),
            MeshMaterial3d(props.leaves.clone()),
            Transform::from_translation(base + Vec3::Y * 4.2 * scale)
                .with_scale(Vec3::splat(scale)),
        ));
    }
}

/// Pórtico sobre a linha de chegada, com bandeiras nas laterais.
pub fn spawn_start_gantry(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    props: &PropMaterials,
    layout: &TrackLayout,
) {
    let sample = layout.sample_at_distance(0.0);
    let half_width = sample.half_width + 2.5;
    let height = 8.0;

    let pillar = meshes.add(Cuboid::new(1.0, height, 1.0));
    let beam = meshes.add(Cuboid::new(half_width * 2.0, 1.4, 1.2));
    let banner = meshes.add(Cuboid::new(half_width * 1.5, 0.9, 0.1));
    let pole = meshes.add(Cylinder::new(0.09, 5.0));
    let flag = meshes.add(Cuboid::new(1.6, 1.0, 0.05));

    let center = sample.center.with_y(GROUND_Y);
    let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, sample.tangent);

    commands.spawn((
        DespawnOnExit(GameState::Racing),
        Transform::from_translation(center).with_rotation(rotation),
        Visibility::default(),
        children![
            (
                Mesh3d(pillar.clone()),
                MeshMaterial3d(props.concrete.clone()),
                Transform::from_xyz(-half_width, height * 0.5, 0.0),
            ),
            (
                Mesh3d(pillar),
                MeshMaterial3d(props.concrete.clone()),
                Transform::from_xyz(half_width, height * 0.5, 0.0),
            ),
            (
                Mesh3d(beam),
                MeshMaterial3d(props.concrete.clone()),
                Transform::from_xyz(0.0, height - 0.7, 0.0),
            ),
            (
                Mesh3d(banner),
                MeshMaterial3d(props.accent.clone()),
                Transform::from_xyz(0.0, height - 2.1, 0.45),
            ),
        ],
    ));

    // Mastros de bandeira ladeando o pórtico.
    for index in 0..6 {
        let side = if index % 2 == 0 { -1.0 } else { 1.0 };
        let offset = 12.0 + (index / 2) as f32 * 9.0;
        let position = layout.sample_at_distance(layout.wrap_distance(offset));
        let base = (position.center + position.right * (side * (position.half_width + 14.0)))
            .with_y(GROUND_Y);

        commands.spawn((
            DespawnOnExit(GameState::Racing),
            Mesh3d(pole.clone()),
            MeshMaterial3d(props.post.clone()),
            Transform::from_translation(base + Vec3::Y * 2.5),
        ));
        commands.spawn((
            DespawnOnExit(GameState::Racing),
            Mesh3d(flag.clone()),
            MeshMaterial3d(props.flag.clone()),
            Transform::from_translation(base + Vec3::Y * 4.4 + position.right * (side * 0.8)),
        ));
    }
}

/// Arquibancadas na reta principal.
pub fn spawn_grandstands(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    props: &PropMaterials,
    layout: &TrackLayout,
) {
    const TIERS: usize = 5;
    // Comprida ao longo da pista (Z local) e rasa na profundidade (X local).
    let tier = meshes.add(Cuboid::new(3.2, 1.4, 46.0));

    for (index, distance) in [-70.0f32, -22.0, 26.0].into_iter().enumerate() {
        let sample = layout.sample_at_distance(layout.wrap_distance(distance));
        let side = if index == 1 { 1.0 } else { -1.0 };
        let base = (sample.center
            + sample.right * (side * (sample.half_width + VERGE_WIDTH + 8.0)))
            .with_y(GROUND_Y);
        let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, sample.tangent);

        let mut root = commands.spawn((
            DespawnOnExit(GameState::Racing),
            Transform::from_translation(base).with_rotation(rotation),
            Visibility::default(),
        ));

        root.with_children(|parent| {
            for step in 0..TIERS {
                let height = 1.4 * (step as f32 + 0.5);
                let depth = side * (step as f32 * 2.6);
                parent.spawn((
                    Mesh3d(tier.clone()),
                    MeshMaterial3d(if step % 2 == 0 {
                        props.concrete.clone()
                    } else {
                        props.post.clone()
                    }),
                    Transform::from_xyz(depth, height, 0.0),
                ));
            }
        });
    }
}
