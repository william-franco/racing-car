//! Painel de corrida: velocímetro, marcha, voltas, tempos e posição.

use bevy::prelude::*;

use crate::camera::CameraMode;
use crate::core::settings::Records;
use crate::core::state::{GameState, PlayerCar, PlayerSnapshot, RacePhase};
use crate::race::records::format_lap_time;
use crate::race::{RaceClock, RaceConfig, RaceProgress};
use crate::vehicle::physics::CarConfig;

use super::theme;

#[derive(Component)]
struct SpeedValue;

#[derive(Component)]
struct GearValue;

/// Barra que enche conforme a rotação do motor sobe.
#[derive(Component)]
struct RpmBar;

#[derive(Component)]
struct LapValue;

#[derive(Component)]
struct PositionValue;

/// Raiz do bloco de tempos; os valores ficam em `TextSpan` filhos.
#[derive(Component)]
struct TimesPanel;

#[derive(Component)]
struct CountdownBanner;

#[derive(Component)]
struct CameraLabel;

/// Avisa quando o carro está de través ou fora do chão.
#[derive(Component)]
struct GripStatus;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Racing), spawn_hud)
            .add_systems(
                Update,
                (
                    update_speed,
                    update_standings,
                    update_times,
                    update_countdown,
                    update_camera_label,
                )
                    .run_if(in_state(GameState::Racing)),
            );
    }
}

/// Caixa translúcida usada em cada canto do HUD. O `anchor` traz só o
/// posicionamento, para que exista um único `Node` no bundle final.
fn hud_panel(anchor: Node) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::axes(px(16), px(10)),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(8)),
            row_gap: px(2),
            ..anchor
        },
        BackgroundColor(theme::PANEL_DEEP.with_alpha(0.72)),
        BorderColor::all(theme::BORDER.with_alpha(0.7)),
    )
}

fn spawn_hud(mut commands: Commands) {
    // Canto superior esquerdo: volta e colocação.
    commands.spawn((
        DespawnOnExit(GameState::Racing),
        hud_panel(Node {
            position_type: PositionType::Absolute,
            top: px(16),
            left: px(16),
            ..default()
        }),
        children![
            (Text::new("VOLTA"), theme::text(16.0, theme::TEXT_DIM)),
            (
                Text::new("1 / 3"),
                theme::text(34.0, theme::ACCENT),
                LapValue
            ),
            (Text::new("POSIÇÃO"), theme::text(16.0, theme::TEXT_DIM)),
            (
                Text::new("-"),
                theme::text(34.0, theme::TEXT),
                PositionValue
            ),
        ],
    ));

    // Canto superior direito: tempos da volta.
    commands.spawn((
        DespawnOnExit(GameState::Racing),
        hud_panel(Node {
            position_type: PositionType::Absolute,
            top: px(16),
            right: px(16),
            align_items: AlignItems::End,
            ..default()
        }),
        children![(
            Text::new("ATUAL  "),
            theme::text(22.0, theme::TEXT_DIM),
            TimesPanel,
            children![
                (TextSpan::new("0:00.000"), theme::text(22.0, theme::TEXT)),
                (
                    TextSpan::new("\nÚLTIMA  "),
                    theme::text(22.0, theme::TEXT_DIM)
                ),
                (TextSpan::new("--"), theme::text(22.0, theme::TEXT)),
                (
                    TextSpan::new("\nMELHOR  "),
                    theme::text(22.0, theme::TEXT_DIM)
                ),
                (TextSpan::new("--"), theme::text(22.0, theme::ACCENT)),
            ],
        )],
    ));

    // Canto inferior direito: velocímetro, marcha e conta-giros.
    commands.spawn((
        DespawnOnExit(GameState::Racing),
        hud_panel(Node {
            position_type: PositionType::Absolute,
            bottom: px(16),
            right: px(16),
            align_items: AlignItems::End,
            ..default()
        }),
        children![
            (
                Node {
                    align_items: AlignItems::Baseline,
                    column_gap: px(8),
                    ..default()
                },
                children![
                    (Text::new("0"), theme::text(64.0, theme::TEXT), SpeedValue),
                    (Text::new("km/h"), theme::text(20.0, theme::TEXT_DIM)),
                    (Text::new("N"), theme::text(46.0, theme::ACCENT), GearValue),
                ],
            ),
            (Text::new(""), theme::text(18.0, theme::DANGER), GripStatus),
            (
                Node {
                    width: px(260),
                    height: px(10),
                    border_radius: BorderRadius::all(px(5)),
                    ..default()
                },
                BackgroundColor(theme::PANEL),
                children![(
                    Node {
                        width: percent(0),
                        height: percent(100),
                        border_radius: BorderRadius::all(px(5)),
                        ..default()
                    },
                    BackgroundColor(theme::ACCENT),
                    RpmBar,
                )],
            ),
        ],
    ));

    // Centro: semáforo da largada e mensagens de fase.
    commands.spawn((
        DespawnOnExit(GameState::Racing),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        Pickable::IGNORE,
        children![(
            Text::new(""),
            theme::text(120.0, theme::ACCENT),
            CountdownBanner
        )],
    ));

    // Topo central: modo de câmera em uso.
    commands.spawn((
        DespawnOnExit(GameState::Racing),
        Node {
            position_type: PositionType::Absolute,
            top: px(18),
            width: percent(100),
            justify_content: JustifyContent::Center,
            ..default()
        },
        Pickable::IGNORE,
        children![(
            Text::new(""),
            theme::text(20.0, theme::TEXT_DIM),
            CameraLabel
        )],
    ));
}

fn update_speed(
    snapshot: Res<PlayerSnapshot>,
    config: Query<&CarConfig, With<PlayerCar>>,
    mut speed: Single<&mut Text, With<SpeedValue>>,
    mut gear: Single<&mut Text, (With<GearValue>, Without<SpeedValue>)>,
    mut status: Single<&mut Text, (With<GripStatus>, Without<SpeedValue>, Without<GearValue>)>,
    mut bar: Single<&mut Node, With<RpmBar>>,
) {
    speed.0 = format!("{:.0}", snapshot.speed_kmh());
    gear.0 = match snapshot.gear {
        gear if gear < 0 => "R".to_string(),
        0 => "N".to_string(),
        gear => gear.to_string(),
    };

    status.0 = if !snapshot.grounded {
        "NO AR".to_string()
    } else if snapshot.drift > 0.32 {
        "DERRAPANDO".to_string()
    } else {
        String::new()
    };

    let Ok(config) = config.single() else {
        return;
    };
    let fraction =
        ((snapshot.rpm - config.idle_rpm) / (config.max_rpm - config.idle_rpm)).clamp(0.0, 1.0);
    bar.width = percent(fraction * 100.0);
}

fn update_standings(
    config: Option<Res<RaceConfig>>,
    player: Query<&RaceProgress, With<PlayerCar>>,
    mut lap: Single<&mut Text, With<LapValue>>,
    mut position: Single<&mut Text, (With<PositionValue>, Without<LapValue>)>,
) {
    let (Some(config), Ok(progress)) = (config, player.single()) else {
        return;
    };

    let current = (progress.lap + 1).min(config.total_laps);
    lap.0 = format!("{current} / {}", config.total_laps);
    position.0 = format!("{} / {}", progress.position, config.entrants);
}

fn update_times(
    clock: Res<RaceClock>,
    records: Res<Records>,
    player: Query<&RaceProgress, With<PlayerCar>>,
    panel: Single<Entity, With<TimesPanel>>,
    mut writer: TextUiWriter,
) {
    let Ok(progress) = player.single() else {
        return;
    };
    let panel = *panel;

    let current = if progress.started && !progress.finished {
        clock.elapsed - progress.lap_start
    } else {
        0.0
    };

    // O índice 0 é o próprio `Text`; os `TextSpan` filhos vêm a partir do 1.
    *writer.text(panel, 1) = format_lap_time(current);
    *writer.text(panel, 3) = progress
        .last_lap
        .map(format_lap_time)
        .unwrap_or_else(|| "--".to_string());
    *writer.text(panel, 5) = records
        .best_lap
        .map(format_lap_time)
        .unwrap_or_else(|| "--".to_string());
}

fn update_countdown(
    phase: Res<State<RacePhase>>,
    clock: Res<RaceClock>,
    mut banner: Single<(&mut Text, &mut TextColor), With<CountdownBanner>>,
) {
    let (text, color) = &mut *banner;

    match phase.get() {
        RacePhase::Countdown => match clock.countdown_number() {
            Some(number) if number > 1 => {
                text.0 = (number - 1).to_string();
                color.0 = theme::DANGER;
            }
            _ => {
                text.0 = "VAI!".to_string();
                color.0 = theme::SUCCESS;
            }
        },
        // Logo depois da largada a mensagem "VAI!" some sozinha.
        RacePhase::Green if clock.elapsed < 1.2 => {
            text.0 = "VAI!".to_string();
            color.0 = theme::SUCCESS;
        }
        _ => text.0.clear(),
    }
}

fn update_camera_label(mode: Res<CameraMode>, mut label: Single<&mut Text, With<CameraLabel>>) {
    if mode.is_changed() {
        label.0 = format!("Câmera: {}", mode.label());
    }
}
