//! Trilha sonora do menu e da corrida, e o motor sintetizado do jogador.

pub mod engine;

use bevy::audio::{AddAudioSource, Volume as AudioVolume};
use bevy::prelude::*;

use crate::core::settings::Volume;
use crate::core::state::{GameState, PlayerCar, PlayerSnapshot};
use crate::vehicle::physics::CarConfig;

use engine::{EngineAudio, EngineSignal};

/// A música fica bem mais baixa que os efeitos para não abafar o motor.
const MUSIC_MIX: f32 = 0.28;
const ENGINE_MIX: f32 = 0.55;

#[derive(Component)]
struct MusicTrack;

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<EngineAudio>()
            .init_resource::<EngineSignal>()
            .add_systems(OnEnter(GameState::Menu), play_menu_music)
            .add_systems(OnEnter(GameState::Racing), (play_race_music, start_engine))
            .add_systems(
                Update,
                (
                    apply_volume.run_if(resource_changed::<Volume>),
                    drive_engine_sound.run_if(in_state(GameState::Racing)),
                ),
            );
    }
}

fn music(
    commands: &mut Commands,
    assets: &AssetServer,
    state: GameState,
    path: &'static str,
    volume: Volume,
) {
    commands.spawn((
        DespawnOnExit(state),
        MusicTrack,
        AudioPlayer::new(assets.load(path)),
        PlaybackSettings::LOOP.with_volume(AudioVolume::Linear(volume.linear() * MUSIC_MIX)),
    ));
}

fn play_menu_music(mut commands: Commands, assets: Res<AssetServer>, volume: Res<Volume>) {
    music(
        &mut commands,
        &assets,
        GameState::Menu,
        "sounds/menu_music.ogg",
        *volume,
    );
}

fn play_race_music(mut commands: Commands, assets: Res<AssetServer>, volume: Res<Volume>) {
    music(
        &mut commands,
        &assets,
        GameState::Racing,
        "sounds/race_music.ogg",
        *volume,
    );
}

/// O motor toca ininterruptamente; o que varia é a frequência e o ganho.
fn start_engine(
    mut commands: Commands,
    mut sources: ResMut<Assets<EngineAudio>>,
    signal: Res<EngineSignal>,
) {
    let source = sources.add(EngineAudio::new(signal.clone()));
    commands.spawn((
        DespawnOnExit(GameState::Racing),
        AudioPlayer(source),
        PlaybackSettings::LOOP,
    ));
}

fn drive_engine_sound(
    snapshot: Res<PlayerSnapshot>,
    signal: Res<EngineSignal>,
    volume: Res<Volume>,
    config: Query<&CarConfig, With<PlayerCar>>,
) {
    let Ok(config) = config.single() else {
        signal.set(0.0, 0.0);
        return;
    };

    let rpm =
        ((snapshot.rpm - config.idle_rpm) / (config.max_rpm - config.idle_rpm)).clamp(0.0, 1.0);

    // Mesmo em marcha lenta o motor se ouve; o acelerador só encorpa o som.
    let load = 0.45 + 0.55 * snapshot.throttle;
    signal.set(rpm, volume.linear() * ENGINE_MIX * load);
}

/// Reaplica o volume das configurações nas faixas que já estão tocando.
fn apply_volume(volume: Res<Volume>, mut tracks: Query<&mut AudioSink, With<MusicTrack>>) {
    for mut sink in &mut tracks {
        sink.set_volume(AudioVolume::Linear(volume.linear() * MUSIC_MIX));
    }
}
