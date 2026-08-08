//! Racing Car — protótipo de corrida 3D arcade em Bevy + Avian.
//!
//! O executável apenas monta a janela e registra os plugins de cada domínio.
//! Toda a lógica vive nos módulos: `core`, `world`, `vehicle`, `camera`,
//! `race`, `ui`, `audio` e `dev`.

// Queries do Bevy com vários filtros disparam esse lint sem que haja ganho real
// em criar aliases de tipo para cada uma delas.
#![allow(clippy::type_complexity)]

mod audio;
mod camera;
mod core;
mod dev;
mod race;
mod ui;
mod vehicle;
mod world;

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::window::WindowResolution;

/// Os assets ficam ao lado do `Cargo.toml`, e não no diretório de trabalho,
/// para que o jogo rode igual via `cargo run` ou pelo binário compilado.
pub fn asset_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: asset_root().to_string_lossy().into_owned(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Racing Car".into(),
                        resolution: WindowResolution::new(1280, 720),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins((
            core::CorePlugin,
            dev::DevPlugin,
            world::WorldPlugin,
            vehicle::VehiclePlugin,
            camera::CameraPlugin,
            race::RacePlugin,
            ui::UiPlugin,
            audio::GameAudioPlugin,
        ))
        .run();
}
