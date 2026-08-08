//! Estados globais e componentes marcadores usados por vários domínios.

use bevy::prelude::*;

/// Estado principal da aplicação. Cada tela registra suas entidades com
/// `DespawnOnExit(...)`, então a troca de estado já limpa tudo sozinha.
#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum GameState {
    #[default]
    Splash,
    Menu,
    Racing,
}

/// Fase interna da corrida. Só é diferente de `Inactive` dentro de
/// [`GameState::Racing`].
#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum RacePhase {
    #[default]
    Inactive,
    /// Carros parados no grid enquanto os semáforos apagam.
    Countdown,
    /// Corrida liberada.
    Green,
    /// Jogador cruzou a última volta; a tela de resultados está no ar.
    Finished,
}

#[derive(Component)]
pub struct PlayerCar;

#[derive(Component)]
pub struct AiCar;

/// Câmera 3D usada durante a corrida.
#[derive(Component)]
pub struct GameCamera;

/// Câmera 2D usada por splash, menu e resultados.
#[derive(Component)]
pub struct MenuCamera;

/// Coeficiente de aderência da superfície atingida pelo raycast das rodas.
/// Asfalto ≈ 1.0, escape ≈ 0.5, grama ≈ 0.3.
#[derive(Component, Clone, Copy, Debug)]
pub struct SurfaceGrip(pub f32);

/// Retrato do carro do jogador publicado uma vez por passo de física.
///
/// Câmera, HUD, áudio e minimapa leem daqui em vez de consultar o carro
/// diretamente, o que mantém esses sistemas independentes do módulo `vehicle`.
#[derive(Resource, Debug, Clone, Copy)]
pub struct PlayerSnapshot {
    pub translation: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    /// Velocidade escalar à frente, em m/s (negativa em marcha a ré).
    pub speed: f32,
    pub rpm: f32,
    pub gear: i32,
    pub throttle: f32,
    /// Quanto o carro está deslizando lateralmente, normalizado em 0..1.
    pub drift: f32,
    pub grounded: bool,
}

impl PlayerSnapshot {
    pub fn speed_kmh(&self) -> f32 {
        self.speed.abs() * 3.6
    }
}

impl Default for PlayerSnapshot {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            velocity: Vec3::ZERO,
            speed: 0.0,
            rpm: 0.0,
            gear: 1,
            throttle: 0.0,
            drift: 0.0,
            grounded: false,
        }
    }
}

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_state::<RacePhase>()
            .init_resource::<PlayerSnapshot>()
            .add_systems(OnExit(GameState::Racing), reset_race_phase);
    }
}

fn reset_race_phase(mut phase: ResMut<NextState<RacePhase>>) {
    phase.set(RacePhase::Inactive);
}
