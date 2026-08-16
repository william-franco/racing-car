//! Splash, menu principal e telas de configuração.
//!
//! Cada tela é um estado próprio em [`MenuState`] e registra suas entidades
//! com `DespawnOnExit`, então trocar de tela já limpa a anterior — não existe
//! nenhum sistema manual de limpeza aqui.

use bevy::app::AppExit;
use bevy::ecs::component::Mutable;
use bevy::ecs::spawn::SpawnWith;
use bevy::prelude::*;

use crate::core::settings::{
    DisplayQuality, LapCount, OpponentCount, Records, ShowFps, SteerSensitivity, Volume,
};
use crate::core::state::{GameState, MenuCamera};
use crate::race::records::format_lap_time;

use super::theme;

/// Tela ativa dentro do menu.
#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum MenuState {
    Main,
    Settings,
    SettingsDisplay,
    SettingsAudio,
    SettingsRace,
    SettingsSteering,
    Controls,
    Credits,
    /// Nenhuma tela de menu no ar (durante a corrida).
    #[default]
    Disabled,
}

/// Marca o botão que representa o valor atualmente escolhido de uma opção.
#[derive(Component)]
struct SelectedOption;

/// Associa um botão ao valor de configuração que ele aplica.
#[derive(Component)]
struct Setting<T>(T);

#[derive(Component, Clone, Copy)]
enum MenuAction {
    Race,
    Settings,
    Display,
    Audio,
    RaceRules,
    Steering,
    Controls,
    Credits,
    BackToMain,
    BackToSettings,
    Quit,
}

pub struct GameMenuPlugin;

impl Plugin for GameMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<MenuState>()
            .add_systems(OnEnter(GameState::Splash), splash_setup)
            .add_systems(OnEnter(GameState::Menu), (menu_camera, open_main_menu))
            .add_systems(OnExit(GameState::Menu), close_menu)
            .add_systems(OnEnter(MenuState::Main), main_menu)
            .add_systems(OnEnter(MenuState::Settings), settings_menu)
            .add_systems(OnEnter(MenuState::SettingsDisplay), display_menu)
            .add_systems(OnEnter(MenuState::SettingsAudio), audio_menu)
            .add_systems(OnEnter(MenuState::SettingsRace), race_menu)
            .add_systems(OnEnter(MenuState::SettingsSteering), steering_menu)
            .add_systems(OnEnter(MenuState::Controls), controls_menu)
            .add_systems(OnEnter(MenuState::Credits), credits_menu)
            .add_systems(Update, splash_countdown.run_if(in_state(GameState::Splash)))
            .add_systems(
                Update,
                (
                    button_feedback,
                    menu_action,
                    (setting_button::<DisplayQuality>, setting_button::<ShowFps>)
                        .run_if(in_state(MenuState::SettingsDisplay)),
                    setting_button::<Volume>.run_if(in_state(MenuState::SettingsAudio)),
                    (setting_button::<LapCount>, setting_button::<OpponentCount>)
                        .run_if(in_state(MenuState::SettingsRace)),
                    setting_button::<SteerSensitivity>
                        .run_if(in_state(MenuState::SettingsSteering)),
                )
                    .run_if(in_state(GameState::Menu)),
            );
    }
}

fn menu_camera(mut commands: Commands) {
    commands.spawn((DespawnOnExit(GameState::Menu), Camera2d, MenuCamera));
}

// --- Splash -----------------------------------------------------------------

#[derive(Resource, Deref, DerefMut)]
struct SplashTimer(Timer);

fn splash_setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((DespawnOnExit(GameState::Splash), Camera2d, MenuCamera));
    commands.spawn((
        DespawnOnExit(GameState::Splash),
        theme::screen(),
        BackgroundColor(theme::BACKDROP),
        children![
            (
                ImageNode::new(assets.load("branding/icon.png")),
                Node {
                    width: px(180),
                    ..default()
                },
            ),
            (Text::new("RACING CAR"), theme::heading()),
            (
                Text::new("um protótipo de corrida em Bevy + Avian"),
                theme::caption(),
            ),
        ],
    ));

    commands.insert_resource(SplashTimer(Timer::from_seconds(1.6, TimerMode::Once)));
}

fn splash_countdown(
    time: Res<Time>,
    mut timer: ResMut<SplashTimer>,
    mut state: ResMut<NextState<GameState>>,
) {
    if timer.tick(time.delta()).is_finished() {
        state.set(GameState::Menu);
    }
}

// --- Navegação ---------------------------------------------------------------

fn open_main_menu(mut menu: ResMut<NextState<MenuState>>) {
    menu.set(MenuState::Main);
}

fn close_menu(mut menu: ResMut<NextState<MenuState>>) {
    menu.set(MenuState::Disabled);
}

fn button_feedback(
    mut buttons: Query<
        (&Interaction, &mut BackgroundColor, Has<SelectedOption>),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut background, selected) in &mut buttons {
        *background = match (*interaction, selected) {
            (Interaction::Pressed, _) => theme::BUTTON_PRESSED.into(),
            (Interaction::Hovered, true) => theme::ACCENT_DEEP.into(),
            (Interaction::Hovered, false) => theme::BUTTON_HOVER.into(),
            (Interaction::None, true) => theme::BUTTON_SELECTED.into(),
            (Interaction::None, false) => theme::BUTTON_IDLE.into(),
        };
    }
}

fn menu_action(
    buttons: Query<(&Interaction, &MenuAction), (Changed<Interaction>, With<Button>)>,
    mut exit: MessageWriter<AppExit>,
    mut menu: ResMut<NextState<MenuState>>,
    mut game: ResMut<NextState<GameState>>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match action {
            MenuAction::Race => {
                game.set(GameState::Racing);
                menu.set(MenuState::Disabled);
            }
            MenuAction::Settings => menu.set(MenuState::Settings),
            MenuAction::Display => menu.set(MenuState::SettingsDisplay),
            MenuAction::Audio => menu.set(MenuState::SettingsAudio),
            MenuAction::RaceRules => menu.set(MenuState::SettingsRace),
            MenuAction::Steering => menu.set(MenuState::SettingsSteering),
            MenuAction::Controls => menu.set(MenuState::Controls),
            MenuAction::Credits => menu.set(MenuState::Credits),
            MenuAction::BackToMain => menu.set(MenuState::Main),
            MenuAction::BackToSettings => menu.set(MenuState::Settings),
            MenuAction::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

/// Aplica o valor de uma opção quando o botão correspondente é clicado.
///
/// É genérico sobre o tipo da opção, então serve para qualidade, volume,
/// voltas e adversários sem repetição.
fn setting_button<T: Resource<Mutability = Mutable> + Component + PartialEq + Copy>(
    buttons: Query<(&Interaction, &Setting<T>, Entity), (Changed<Interaction>, With<Button>)>,
    selected: Single<(Entity, &mut BackgroundColor), (With<SelectedOption>, With<Setting<T>>)>,
    mut commands: Commands,
    mut setting: ResMut<T>,
) {
    let (previous, mut previous_color) = selected.into_inner();

    for (interaction, choice, entity) in &buttons {
        if *interaction == Interaction::Pressed && *setting != choice.0 {
            *previous_color = theme::BUTTON_IDLE.into();
            commands.entity(previous).remove::<SelectedOption>();
            commands.entity(entity).insert(SelectedOption);
            *setting = choice.0;
        }
    }
}

// --- Telas -------------------------------------------------------------------

/// Botão de ação largo, com rótulo e ícone opcional.
fn action_button(label: &str, action: MenuAction, icon: Option<Handle<Image>>) -> impl Bundle {
    let label = label.to_string();

    (
        Button,
        theme::button_node(),
        theme::button_visuals(),
        action,
        Children::spawn(SpawnWith(move |parent: &mut ChildSpawner| {
            if let Some(icon) = icon {
                parent.spawn((
                    ImageNode::new(icon),
                    Node {
                        width: px(26),
                        position_type: PositionType::Absolute,
                        left: px(14),
                        ..default()
                    },
                ));
            }
            parent.spawn((Text::new(label), theme::button_text()));
        })),
    )
}

fn main_menu(mut commands: Commands, assets: Res<AssetServer>, records: Res<Records>) {
    let best = records
        .best_lap
        .map(|lap| format!("Melhor volta: {}", format_lap_time(lap)))
        .unwrap_or_else(|| "Nenhuma volta registrada ainda".to_string());

    commands.spawn((
        DespawnOnExit(MenuState::Main),
        theme::screen(),
        BackgroundColor(theme::BACKDROP),
        children![(
            theme::panel(),
            children![
                (Text::new("RACING CAR"), theme::heading()),
                theme::checkered_strip(14),
                action_button(
                    "Correr",
                    MenuAction::Race,
                    Some(assets.load("textures/ui/play.png"))
                ),
                action_button(
                    "Configurações",
                    MenuAction::Settings,
                    Some(assets.load("textures/ui/settings.png"))
                ),
                action_button("Controles", MenuAction::Controls, None),
                action_button("Créditos", MenuAction::Credits, None),
                action_button(
                    "Sair",
                    MenuAction::Quit,
                    Some(assets.load("textures/ui/exit.png"))
                ),
                (Text::new(best), theme::caption()),
            ]
        )],
    ));
}

fn settings_menu(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(MenuState::Settings),
        theme::screen(),
        BackgroundColor(theme::BACKDROP),
        children![(
            theme::panel(),
            children![
                (Text::new("Configurações"), theme::subheading()),
                theme::checkered_strip(10),
                action_button("Vídeo", MenuAction::Display, None),
                action_button("Áudio", MenuAction::Audio, None),
                action_button("Corrida", MenuAction::RaceRules, None),
                action_button("Direção", MenuAction::Steering, None),
                action_button("Voltar", MenuAction::BackToMain, None),
            ]
        )],
    ));
}

/// Uma linha de opções: rótulo à esquerda e os valores possíveis à direita.
fn option_row<T, I>(label: &str, current: T, values: I, chip_width: f32) -> impl Bundle
where
    T: Resource<Mutability = Mutable> + Component + PartialEq + Copy,
    I: IntoIterator<Item = (T, String)> + Send + Sync + 'static,
{
    let label = label.to_string();

    (
        Node {
            align_items: AlignItems::Center,
            margin: UiRect::vertical(px(6)),
            ..default()
        },
        Children::spawn(SpawnWith(move |parent: &mut ChildSpawner| {
            parent.spawn((
                Text::new(label),
                theme::text(26.0, theme::TEXT_DIM),
                Node {
                    // Larga o bastante para o maior rótulo caber numa linha só.
                    width: px(230),
                    ..default()
                },
            ));

            for (value, caption) in values {
                let mut chip = parent.spawn((
                    Button,
                    theme::chip_node(chip_width),
                    theme::button_visuals(),
                    Setting(value),
                    children![(Text::new(caption), theme::text(22.0, theme::TEXT))],
                ));

                if current == value {
                    chip.insert((SelectedOption, BackgroundColor(theme::BUTTON_SELECTED)));
                }
            }
        })),
    )
}

fn display_menu(mut commands: Commands, quality: Res<DisplayQuality>, show_fps: Res<ShowFps>) {
    let options = [
        (DisplayQuality::Low, "Baixa".to_string()),
        (DisplayQuality::Medium, "Média".to_string()),
        (DisplayQuality::High, "Alta".to_string()),
    ];
    let fps_options = [
        (ShowFps(true), "Ligado".to_string()),
        (ShowFps(false), "Desligado".to_string()),
    ];

    commands.spawn((
        DespawnOnExit(MenuState::SettingsDisplay),
        theme::screen(),
        BackgroundColor(theme::BACKDROP),
        children![(
            theme::panel(),
            children![
                (Text::new("Vídeo"), theme::subheading()),
                theme::checkered_strip(10),
                option_row("Qualidade", *quality, options, 130.0),
                option_row("Contador de FPS", *show_fps, fps_options, 130.0),
                (
                    Text::new(
                        "A qualidade controla sombras, densidade do cenário e\n\
                         a intensidade do motion blur. O contador também liga e\n\
                         desliga com F3."
                    ),
                    theme::caption(),
                ),
                action_button("Voltar", MenuAction::BackToSettings, None),
            ]
        )],
    ));
}

fn audio_menu(mut commands: Commands, volume: Res<Volume>) {
    let options: Vec<(Volume, String)> = (0..=Volume::MAX)
        .map(|step| (Volume(step), step.to_string()))
        .collect();

    commands.spawn((
        DespawnOnExit(MenuState::SettingsAudio),
        theme::screen(),
        BackgroundColor(theme::BACKDROP),
        children![(
            theme::panel(),
            children![
                (Text::new("Áudio"), theme::subheading()),
                theme::checkered_strip(10),
                option_row("Volume", *volume, options, 44.0),
                action_button("Voltar", MenuAction::BackToSettings, None),
            ]
        )],
    ));
}

fn race_menu(mut commands: Commands, laps: Res<LapCount>, opponents: Res<OpponentCount>) {
    let lap_options: Vec<(LapCount, String)> = [1u32, 2, 3, 5, 8]
        .into_iter()
        .map(|count| (LapCount(count), count.to_string()))
        .collect();
    let opponent_options: Vec<(OpponentCount, String)> = [0u32, 3, 5, 7, 9]
        .into_iter()
        .map(|count| (OpponentCount(count), count.to_string()))
        .collect();

    commands.spawn((
        DespawnOnExit(MenuState::SettingsRace),
        theme::screen(),
        BackgroundColor(theme::BACKDROP),
        children![(
            theme::panel(),
            children![
                (Text::new("Corrida"), theme::subheading()),
                theme::checkered_strip(10),
                option_row("Voltas", *laps, lap_options, 66.0),
                option_row("Adversários", *opponents, opponent_options, 66.0),
                action_button("Voltar", MenuAction::BackToSettings, None),
            ]
        )],
    ));
}

fn steering_menu(mut commands: Commands, sensitivity: Res<SteerSensitivity>) {
    let options: Vec<(SteerSensitivity, String)> = (0..=SteerSensitivity::MAX)
        .map(|step| {
            let value = SteerSensitivity(step);
            (value, value.label().to_string())
        })
        .collect();

    commands.spawn((
        DespawnOnExit(MenuState::SettingsSteering),
        theme::screen(),
        BackgroundColor(theme::BACKDROP),
        children![(
            theme::panel(),
            children![
                (Text::new("Direção"), theme::subheading()),
                theme::checkered_strip(10),
                option_row("Sensibilidade", *sensitivity, options, 140.0),
                (
                    Text::new(
                        "Define o quanto o volante gira ao segurar A/D e a rapidez\n\
                         da resposta. Valores baixos ajudam em alta velocidade."
                    ),
                    theme::caption(),
                ),
                action_button("Voltar", MenuAction::BackToSettings, None),
            ]
        )],
    ));
}

fn credits_menu(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(MenuState::Credits),
        theme::screen(),
        BackgroundColor(theme::BACKDROP),
        children![(
            theme::panel(),
            children![
                (Text::new("Créditos"), theme::subheading()),
                theme::checkered_strip(10),
                (
                    Text::new("Racing Car — protótipo de corrida em Rust"),
                    theme::text(26.0, theme::TEXT),
                ),
                (
                    Text::new("Desenvolvido por William Franco"),
                    theme::text(24.0, theme::TEXT_DIM),
                ),
                (
                    Text::new("github.com/william-franco/racing-car"),
                    theme::text(26.0, theme::ACCENT),
                    Node {
                        margin: UiRect::vertical(px(14)),
                        ..default()
                    },
                ),
                (
                    Text::new(
                        "Motor Bevy 0.19 e física Avian3D 0.7.\n\
                         Tipografia Fira Sans, da Mozilla, sob a SIL Open Font\n\
                         License 1.1 (assets/fonts/OFL.txt)."
                    ),
                    theme::caption(),
                ),
                action_button("Voltar", MenuAction::BackToMain, None),
            ]
        )],
    ));
}

fn controls_menu(mut commands: Commands) {
    const BINDINGS: [(&str, &str); 9] = [
        ("W / ↑", "Acelerar"),
        ("S / ↓", "Frear e engatar a ré"),
        ("A / D", "Esterçar"),
        ("Espaço", "Freio de mão"),
        ("C", "Alternar câmera"),
        ("R", "Voltar para a pista"),
        ("1 – 4", "Ajustar o motion blur"),
        ("F3 / F7 / F8", "FPS, colisores e traçado"),
        ("Esc", "Voltar ao menu"),
    ];

    let rows: Vec<_> = BINDINGS
        .iter()
        .map(|(key, description)| {
            (
                Node {
                    width: px(520),
                    justify_content: JustifyContent::SpaceBetween,
                    margin: UiRect::vertical(px(4)),
                    ..default()
                },
                children![
                    (Text::new(*key), theme::text(24.0, theme::ACCENT)),
                    (Text::new(*description), theme::text(24.0, theme::TEXT)),
                ],
            )
        })
        .collect();

    commands.spawn((
        DespawnOnExit(MenuState::Controls),
        theme::screen(),
        BackgroundColor(theme::BACKDROP),
        children![(
            theme::panel(),
            Children::spawn((
                Spawn((Text::new("Controles"), theme::subheading())),
                Spawn(theme::checkered_strip(10)),
                bevy::ecs::spawn::SpawnIter(rows.into_iter()),
                Spawn(action_button("Voltar", MenuAction::BackToMain, None)),
            )),
        )],
    ));
}
