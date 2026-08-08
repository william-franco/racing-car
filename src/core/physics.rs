//! Integração com o Avian: camadas de colisão e parâmetros globais.

use avian3d::prelude::*;
use bevy::prelude::*;

/// Camadas de colisão do jogo.
///
/// O primeiro bit é reservado pelo Avian para a camada padrão, por isso
/// `Default` precisa ser a primeira variante.
#[derive(PhysicsLayer, Default, Clone, Copy, Debug)]
pub enum GameLayer {
    #[default]
    Default,
    /// Asfalto, escapes e o terreno — tudo em que as rodas podem apoiar.
    Ground,
    /// Chassi dos carros.
    Car,
    /// Guard-rails e obstáculos sólidos.
    Barrier,
    /// Sensores de checkpoint.
    Trigger,
}

impl GameLayer {
    pub fn ground() -> CollisionLayers {
        CollisionLayers::new(GameLayer::Ground, [GameLayer::Car])
    }

    pub fn barrier() -> CollisionLayers {
        CollisionLayers::new(GameLayer::Barrier, [GameLayer::Car])
    }

    pub fn car() -> CollisionLayers {
        CollisionLayers::new(
            GameLayer::Car,
            [
                GameLayer::Ground,
                GameLayer::Barrier,
                GameLayer::Car,
                GameLayer::Trigger,
            ],
        )
    }

    pub fn trigger() -> CollisionLayers {
        CollisionLayers::new(GameLayer::Trigger, [GameLayer::Car])
    }
}

/// Gravidade exagerada em relação à real: mantém o carro colado no chão e dá
/// o peso característico de um arcade de corrida.
pub const GRAVITY: f32 = 20.0;

pub struct GamePhysicsPlugin;

impl Plugin for GamePhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default())
            .insert_resource(Gravity(Vec3::NEG_Y * GRAVITY));
    }
}
