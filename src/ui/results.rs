//! Tela de resultados, exibida sobre a pista quando a corrida acaba.

use bevy::prelude::*;

use crate::core::state::{GameState, RacePhase};
use crate::race::records::format_lap_time;
use crate::race::{RaceConfig, RaceProgress};
use crate::vehicle::CarIdentity;

use super::theme;

#[derive(Component, Clone, Copy)]
enum ResultsAction {
    Restart,
    BackToMenu,
}

/// Marca que o jogador pediu outra corrida. A corrida precisa ser desmontada
/// antes de ser recriada, então o pedido sobrevive um quadro no menu.
#[derive(Resource)]
struct PendingRestart;

pub struct ResultsPlugin;

impl Plugin for ResultsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(RacePhase::Finished), show_results)
            .add_systems(Update, results_action.run_if(in_state(RacePhase::Finished)))
            .add_systems(
                Update,
                apply_restart
                    .run_if(in_state(GameState::Menu).and_then(resource_exists::<PendingRestart>)),
            );
    }
}

fn apply_restart(mut commands: Commands, mut game: ResMut<NextState<GameState>>) {
    commands.remove_resource::<PendingRestart>();
    game.set(GameState::Racing);
}

fn show_results(
    mut commands: Commands,
    config: Option<Res<RaceConfig>>,
    cars: Query<(&CarIdentity, &RaceProgress)>,
) {
    let Some(config) = config else {
        return;
    };

    let mut standings: Vec<(&CarIdentity, &RaceProgress)> = cars.iter().collect();
    standings.sort_by_key(|(_, progress)| progress.position);

    let rows: Vec<_> = standings
        .iter()
        .map(|(identity, progress)| {
            let time = progress
                .finish_time
                .map(format_lap_time)
                // Quem ainda não cruzou a linha aparece com a volta em que está.
                .unwrap_or_else(|| format!("volta {}", progress.lap + 1));
            let best = progress
                .best_lap
                .map(format_lap_time)
                .unwrap_or_else(|| "--".to_string());

            let highlight = if identity.is_player {
                theme::ACCENT
            } else {
                theme::TEXT
            };

            (
                Node {
                    width: px(560),
                    justify_content: JustifyContent::SpaceBetween,
                    margin: UiRect::vertical(px(3)),
                    ..default()
                },
                children![
                    (
                        Text::new(format!("{}º  {}", progress.position, identity.name)),
                        theme::text(24.0, highlight),
                    ),
                    (
                        // Quanto o piloto ganhou ou perdeu em relação ao grid.
                        Text::new(format!("largou em {}º", identity.grid_slot + 1)),
                        theme::text(20.0, theme::TEXT_DIM),
                    ),
                    (Text::new(time), theme::text(24.0, theme::TEXT_DIM)),
                    (Text::new(best), theme::text(24.0, theme::TEXT_DIM)),
                ],
            )
        })
        .collect();

    let title = standings
        .iter()
        .find(|(identity, _)| identity.is_player)
        .map(|(_, progress)| match progress.position {
            1 => "VITÓRIA!".to_string(),
            place => format!("{place}º lugar"),
        })
        .unwrap_or_else(|| "Fim de corrida".to_string());

    commands.spawn((
        DespawnOnExit(RacePhase::Finished),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(theme::BACKDROP.with_alpha(0.82)),
        children![(
            theme::panel(),
            Children::spawn((
                Spawn((Text::new(title), theme::heading())),
                Spawn((
                    Text::new(if config.total_laps == 1 {
                        "1 volta".to_string()
                    } else {
                        format!("{} voltas", config.total_laps)
                    }),
                    theme::caption(),
                )),
                Spawn(theme::checkered_strip(14)),
                bevy::ecs::spawn::SpawnIter(rows.into_iter()),
                Spawn((
                    Node {
                        margin: UiRect::top(px(18)),
                        ..default()
                    },
                    children![
                        (
                            Button,
                            theme::button_node(),
                            theme::button_visuals(),
                            ResultsAction::Restart,
                            children![(Text::new("Correr de novo"), theme::button_text())],
                        ),
                        (
                            Button,
                            theme::button_node(),
                            theme::button_visuals(),
                            ResultsAction::BackToMenu,
                            children![(Text::new("Menu"), theme::button_text())],
                        ),
                    ],
                )),
            )),
        )],
    ));
}

fn results_action(
    mut commands: Commands,
    buttons: Query<(&Interaction, &ResultsAction), (Changed<Interaction>, With<Button>)>,
    mut game: ResMut<NextState<GameState>>,
    mut phase: ResMut<NextState<RacePhase>>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }

        // Nos dois casos a corrida é desmontada; correr de novo apenas deixa
        // um pedido que o menu reaproveita no quadro seguinte.
        game.set(GameState::Menu);
        phase.set(RacePhase::Inactive);

        if matches!(action, ResultsAction::Restart) {
            commands.insert_resource(PendingRestart);
        }
    }
}
