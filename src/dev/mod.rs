//! Ferramentas de desenvolvimento: contador de FPS e gizmos de depuração.

pub mod debug;
pub mod fps_overlay;

use bevy::prelude::*;

pub struct DevPlugin;

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((fps_overlay::GameFpsOverlayPlugin, debug::DebugPlugin));
    }
}
