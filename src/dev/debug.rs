//! Gizmos de depuração: colisores do Avian (F7), traçado da pista (F8) e
//! captura de tela (F9).

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use crate::race::checkpoint::Checkpoint;
use crate::world::track::TrackLayout;

/// Liga/desliga o desenho do traçado, dos checkpoints e da linha de corrida.
#[derive(Resource, Default)]
pub struct TrackGizmos(pub bool);

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsDebugPlugin)
            .init_resource::<TrackGizmos>()
            .add_systems(Startup, disable_physics_gizmos)
            .add_systems(
                Update,
                (
                    toggle_gizmos,
                    take_screenshot,
                    draw_track_gizmos.run_if(track_gizmos_enabled),
                ),
            );
    }
}

fn disable_physics_gizmos(mut store: ResMut<GizmoConfigStore>) {
    store.config_mut::<PhysicsGizmos>().0.enabled = false;
}

fn track_gizmos_enabled(toggle: Res<TrackGizmos>) -> bool {
    toggle.0
}

fn toggle_gizmos(
    input: Res<ButtonInput<KeyCode>>,
    mut store: ResMut<GizmoConfigStore>,
    mut track: ResMut<TrackGizmos>,
) {
    if input.just_pressed(KeyCode::F7) {
        let config = store.config_mut::<PhysicsGizmos>().0;
        config.enabled = !config.enabled;
    }
    if input.just_pressed(KeyCode::F8) {
        track.0 = !track.0;
    }
}

/// Salva um PNG da janela em `screenshots/`, útil para registrar bugs visuais.
fn take_screenshot(
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut counter: Local<u32>,
) {
    if !input.just_pressed(KeyCode::F9) {
        return;
    }

    if let Err(error) = std::fs::create_dir_all("screenshots") {
        warn!("não foi possível criar a pasta screenshots: {error}");
        return;
    }

    let path = format!("screenshots/racing-car-{:03}.png", *counter);
    *counter += 1;
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

fn draw_track_gizmos(
    mut gizmos: Gizmos,
    layout: Option<Res<TrackLayout>>,
    checkpoints: Query<(&Transform, &Checkpoint)>,
) {
    let Some(layout) = layout else {
        return;
    };

    let samples = layout.samples();
    for window in samples.windows(2) {
        let (a, b) = (&window[0], &window[1]);
        gizmos.line(a.center, b.center, Color::srgb(1.0, 0.9, 0.2));
        gizmos.line(a.left_edge(), b.left_edge(), Color::srgb(0.2, 0.9, 1.0));
        gizmos.line(a.right_edge(), b.right_edge(), Color::srgb(0.2, 0.9, 1.0));
    }

    for (transform, checkpoint) in &checkpoints {
        let color = if checkpoint.index == 0 {
            Color::srgb(0.2, 1.0, 0.4)
        } else {
            Color::srgb(1.0, 0.4, 0.8)
        };
        gizmos.rect(
            Isometry3d::new(transform.translation, transform.rotation),
            Vec2::new(checkpoint.half_width * 2.0, 6.0),
            color,
        );
    }
}
