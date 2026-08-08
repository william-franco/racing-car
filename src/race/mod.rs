//! A corrida em si: checkpoints, voltas, classificação e recordes.

pub mod checkpoint;
pub mod flow;
pub mod records;

use bevy::prelude::*;

use crate::core::state::GameState;

pub use flow::{RaceClock, RaceConfig, RaceProgress};

pub struct RacePlugin;

impl Plugin for RacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((flow::RaceFlowPlugin, records::RecordsPlugin))
            .add_systems(OnEnter(GameState::Racing), checkpoint::spawn_checkpoints);
    }
}
