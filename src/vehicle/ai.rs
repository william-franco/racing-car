//! Pilotos controlados pelo computador.
//!
//! A IA não teleporta pela pista: ela dirige o mesmo carro físico do jogador,
//! preenchendo o mesmo [`DriveInput`]. O piloto mira um ponto adiante no
//! traçado, corrige o volante pelo erro angular e escolhe a velocidade
//! conforme a curvatura que vem pela frente.

use bevy::math::ops;
use bevy::prelude::*;
use rand::RngExt;

use crate::core::state::{AiCar, RacePhase};
use crate::world::track::TrackLayout;

use super::physics::{CarConfig, DriveInput, EngineState};

/// Personalidade de um piloto da IA.
#[derive(Component, Clone, Copy, Debug)]
pub struct AiDriver {
    /// Linha preferida, em metros a partir do eixo da pista.
    pub racing_line: f32,
    /// Multiplicador da velocidade-alvo; abaixo de 1 o piloto é mais cauteloso.
    pub skill: f32,
    /// Distância de antecipação base, em metros.
    pub look_ahead: f32,
    /// Índice da amostra mais próxima no quadro anterior, para busca local.
    pub last_index: Option<usize>,
}

impl AiDriver {
    pub fn new(index: usize, rng: &mut impl RngExt) -> Self {
        // Cada piloto tem uma linha e um ritmo levemente diferentes, o que faz
        // o pelotão se espalhar de forma natural.
        let side = if index.is_multiple_of(2) { -1.0 } else { 1.0 };
        Self {
            racing_line: side * rng.random_range(0.6..2.8),
            skill: 0.88 + rng.random_range(0.0..0.13) - index as f32 * 0.012,
            look_ahead: rng.random_range(11.0..16.0),
            last_index: None,
        }
    }
}

pub struct VehicleAiPlugin;

impl Plugin for VehicleAiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            drive_ai_cars.run_if(in_state(RacePhase::Green)),
        );
    }
}

fn drive_ai_cars(
    layout: Res<TrackLayout>,
    mut drivers: Query<
        (
            &mut AiDriver,
            &mut DriveInput,
            &EngineState,
            &CarConfig,
            &Transform,
        ),
        With<AiCar>,
    >,
) {
    for (mut driver, mut input, engine, config, transform) in &mut drivers {
        let location = layout.locate(transform.translation, driver.last_index);
        driver.last_index = Some(location.index);

        // A antecipação cresce com a velocidade: em reta o piloto olha longe,
        // em curva ele encurta a mira, senão corta o vértice e sai largo.
        let bend = (peak_curvature(&layout, location.distance, 40.0) * 40.0).clamp(0.0, 1.0);
        let look_ahead = (driver.look_ahead + engine.speed.abs() * 0.45) * (1.0 - 0.45 * bend);
        let target_distance = location.distance + look_ahead;
        let target_sample = layout.sample_at_distance(target_distance);

        // A linha desejada é limitada pela largura real da pista naquele ponto.
        let lateral = driver.racing_line.clamp(
            -target_sample.half_width + 2.0,
            target_sample.half_width - 2.0,
        );
        let target = target_sample.center + target_sample.right * lateral;

        // Quanto da pista já foi gasta para o lado de fora, de 0 a 1.
        let wide = (location.lateral.abs() / location.sample.half_width).clamp(0.0, 1.0);

        // --- Volante -------------------------------------------------------
        let to_target = (target - transform.translation).with_y(0.0);
        let forward = (transform.rotation * Vec3::NEG_Z).with_y(0.0);
        let steer = if to_target.length_squared() > 0.01 && forward.length_squared() > 0.01 {
            let to_target = to_target.normalize();
            let forward = forward.normalize();
            // Ângulo com sinal entre a frente do carro e o alvo.
            let angle = ops::atan2(
                forward.cross(to_target).dot(Vec3::Y),
                forward.dot(to_target),
            );
            (-angle * 2.2).clamp(-1.0, 1.0)
        } else {
            0.0
        };

        // Corrige a deriva contra-esterçando quando a traseira escapa.
        let counter_steer = engine.drift * engine.speed.signum() * 0.35;
        input.steer = (steer - counter_steer).clamp(-1.0, 1.0);

        // --- Acelerador e freio ---------------------------------------------
        // O horizonte de frenagem cresce com o quadrado da velocidade, que é
        // como cresce a distância necessária para desacelerar.
        let braking_horizon = look_ahead + engine.speed * engine.speed / 11.0;
        let curvature = peak_curvature(&layout, location.distance, braking_horizon);
        // Encostado na borda sobra menos pista para curvar, então o piloto se
        // dá uma margem até recuperar a linha.
        let margin = 1.0 - 0.25 * wide;
        let target_speed = corner_speed(curvature, config.top_speed) * driver.skill * margin;
        let error = target_speed - engine.speed;

        if error > 1.0 {
            input.throttle = (error * 0.4).clamp(0.0, 1.0);
            input.brake = 0.0;
        } else if error < -2.0 {
            input.throttle = 0.0;
            input.brake = (-error * 0.18).clamp(0.0, 1.0);
        } else {
            input.throttle = 0.35;
            input.brake = 0.0;
        }

        // Sair da pista custa caro, então o piloto alivia enquanto se recupera.
        if location.off_track() > 1.0 {
            input.throttle *= 0.55;
        }

        input.handbrake = false;
    }
}

/// Maior curvatura encontrada num trecho à frente do carro.
fn peak_curvature(layout: &TrackLayout, from: f32, span: f32) -> f32 {
    let spacing = layout.spacing().max(0.5);
    let steps = (span / spacing).ceil().max(1.0) as usize;
    let start = layout.index_at_distance(from);

    (0..steps)
        .map(|step| layout.sample((start + step) as i32).curvature.abs())
        .fold(0.0f32, f32::max)
}

/// Velocidade que uma curva daquela curvatura comporta.
fn corner_speed(curvature: f32, top_speed: f32) -> f32 {
    if curvature < 1e-4 {
        return top_speed;
    }
    // v = sqrt(a_lateral / k), com a aceleração lateral que o modelo de pneu
    // entrega de fato — chutar alto aqui faz o pelotão inteiro errar a curva.
    const LATERAL_ACCELERATION: f32 = 12.0;
    (LATERAL_ACCELERATION / curvature).sqrt().min(top_speed)
}
