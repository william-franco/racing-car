//! Toda a interface: menus, HUD, minimapa e tela de resultados.

pub mod game_menu;
pub mod hud;
pub mod minimap;
pub mod results;
pub mod theme;

use bevy::prelude::*;

use crate::core::state::{GameState, RacePhase};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            theme::ThemePlugin,
            game_menu::GameMenuPlugin,
            hud::HudPlugin,
            minimap::MinimapPlugin,
            results::ResultsPlugin,
        ))
        .add_systems(
            Update,
            leave_race
                .run_if(in_state(GameState::Racing).and_then(not(in_state(RacePhase::Finished)))),
        );
    }
}

/// `Esc` abandona a corrida e volta ao menu.
fn leave_race(
    keys: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<NextState<GameState>>,
    mut phase: ResMut<NextState<RacePhase>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        game.set(GameState::Menu);
        phase.set(RacePhase::Inactive);
    }
}
