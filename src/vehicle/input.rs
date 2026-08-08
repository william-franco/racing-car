//! Traduz o teclado em comandos de direção para o carro do jogador.

use bevy::prelude::*;

use crate::core::state::{GameState, PlayerCar};

use super::physics::DriveInput;

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
    keys: Res<ButtonInput<KeyCode>>,
    mut respawn: MessageWriter<RespawnRequested>,
    mut player: Query<&mut DriveInput, With<PlayerCar>>,
) {
    let Ok(mut input) = player.single_mut() else {
        return;
    };

    let pressed = |codes: [KeyCode; 2]| codes.iter().any(|code| keys.pressed(*code)) as u8 as f32;

    input.throttle = pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    input.brake = pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    input.steer = pressed([KeyCode::KeyD, KeyCode::ArrowRight])
        - pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    input.handbrake = keys.pressed(KeyCode::Space);

    if keys.just_pressed(KeyCode::KeyR) {
        respawn.write(RespawnRequested);
    }
}
