//! Fundações compartilhadas: máquina de estados, configurações persistidas
//! e a integração com o motor de física.

pub mod physics;
pub mod settings;
pub mod state;

use bevy::prelude::*;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            state::StatePlugin,
            settings::SettingsPlugin,
            physics::GamePhysicsPlugin,
        ));
    }
}
