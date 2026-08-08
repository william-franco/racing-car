//! Fluxo da corrida: grid, contagem regressiva, voltas, posições e bandeirada.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::core::settings::LapCount;
use crate::core::state::{GameState, PlayerCar, RacePhase};
use crate::vehicle::CarIdentity;
use crate::world::track::TrackLayout;

use super::checkpoint::{CHECKPOINT_COUNT, Checkpoint};
use super::records::record_lap;

/// Segundos de semáforo antes da largada.
const COUNTDOWN_SECONDS: f32 = 4.0;

/// Parâmetros fixos da corrida em andamento.
#[derive(Resource, Debug, Clone, Copy)]
pub struct RaceConfig {
    pub total_laps: u32,
    pub entrants: usize,
}

/// Cronômetro da corrida, contado a partir da largada.
#[derive(Resource, Debug, Default)]
pub struct RaceClock {
    pub elapsed: f32,
    /// Tempo restante de contagem regressiva; zero depois da largada.
    pub countdown: f32,
}

impl RaceClock {
    /// Número mostrado no semáforo, ou `None` quando já é "VAI!".
    pub fn countdown_number(&self) -> Option<u32> {
        (self.countdown > 0.0).then(|| self.countdown.ceil() as u32)
    }
}

/// Progresso de um carro na corrida.
#[derive(Component, Debug, Clone)]
pub struct RaceProgress {
    pub lap: u32,
    /// Próximo checkpoint que precisa ser cruzado.
    pub next_checkpoint: usize,
    /// A volta só começa a contar quando o carro cruza a linha pela 1ª vez.
    pub started: bool,
    pub lap_start: f32,
    pub last_lap: Option<f32>,
    pub best_lap: Option<f32>,
    pub finished: bool,
    pub finish_time: Option<f32>,
    /// Colocação atual, começando em 1.
    pub position: usize,
    /// Distância total percorrida, usada para ordenar o pelotão.
    pub distance: f32,
    /// Índice da amostra mais próxima, para acelerar a busca no traçado.
    pub track_hint: Option<usize>,
}

impl Default for RaceProgress {
    fn default() -> Self {
        Self {
            lap: 0,
            next_checkpoint: 0,
            started: false,
            lap_start: 0.0,
            last_lap: None,
            best_lap: None,
            finished: false,
            finish_time: None,
            position: 1,
            distance: 0.0,
            track_hint: None,
        }
    }
}

pub struct RaceFlowPlugin;

impl Plugin for RaceFlowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RaceClock>()
            .add_systems(OnEnter(GameState::Racing), start_countdown)
            .add_systems(
                Update,
                (
                    attach_progress,
                    tick_countdown.run_if(in_state(RacePhase::Countdown)),
                    (advance_clock, track_checkpoints, update_standings)
                        .chain()
                        .run_if(in_state(RacePhase::Green)),
                )
                    .run_if(in_state(GameState::Racing)),
            );
    }
}

fn start_countdown(
    mut commands: Commands,
    mut clock: ResMut<RaceClock>,
    mut phase: ResMut<NextState<RacePhase>>,
    laps: Res<LapCount>,
    cars: Query<(), With<CarIdentity>>,
) {
    *clock = RaceClock {
        elapsed: 0.0,
        countdown: COUNTDOWN_SECONDS,
    };
    commands.insert_resource(RaceConfig {
        total_laps: laps.0,
        entrants: cars.iter().count(),
    });
    phase.set(RacePhase::Countdown);
}

/// Os carros nascem no plugin de veículos; a corrida só anexa o progresso.
fn attach_progress(
    mut commands: Commands,
    mut config: Option<ResMut<RaceConfig>>,
    cars: Query<Entity, (With<CarIdentity>, Without<RaceProgress>)>,
) {
    let mut added = 0;
    for entity in &cars {
        commands.entity(entity).insert(RaceProgress::default());
        added += 1;
    }

    if added > 0
        && let Some(config) = config.as_mut()
    {
        config.entrants += added;
    }
}

fn tick_countdown(
    time: Res<Time>,
    mut clock: ResMut<RaceClock>,
    mut phase: ResMut<NextState<RacePhase>>,
) {
    clock.countdown -= time.delta_secs();
    if clock.countdown <= 0.0 {
        clock.countdown = 0.0;
        phase.set(RacePhase::Green);
    }
}

fn advance_clock(time: Res<Time>, mut clock: ResMut<RaceClock>) {
    clock.elapsed += time.delta_secs();
}

#[allow(clippy::too_many_arguments)]
fn track_checkpoints(
    mut collisions: MessageReader<CollisionStart>,
    clock: Res<RaceClock>,
    config: Res<RaceConfig>,
    checkpoints: Query<&Checkpoint>,
    mut cars: Query<(&mut RaceProgress, Has<PlayerCar>)>,
    mut phase: ResMut<NextState<RacePhase>>,
    mut commands: Commands,
) {
    for event in collisions.read() {
        // Um dos lados é o portal e o outro é o carro; a ordem não é garantida.
        let Some((checkpoint, car)) = pair(&checkpoints, event) else {
            continue;
        };
        let Ok((mut progress, is_player)) = cars.get_mut(car) else {
            continue;
        };

        if progress.finished || checkpoint.index != progress.next_checkpoint {
            continue;
        }

        if checkpoint.index == 0 {
            if progress.started {
                let lap_time = clock.elapsed - progress.lap_start;
                progress.lap += 1;
                progress.last_lap = Some(lap_time);
                progress.best_lap = Some(match progress.best_lap {
                    Some(best) => best.min(lap_time),
                    None => lap_time,
                });

                if is_player {
                    commands.run_system_cached_with(record_lap, lap_time);
                }

                if progress.lap >= config.total_laps {
                    progress.finished = true;
                    progress.finish_time = Some(clock.elapsed);
                    if is_player {
                        phase.set(RacePhase::Finished);
                    }
                }
            } else {
                progress.started = true;
            }

            progress.lap_start = clock.elapsed;
        }

        progress.next_checkpoint = (checkpoint.index + 1) % CHECKPOINT_COUNT;
    }
}

/// Descobre qual lado da colisão é o checkpoint e qual é o corpo do carro.
fn pair(checkpoints: &Query<&Checkpoint>, event: &CollisionStart) -> Option<(Checkpoint, Entity)> {
    if let Ok(checkpoint) = checkpoints.get(event.collider1) {
        return Some((*checkpoint, event.body2?));
    }
    if let Ok(checkpoint) = checkpoints.get(event.collider2) {
        return Some((*checkpoint, event.body1?));
    }
    None
}

/// Recalcula a distância percorrida por cada carro e ordena o pelotão.
fn update_standings(
    layout: Res<TrackLayout>,
    mut cars: Query<(Entity, &Position, &mut RaceProgress)>,
) {
    let mut ranking: Vec<(Entity, f32)> = Vec::new();

    for (entity, position, mut progress) in &mut cars {
        let location = layout.locate(position.0, progress.track_hint);
        progress.track_hint = Some(location.index);

        // A volta só passa a contar depois da primeira passagem pela linha, e
        // antes disso o carro está atrás dela — daí a distância negativa.
        let lap_distance = if progress.started {
            location.distance
        } else {
            location.distance - layout.length()
        };
        progress.distance = progress.lap as f32 * layout.length() + lap_distance;

        // Quem terminou fica congelado na frente, ordenado pelo tempo.
        let score = match progress.finish_time {
            Some(time) => f32::MAX - time,
            None => progress.distance,
        };
        ranking.push((entity, score));
    }

    ranking.sort_by(|a, b| b.1.total_cmp(&a.1));

    for (place, (entity, _)) in ranking.iter().enumerate() {
        if let Ok((_, _, mut progress)) = cars.get_mut(*entity) {
            progress.position = place + 1;
        }
    }
}
