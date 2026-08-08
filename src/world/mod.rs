//! O mundo do jogo: ambiente, terreno e circuito.

pub mod environment;
pub mod textures;
pub mod track;

use bevy::prelude::*;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((track::TrackPlugin, environment::EnvironmentPlugin));
    }
}
