//! Overlay de FPS com gráfico de frame time, ligado às teclas F3–F6.

use std::time::Duration;

use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig};
use bevy::prelude::*;
use bevy::text::FontSmoothing;

const OVERLAY_COLOR: Color = Color::srgb(0.45, 1.0, 0.55);
const OVERLAY_ALERT: Color = Color::srgb(1.0, 0.45, 0.35);

pub struct GameFpsOverlayPlugin;

impl Plugin for GameFpsOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FpsOverlayPlugin {
            config: FpsOverlayConfig {
                text_config: TextFont {
                    font_size: FontSize::Px(20.0),
                    font: default(),
                    font_smoothing: FontSmoothing::default(),
                    ..default()
                },
                text_color: OVERLAY_COLOR,
                refresh_interval: Duration::from_millis(120),
                // Começa desligado para não poluir o menu; F3 liga.
                enabled: false,
                frame_time_graph_config: FrameTimeGraphConfig {
                    // O gráfico tem visibilidade própria: se ficar ligado aqui
                    // ele aparece sozinho mesmo com o overlay desligado.
                    enabled: false,
                    min_fps: 30.0,
                    target_fps: 144.0,
                },
            },
        })
        .add_systems(Update, control_overlay);
    }
}

fn control_overlay(input: Res<ButtonInput<KeyCode>>, mut overlay: ResMut<FpsOverlayConfig>) {
    if input.just_pressed(KeyCode::F3) {
        overlay.enabled = !overlay.enabled;
        overlay.frame_time_graph_config.enabled = overlay.enabled;
    }

    if input.just_pressed(KeyCode::F4) && overlay.enabled {
        overlay.frame_time_graph_config.enabled = !overlay.frame_time_graph_config.enabled;
    }

    if let FontSize::Px(size) = overlay.text_config.font_size {
        let mut next = size;
        if input.just_pressed(KeyCode::F5) {
            next = (size - 2.0).max(10.0);
        }
        if input.just_pressed(KeyCode::F6) {
            next = (size + 2.0).min(48.0);
        }
        if next != size {
            overlay.text_config.font_size = FontSize::Px(next);
        }
    }

    if input.just_pressed(KeyCode::F2) {
        overlay.text_color = if overlay.text_color == OVERLAY_COLOR {
            OVERLAY_ALERT
        } else {
            OVERLAY_COLOR
        };
    }
}
