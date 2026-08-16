//! Overlay de FPS com gráfico de frame time, ligado às teclas F2–F6.
//!
//! Quem manda na visibilidade é a opção [`ShowFps`], e não o `enabled` do
//! Bevy: assim o menu de vídeo e a tecla `F3` mexem no mesmo lugar e a escolha
//! sobrevive ao fechar o jogo.

use std::time::Duration;

use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig};
use bevy::prelude::*;
use bevy::text::FontSmoothing;

use crate::core::settings::ShowFps;

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
                enabled: true,
                frame_time_graph_config: FrameTimeGraphConfig {
                    // O gráfico tem visibilidade própria: se ficar ligado aqui
                    // ele aparece sozinho mesmo com o overlay desligado.
                    enabled: false,
                    min_fps: 30.0,
                    target_fps: 144.0,
                },
            },
        })
        .add_systems(Startup, apply_show_fps)
        .add_systems(
            Update,
            (
                control_overlay,
                apply_show_fps.run_if(resource_changed::<ShowFps>),
            )
                .chain(),
        );
    }
}

/// Reflete a opção do jogador no overlay do Bevy.
fn apply_show_fps(show: Res<ShowFps>, mut overlay: ResMut<FpsOverlayConfig>) {
    overlay.enabled = show.0;
    if !show.0 {
        overlay.frame_time_graph_config.enabled = false;
    }
}

fn control_overlay(
    input: Res<ButtonInput<KeyCode>>,
    mut show: ResMut<ShowFps>,
    mut overlay: ResMut<FpsOverlayConfig>,
) {
    if input.just_pressed(KeyCode::F3) {
        // Só inverte a opção: `apply_show_fps` cuida do overlay em seguida.
        show.0 = !show.0;
        overlay.frame_time_graph_config.enabled = show.0;
    }

    if input.just_pressed(KeyCode::F4) && show.0 {
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
