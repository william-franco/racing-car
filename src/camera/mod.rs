//! Câmera de corrida com três modos e motion blur por objeto.
//!
//! O modo perseguição usa um braço elástico: o alvo é calculado a partir da
//! pose do carro e a câmera converge para ele com suavização exponencial, que
//! é independente da taxa de quadros. É isso que dá a sensação de peso sem
//! introduzir tremor.

use bevy::camera::Hdr;
use bevy::post_process::motion_blur::MotionBlur;
use bevy::prelude::*;

use crate::core::settings::DisplayQuality;
use crate::core::state::{GameCamera, GameState, PlayerSnapshot, RacePhase};
use crate::vehicle::physics::SnapshotSystems;
use crate::world::track::TrackLayout;

/// Posição do braço da câmera de perseguição, em espaço do carro.
const CHASE_OFFSET: Vec3 = Vec3::new(0.0, 2.5, 7.4);

/// Ponto para onde a câmera olha, à frente do carro.
const CHASE_LOOK_AHEAD: f32 = 8.0;

/// Ponto de vista do piloto, logo acima da ponta do capô.
///
/// Sentar a câmera no lugar da cabeça deixaria a própria carroceria ocupando
/// quase metade da tela, já que o carro é um volume sólido sem interior
/// modelado. À frente do capô sobra só um naco do bico como referência.
const COCKPIT_OFFSET: Vec3 = Vec3::new(0.0, 0.62, -1.8);

const BASE_FOV: f32 = 1.05;

/// Distância entre as torres de câmera espalhadas pelo circuito.
const BROADCAST_SPACING: f32 = 110.0;

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraMode {
    #[default]
    Chase,
    Cockpit,
    /// Torre fixa na beira da pista, como uma câmera de transmissão.
    Broadcast,
}

impl CameraMode {
    fn next(self) -> Self {
        match self {
            CameraMode::Chase => CameraMode::Cockpit,
            CameraMode::Cockpit => CameraMode::Broadcast,
            CameraMode::Broadcast => CameraMode::Chase,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            CameraMode::Chase => "Perseguição",
            CameraMode::Cockpit => "Cockpit",
            CameraMode::Broadcast => "Transmissão",
        }
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraMode>()
            .add_systems(OnEnter(GameState::Racing), spawn_camera)
            .add_systems(
                Update,
                (cycle_mode, tune_motion_blur).run_if(in_state(GameState::Racing)),
            )
            .add_systems(
                PostUpdate,
                follow_car
                    .after(SnapshotSystems)
                    .before(TransformSystems::Propagate)
                    .run_if(in_state(GameState::Racing)),
            );
    }
}

fn spawn_camera(
    mut commands: Commands,
    quality: Res<DisplayQuality>,
    mut mode: ResMut<CameraMode>,
) {
    *mode = CameraMode::Chase;
    let (samples, shutter_angle) = quality.motion_blur();

    commands.spawn((
        DespawnOnExit(GameState::Racing),
        Camera3d::default(),
        // Renderiza em HDR: o motion blur e as luzes emissivas do carro ficam
        // corretos sem estourar o branco.
        Hdr,
        Projection::Perspective(PerspectiveProjection {
            fov: BASE_FOV,
            near: 0.12,
            far: 4000.0,
            ..default()
        }),
        MotionBlur {
            shutter_angle,
            samples,
        },
        Transform::default(),
        GameCamera,
    ));
}

fn cycle_mode(keys: Res<ButtonInput<KeyCode>>, mut mode: ResMut<CameraMode>) {
    if keys.just_pressed(KeyCode::KeyC) {
        *mode = mode.next();
    }
}

fn tune_motion_blur(
    keys: Res<ButtonInput<KeyCode>>,
    mut blur: Single<&mut MotionBlur, With<GameCamera>>,
) {
    if keys.just_pressed(KeyCode::Digit1) {
        blur.shutter_angle -= 0.25;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        blur.shutter_angle += 0.25;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        blur.samples = blur.samples.saturating_sub(1);
    }
    if keys.just_pressed(KeyCode::Digit4) {
        blur.samples += 1;
    }

    blur.shutter_angle = blur.shutter_angle.clamp(0.0, 1.0);
    blur.samples = blur.samples.clamp(0, 32);
}

#[allow(clippy::type_complexity)]
fn follow_car(
    time: Res<Time>,
    mode: Res<CameraMode>,
    phase: Res<State<RacePhase>>,
    snapshot: Res<PlayerSnapshot>,
    layout: Option<Res<TrackLayout>>,
    mut camera: Query<(&mut Transform, &mut Projection), With<GameCamera>>,
) {
    let Ok((mut transform, mut projection)) = camera.single_mut() else {
        return;
    };

    let car = snapshot.translation;

    // Só a guinada do carro entra no braço da câmera: rolagem e arfagem
    // ficariam enjoativas na perseguição.
    let forward = (snapshot.rotation * Vec3::NEG_Z)
        .with_y(0.0)
        .normalize_or(Vec3::NEG_Z);
    let yaw = Quat::from_rotation_arc(Vec3::NEG_Z, forward);

    let speed_factor = (snapshot.speed.abs() / 70.0).clamp(0.0, 1.0);

    let (target, look_at, fov) = match *mode {
        CameraMode::Chase => {
            // A câmera recua e abaixa um pouco conforme a velocidade sobe, e
            // desliza para o lado de fora quando o carro está de través.
            let drift_swing = snapshot.velocity.dot(snapshot.rotation * Vec3::X) * 0.12;
            let offset = CHASE_OFFSET
                + Vec3::new(0.0, -0.35, 1.6) * speed_factor
                + Vec3::new(drift_swing.clamp(-2.2, 2.2), 0.0, 0.0);
            (
                car + yaw * offset,
                car + forward * CHASE_LOOK_AHEAD + Vec3::Y * 1.2,
                BASE_FOV + 0.22 * speed_factor,
            )
        }
        CameraMode::Cockpit => (
            car + snapshot.rotation * COCKPIT_OFFSET,
            // O piloto olha na horizontal, na altura dos próprios olhos.
            car + snapshot.rotation * (COCKPIT_OFFSET + Vec3::NEG_Z * 30.0),
            BASE_FOV + 0.1 * speed_factor,
        ),
        CameraMode::Broadcast => {
            let post = layout
                .as_deref()
                .map(|layout| broadcast_post(layout, car))
                .unwrap_or_else(|| car + Vec3::new(18.0, 11.0, 18.0));
            (post, car, 0.5)
        }
    };

    // Suavização exponencial: `1 - e^(-k·dt)` converge igual em qualquer FPS.
    let stiffness = match *mode {
        // No cockpit a câmera é rígida, senão parece que a cabeça flutua.
        CameraMode::Cockpit => 60.0,
        // A transmissão corta de uma torre para a outra em vez de voar entre
        // elas, que é como uma câmera de TV se comporta.
        CameraMode::Broadcast => 1000.0,
        CameraMode::Chase if *phase.get() == RacePhase::Green => 9.0,
        _ => 4.0,
    };
    let blend = 1.0 - (-stiffness * time.delta_secs()).exp();

    transform.translation = transform.translation.lerp(target, blend);
    transform.look_at(look_at, Vec3::Y);

    if let Projection::Perspective(perspective) = &mut *projection {
        perspective.fov = perspective.fov.lerp(fov, blend);
    }
}

/// Torre de câmera mais próxima do carro, alternando de lado a cada uma.
fn broadcast_post(layout: &TrackLayout, car: Vec3) -> Vec3 {
    let location = layout.locate(car, None);
    let post = (location.distance / BROADCAST_SPACING).round();
    let sample = layout.sample_at_distance(layout.wrap_distance(post * BROADCAST_SPACING));

    let side = if post as i32 % 2 == 0 { 1.0 } else { -1.0 };
    sample.center + sample.right * (side * (sample.half_width + 16.0)) + Vec3::Y * 8.0
}
