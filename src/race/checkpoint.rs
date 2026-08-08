//! Checkpoints: portais invisíveis que validam a volta.
//!
//! Sem eles daria para cortar caminho e ainda assim contar a volta. Como só
//! contam quando cruzados em ordem, o carro precisa mesmo percorrer o traçado.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::core::physics::GameLayer;
use crate::core::state::GameState;
use crate::world::track::TrackLayout;

/// Quantidade de portais distribuídos pelo circuito. O de índice 0 é a
/// linha de chegada.
pub const CHECKPOINT_COUNT: usize = 16;

/// Altura do portal, generosa o bastante para pegar carros no ar.
const CHECKPOINT_HEIGHT: f32 = 6.0;

/// Espessura do portal ao longo da pista.
const CHECKPOINT_DEPTH: f32 = 0.5;

#[derive(Component, Clone, Copy, Debug)]
pub struct Checkpoint {
    pub index: usize,
    pub half_width: f32,
}

pub fn spawn_checkpoints(mut commands: Commands, layout: Res<TrackLayout>) {
    let step = layout.length() / CHECKPOINT_COUNT as f32;

    for index in 0..CHECKPOINT_COUNT {
        let distance = layout.wrap_distance(index as f32 * step);
        let sample = layout.sample_at_distance(distance);
        // Um pouco mais largo que o asfalto para pegar quem passa pela grama.
        let half_width = sample.half_width + 6.0;

        commands.spawn((
            DespawnOnExit(GameState::Racing),
            Checkpoint { index, half_width },
            Transform::from_translation(sample.center + Vec3::Y * (CHECKPOINT_HEIGHT * 0.4))
                .with_rotation(sample.orientation()),
            RigidBody::Static,
            Collider::cuboid(half_width * 2.0, CHECKPOINT_HEIGHT, CHECKPOINT_DEPTH),
            Sensor,
            CollisionEventsEnabled,
            GameLayer::trigger(),
        ));
    }
}
