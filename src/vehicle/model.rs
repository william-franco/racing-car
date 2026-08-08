//! Montagem procedural do carro: carroceria, aerofólio, faróis e rodas.
//!
//! Nada aqui depende de arquivos de modelo — o veículo inteiro é composto de
//! primitivas do Bevy, o que mantém o projeto autocontido e deixa a paleta de
//! cada carro configurável em tempo de execução.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::core::physics::GameLayer;
use crate::core::state::GameState;

use super::physics::{CarChassis, CarConfig, DriveInput, EngineState, Wheel, WheelState};

/// Dimensões do chassi usadas tanto pelo colisor quanto pela carroceria.
pub const CAR_LENGTH: f32 = 4.3;
pub const CAR_WIDTH: f32 = 1.9;
pub const CAR_HEIGHT: f32 = 0.72;

/// Meia-distância entre eixos e entre rodas.
const WHEELBASE: f32 = 1.42;
const TRACK_WIDTH: f32 = 0.86;

/// Altura do ponto de fixação da suspensão, relativa ao centro do chassi.
const AXLE_HEIGHT: f32 = 0.05;

/// Paleta de um carro.
#[derive(Clone, Copy, Debug)]
pub struct CarPaint {
    pub body: Color,
    pub accent: Color,
    pub glass: Color,
}

impl CarPaint {
    pub const PLAYER: Self = Self {
        body: Color::srgb(0.85, 0.16, 0.12),
        accent: Color::srgb(0.96, 0.96, 0.96),
        glass: Color::srgb(0.09, 0.14, 0.2),
    };

    /// Paletas dos adversários, escolhidas por índice.
    pub fn opponent(index: usize) -> Self {
        const BODIES: [Color; 6] = [
            Color::srgb(0.12, 0.36, 0.85),
            Color::srgb(0.95, 0.78, 0.12),
            Color::srgb(0.14, 0.68, 0.36),
            Color::srgb(0.62, 0.16, 0.78),
            Color::srgb(0.95, 0.45, 0.08),
            Color::srgb(0.1, 0.72, 0.76),
        ];

        Self {
            body: BODIES[index % BODIES.len()],
            accent: Color::srgb(0.14, 0.14, 0.16),
            glass: Color::srgb(0.09, 0.14, 0.2),
        }
    }
}

/// Malhas compartilhadas por todos os carros, criadas uma única vez.
pub struct CarMeshes {
    body: Handle<Mesh>,
    hood: Handle<Mesh>,
    cabin: Handle<Mesh>,
    side_skirt: Handle<Mesh>,
    diffuser: Handle<Mesh>,
    wing: Handle<Mesh>,
    wing_post: Handle<Mesh>,
    headlight: Handle<Mesh>,
    taillight: Handle<Mesh>,
    exhaust: Handle<Mesh>,
    mirror: Handle<Mesh>,
    tyre: Handle<Mesh>,
    rim: Handle<Mesh>,
    spoke: Handle<Mesh>,
}

impl CarMeshes {
    pub fn new(meshes: &mut Assets<Mesh>) -> Self {
        Self {
            body: meshes.add(Cuboid::new(CAR_WIDTH, CAR_HEIGHT, CAR_LENGTH)),
            hood: meshes.add(Cuboid::new(CAR_WIDTH * 0.92, 0.22, 1.35)),
            cabin: meshes.add(Cuboid::new(CAR_WIDTH * 0.78, 0.52, 1.75)),
            side_skirt: meshes.add(Cuboid::new(0.16, 0.24, CAR_LENGTH * 0.62)),
            diffuser: meshes.add(Cuboid::new(CAR_WIDTH * 0.88, 0.2, 0.5)),
            wing: meshes.add(Cuboid::new(CAR_WIDTH * 0.86, 0.08, 0.42)),
            wing_post: meshes.add(Cuboid::new(0.08, 0.34, 0.14)),
            headlight: meshes.add(Cuboid::new(0.44, 0.14, 0.1)),
            taillight: meshes.add(Cuboid::new(0.38, 0.12, 0.08)),
            exhaust: meshes.add(Cylinder::new(0.07, 0.28)),
            mirror: meshes.add(Cuboid::new(0.2, 0.1, 0.08)),
            tyre: meshes.add(Cylinder::new(0.36, 0.28)),
            rim: meshes.add(Cylinder::new(0.23, 0.3)),
            spoke: meshes.add(Cuboid::new(0.06, 0.31, 0.42)),
        }
    }
}

struct CarMaterials {
    body: Handle<StandardMaterial>,
    accent: Handle<StandardMaterial>,
    glass: Handle<StandardMaterial>,
    rubber: Handle<StandardMaterial>,
    metal: Handle<StandardMaterial>,
    headlight: Handle<StandardMaterial>,
    taillight: Handle<StandardMaterial>,
}

impl CarMaterials {
    fn new(materials: &mut Assets<StandardMaterial>, paint: CarPaint) -> Self {
        Self {
            body: materials.add(StandardMaterial {
                base_color: paint.body,
                perceptual_roughness: 0.32,
                metallic: 0.45,
                ..default()
            }),
            accent: materials.add(StandardMaterial {
                base_color: paint.accent,
                perceptual_roughness: 0.4,
                metallic: 0.2,
                ..default()
            }),
            glass: materials.add(StandardMaterial {
                base_color: paint.glass,
                perceptual_roughness: 0.08,
                metallic: 0.1,
                reflectance: 0.85,
                ..default()
            }),
            rubber: materials.add(StandardMaterial {
                base_color: Color::srgb(0.055, 0.055, 0.06),
                perceptual_roughness: 0.95,
                ..default()
            }),
            metal: materials.add(StandardMaterial {
                base_color: Color::srgb(0.76, 0.77, 0.8),
                perceptual_roughness: 0.25,
                metallic: 0.95,
                ..default()
            }),
            headlight: materials.add(StandardMaterial {
                base_color: Color::srgb(0.95, 0.95, 0.82),
                emissive: LinearRgba::rgb(6.0, 5.6, 3.6),
                ..default()
            }),
            taillight: materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.06, 0.06),
                emissive: LinearRgba::rgb(4.5, 0.3, 0.2),
                ..default()
            }),
        }
    }
}

/// Cria um carro completo e devolve a entidade do chassi.
pub fn spawn_car(
    commands: &mut Commands,
    car_meshes: &CarMeshes,
    materials: &mut Assets<StandardMaterial>,
    config: CarConfig,
    paint: CarPaint,
    transform: Transform,
) -> Entity {
    let palette = CarMaterials::new(materials, paint);

    // As rodas nascem primeiro para que o chassi possa guardar suas entidades.
    let wheels: [Entity; 4] = std::array::from_fn(|index| {
        let front = index < 2;
        let side = if index % 2 == 0 { -1.0 } else { 1.0 };
        let anchor = Vec3::new(
            side * TRACK_WIDTH,
            AXLE_HEIGHT,
            if front { -WHEELBASE } else { WHEELBASE },
        );

        commands
            .spawn((
                Wheel {
                    index,
                    anchor,
                    radius: config.wheel_radius,
                    steered: front,
                    // Tração traseira: sobresteer fácil e drift acessível.
                    powered: !front,
                },
                WheelState::default(),
                Transform::from_translation(anchor - Vec3::Y * config.suspension_rest),
                Visibility::default(),
                children![
                    (
                        Mesh3d(car_meshes.tyre.clone()),
                        MeshMaterial3d(palette.rubber.clone()),
                        // O cilindro nasce em pé; deitá-lo alinha o eixo com X.
                        Transform::from_rotation(Quat::from_rotation_z(
                            std::f32::consts::FRAC_PI_2
                        )),
                    ),
                    (
                        Mesh3d(car_meshes.rim.clone()),
                        MeshMaterial3d(palette.metal.clone()),
                        Transform::from_rotation(Quat::from_rotation_z(
                            std::f32::consts::FRAC_PI_2
                        )),
                    ),
                    (
                        Mesh3d(car_meshes.spoke.clone()),
                        MeshMaterial3d(palette.accent.clone()),
                        Transform::default(),
                    ),
                    (
                        Mesh3d(car_meshes.spoke.clone()),
                        MeshMaterial3d(palette.accent.clone()),
                        Transform::from_rotation(Quat::from_rotation_x(
                            std::f32::consts::FRAC_PI_2
                        )),
                    ),
                ],
            ))
            .id()
    });

    let body = (
        // Corpo rígido: o colisor é um cuboide simples, já que o contato com o
        // chão é resolvido pelos raycasts das rodas.
        RigidBody::Dynamic,
        Collider::cuboid(CAR_WIDTH, CAR_HEIGHT, CAR_LENGTH),
        // A densidade é derivada da massa desejada para que massa e inércia
        // angular continuem coerentes entre si.
        ColliderDensity(config.mass / (CAR_WIDTH * CAR_HEIGHT * CAR_LENGTH)),
        // Centro de massa baixo e levemente atrás: estável e com tendência a
        // girar a traseira quando o piloto exagera.
        CenterOfMass::new(0.0, -0.28, 0.12),
        NoAutoCenterOfMass,
        LinearDamping(0.08),
        AngularDamping(1.4),
        Friction::new(0.35),
        Restitution::new(0.08),
        GameLayer::car(),
        CollisionEventsEnabled,
        TransformInterpolation,
    );

    let chassis = commands
        .spawn((
            DespawnOnExit(GameState::Racing),
            transform,
            Visibility::default(),
            body,
            CarChassis { wheels },
            config,
            DriveInput::default(),
            EngineState::default(),
        ))
        .id();

    commands.entity(chassis).add_children(&wheels);
    spawn_body(commands, chassis, car_meshes, &palette);
    chassis
}

/// Todos os volumes visuais da carroceria, presos ao chassi.
fn spawn_body(
    commands: &mut Commands,
    chassis: Entity,
    meshes: &CarMeshes,
    palette: &CarMaterials,
) {
    let nose = -CAR_LENGTH * 0.5;
    let tail = CAR_LENGTH * 0.5;

    commands.entity(chassis).with_children(|parent| {
        parent.spawn((
            Mesh3d(meshes.body.clone()),
            MeshMaterial3d(palette.body.clone()),
            Transform::default(),
        ));

        // Capô inclinado para a frente.
        parent.spawn((
            Mesh3d(meshes.hood.clone()),
            MeshMaterial3d(palette.body.clone()),
            Transform::from_xyz(0.0, CAR_HEIGHT * 0.42, nose + 0.85)
                .with_rotation(Quat::from_rotation_x(-0.07)),
        ));

        // Cabine envidraçada, recuada em relação ao capô.
        parent.spawn((
            Mesh3d(meshes.cabin.clone()),
            MeshMaterial3d(palette.glass.clone()),
            Transform::from_xyz(0.0, CAR_HEIGHT * 0.62, 0.16),
        ));

        parent.spawn((
            Mesh3d(meshes.diffuser.clone()),
            MeshMaterial3d(palette.accent.clone()),
            Transform::from_xyz(0.0, -CAR_HEIGHT * 0.32, tail - 0.22),
        ));

        for side in [-1.0f32, 1.0] {
            parent.spawn((
                Mesh3d(meshes.side_skirt.clone()),
                MeshMaterial3d(palette.accent.clone()),
                Transform::from_xyz(side * CAR_WIDTH * 0.52, -CAR_HEIGHT * 0.3, 0.0),
            ));

            parent.spawn((
                Mesh3d(meshes.mirror.clone()),
                MeshMaterial3d(palette.accent.clone()),
                Transform::from_xyz(side * CAR_WIDTH * 0.56, CAR_HEIGHT * 0.55, -0.55),
            ));

            parent.spawn((
                Mesh3d(meshes.wing_post.clone()),
                MeshMaterial3d(palette.accent.clone()),
                Transform::from_xyz(side * CAR_WIDTH * 0.34, CAR_HEIGHT * 0.68, tail - 0.28),
            ));

            parent.spawn((
                Mesh3d(meshes.headlight.clone()),
                MeshMaterial3d(palette.headlight.clone()),
                Transform::from_xyz(side * CAR_WIDTH * 0.3, CAR_HEIGHT * 0.12, nose + 0.02),
            ));

            parent.spawn((
                Mesh3d(meshes.taillight.clone()),
                MeshMaterial3d(palette.taillight.clone()),
                Transform::from_xyz(side * CAR_WIDTH * 0.32, CAR_HEIGHT * 0.16, tail - 0.02),
            ));

            parent.spawn((
                Mesh3d(meshes.exhaust.clone()),
                MeshMaterial3d(palette.metal.clone()),
                Transform::from_xyz(side * 0.34, -CAR_HEIGHT * 0.24, tail + 0.06)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            ));
        }

        // Aerofólio apoiado nas duas hastes.
        parent.spawn((
            Mesh3d(meshes.wing.clone()),
            MeshMaterial3d(palette.accent.clone()),
            Transform::from_xyz(0.0, CAR_HEIGHT * 0.85, tail - 0.28)
                .with_rotation(Quat::from_rotation_x(0.14)),
        ));
    });
}
