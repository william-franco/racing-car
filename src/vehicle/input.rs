//! Traduz o teclado em comandos de direção para o carro do jogador.

use bevy::prelude::*;

use crate::core::settings::SteerSensitivity;
use crate::core::state::{GameState, PlayerCar};

use super::physics::DriveInput;

/// Quanto o volante gira por segundo ao segurar a tecla, e quanto ele volta ao
/// centro ao soltar. Teclado é digital: sem essa rampa o carro recebe esterço
/// máximo num único quadro e roda.
const STEER_ATTACK: f32 = 3.5;
const STEER_RELEASE: f32 = 6.0;

/// Pedido de reposicionamento do carro na pista, disparado pelo jogador.
#[derive(Message, Debug, Clone, Copy)]
pub struct RespawnRequested;

pub struct VehicleInputPlugin;

impl Plugin for VehicleInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<RespawnRequested>()
            .add_systems(Update, read_keyboard.run_if(in_state(GameState::Racing)));
    }
}

fn read_keyboard(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    sensitivity: Res<SteerSensitivity>,
    mut respawn: MessageWriter<RespawnRequested>,
    mut player: Query<&mut DriveInput, With<PlayerCar>>,
) {
    let Ok(mut input) = player.single_mut() else {
        return;
    };

    let pressed = |codes: [KeyCode; 2]| codes.iter().any(|code| keys.pressed(*code)) as u8 as f32;

    input.throttle = pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    input.brake = pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    input.handbrake = keys.pressed(KeyCode::Space);

    // A sensibilidade mexe em duas coisas ao mesmo tempo: o quanto o volante
    // chega a girar e a rapidez com que ele responde.
    let gain = sensitivity.multiplier();
    let target = (pressed([KeyCode::KeyD, KeyCode::ArrowRight])
        - pressed([KeyCode::KeyA, KeyCode::ArrowLeft]))
        * gain;
    let target = target.clamp(-1.0, 1.0);

    // Voltar ao centro, ou trocar de lado, é mais rápido que carregar o esterço.
    let returning = target == 0.0 || target * input.steer < 0.0;
    let rate = if returning {
        STEER_RELEASE
    } else {
        STEER_ATTACK * gain
    };
    let step = rate * time.delta_secs();
    input.steer += (target - input.steer).clamp(-step, step);

    if keys.just_pressed(KeyCode::KeyR) {
        respawn.write(RespawnRequested);
    }
}
