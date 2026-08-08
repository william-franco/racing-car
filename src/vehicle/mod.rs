//! Os carros: modelo 3D, física, comandos do jogador e pilotos da IA.

pub mod ai;
pub mod input;
pub mod model;
pub mod physics;

use avian3d::prelude::*;
use bevy::prelude::*;
use rand::SeedableRng;
use rand::rngs::SmallRng;

use crate::core::settings::OpponentCount;
use crate::core::state::{AiCar, GameState, PlayerCar};
use crate::world::track::TrackLayout;

use ai::AiDriver;
use input::RespawnRequested;
use model::{CarMeshes, CarPaint, spawn_car};
use physics::{CarConfig, DriveInput, EngineState};

/// Altura do centro do chassi ao nascer, com a suspensão a meio curso.
const RIDE_HEIGHT: f32 = 0.68;

/// Espaçamento entre fileiras do grid, em metros.
const GRID_ROW_SPACING: f32 = 9.0;

/// Deslocamento lateral de cada coluna do grid.
const GRID_COLUMN_OFFSET: f32 = 3.0;

/// Identifica um carro dentro da corrida.
#[derive(Component, Clone, Debug)]
pub struct CarIdentity {
    pub name: String,
    /// Posição de largada, contada a partir da pole.
    pub grid_slot: usize,
    pub is_player: bool,
}

pub struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            physics::VehiclePhysicsPlugin,
            input::VehicleInputPlugin,
            ai::VehicleAiPlugin,
        ))
        .add_systems(OnEnter(GameState::Racing), spawn_grid)
        .add_systems(
            Update,
            (handle_respawn, rescue_stuck_cars).run_if(in_state(GameState::Racing)),
        );
    }
}

/// Pose de largada de um lugar do grid.
fn grid_pose(layout: &TrackLayout, slot: usize) -> Transform {
    let row = slot / 2;
    let column = slot % 2;

    // A pole fica logo atrás da linha de chegada, e as fileiras recuam.
    let distance = layout.wrap_distance(-10.0 - row as f32 * GRID_ROW_SPACING);
    let lateral = if column == 0 {
        -GRID_COLUMN_OFFSET
    } else {
        GRID_COLUMN_OFFSET
    };

    let sample = layout.sample_at_distance(distance);
    Transform::from_translation(sample.edge(lateral) + sample.up * RIDE_HEIGHT)
        .with_rotation(sample.orientation())
}

fn spawn_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    layout: Res<TrackLayout>,
    opponents: Res<OpponentCount>,
) {
    let car_meshes = CarMeshes::new(&mut meshes);
    // Semente fixa: o mesmo grid em toda partida, o que torna as corridas
    // comparáveis entre si.
    let mut rng = SmallRng::seed_from_u64(0x5EED_C0DE);

    let total = opponents.0 as usize;
    // O jogador larga em último para ter algo a fazer nas primeiras voltas.
    let player_slot = total;

    for slot in 0..total {
        let entity = spawn_car(
            &mut commands,
            &car_meshes,
            &mut materials,
            opponent_config(slot),
            CarPaint::opponent(slot),
            grid_pose(&layout, slot),
        );

        commands.entity(entity).insert((
            AiCar,
            AiDriver::new(slot, &mut rng),
            CarIdentity {
                name: format!("CPU {:02}", slot + 1),
                grid_slot: slot,
                is_player: false,
            },
        ));
    }

    let player = spawn_car(
        &mut commands,
        &car_meshes,
        &mut materials,
        CarConfig::default(),
        CarPaint::PLAYER,
        grid_pose(&layout, player_slot),
    );

    commands.entity(player).insert((
        PlayerCar,
        CarIdentity {
            name: "Você".into(),
            grid_slot: player_slot,
            is_player: true,
        },
    ));
}

/// Adversários ligeiramente mais fracos que o jogador, e entre si.
fn opponent_config(slot: usize) -> CarConfig {
    let handicap = 1.0 - slot as f32 * 0.006;
    let base = CarConfig::default();

    CarConfig {
        engine_force: base.engine_force * (0.94 * handicap),
        top_speed: base.top_speed * (0.97 * handicap),
        ..base
    }
}

/// Estado de pose de um corpo do Avian, escrito diretamente para o motor não
/// sobrescrever a correção no próximo passo.
type CarPose<'a> = (
    Mut<'a, Position>,
    Mut<'a, Rotation>,
    Mut<'a, LinearVelocity>,
    Mut<'a, AngularVelocity>,
);

/// Recoloca um carro no eixo da pista, apontado para a frente e parado.
fn place_on_track(layout: &TrackLayout, pose: &mut CarPose) {
    let (position, rotation, linear, angular) = pose;

    let location = layout.locate(position.0, None);
    let sample = layout.sample_at_distance(location.distance);

    position.0 = sample.center + sample.up * RIDE_HEIGHT;
    rotation.0 = sample.orientation();
    linear.0 = Vec3::ZERO;
    angular.0 = Vec3::ZERO;
}

#[allow(clippy::type_complexity)]
fn handle_respawn(
    mut requests: MessageReader<RespawnRequested>,
    layout: Res<TrackLayout>,
    mut player: Query<
        (
            &mut Position,
            &mut Rotation,
            &mut LinearVelocity,
            &mut AngularVelocity,
            &mut EngineState,
            &mut DriveInput,
        ),
        With<PlayerCar>,
    >,
) {
    if requests.read().count() == 0 {
        return;
    }

    let Ok((position, rotation, linear, angular, mut engine, mut input)) = player.single_mut()
    else {
        return;
    };

    place_on_track(&layout, &mut (position, rotation, linear, angular));
    *engine = EngineState::default();
    *input = DriveInput::default();
}

/// Carro capotado, longe da pista ou encalhado volta sozinho depois de um tempo.
#[derive(Component, Default)]
struct StuckTimer(f32);

/// Abaixo desta velocidade, com o acelerador no fundo, o carro está encalhado.
/// Nenhuma curva do circuito é lenta o bastante para se andar assim de propósito.
const STALLED_SPEED: f32 = 2.5;

#[allow(clippy::type_complexity)]
fn rescue_stuck_cars(
    time: Res<Time>,
    layout: Res<TrackLayout>,
    mut commands: Commands,
    mut cars: Query<
        (
            Entity,
            &mut Position,
            &mut Rotation,
            &mut LinearVelocity,
            &mut AngularVelocity,
            &DriveInput,
            Option<&mut StuckTimer>,
        ),
        With<CarIdentity>,
    >,
) {
    for (entity, position, rotation, linear, angular, input, timer) in &mut cars {
        let upright = (rotation.0 * Vec3::Y).dot(Vec3::Y);
        let location = layout.locate(position.0, None);
        // Acelerar sem sair do lugar é sinal de carro preso no guard-rail ou
        // enroscado em outro carro; sem isso ele ficaria ali a corrida inteira.
        let stalled = input.throttle > 0.3 && linear.length() < STALLED_SPEED;
        let in_trouble = upright < 0.25 || location.off_track() > 22.0 || stalled;

        let elapsed = match timer {
            Some(mut timer) => {
                timer.0 = if in_trouble {
                    timer.0 + time.delta_secs()
                } else {
                    0.0
                };
                timer.0
            }
            None => {
                commands.entity(entity).insert(StuckTimer::default());
                0.0
            }
        };

        if elapsed > 3.0 {
            place_on_track(&layout, &mut (position, rotation, linear, angular));
            commands.entity(entity).insert(StuckTimer::default());
        }
    }
}
