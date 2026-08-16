//! Modelo de veículo por raycast: um único corpo rígido para o chassi e
//! quatro rodas resolvidas com traçado de raios.
//!
//! Cada roda lança um raio para baixo, mede a compressão da suspensão e
//! aplica três forças no ponto de contato: mola/amortecedor na vertical,
//! tração ou frenagem na longitudinal e aderência na lateral. Transferência
//! de peso, saltos e drift saem naturalmente dessa composição, sem precisar
//! simular a curva de deslizamento real do pneu.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::core::physics::GameLayer;
use crate::core::state::{PlayerCar, PlayerSnapshot, RacePhase, SurfaceGrip};

/// Índices das rodas: dianteira esquerda, dianteira direita, traseira
/// esquerda, traseira direita.
pub const WHEEL_COUNT: usize = 4;

/// Parâmetros de dirigibilidade de um carro.
#[derive(Component, Clone, Debug)]
pub struct CarConfig {
    pub mass: f32,
    /// Comprimento da suspensão totalmente estendida, em metros.
    pub suspension_rest: f32,
    /// Constante da mola, em N/m.
    pub spring_strength: f32,
    /// Constante do amortecedor, em N·s/m.
    pub damper_strength: f32,
    /// Rigidez da barra estabilizadora, em N/m de diferença entre as rodas.
    pub anti_roll: f32,
    pub wheel_radius: f32,
    /// Esterçamento máximo das rodas dianteiras, em radianos.
    pub max_steer: f32,
    /// Quanto o esterçamento fecha em velocidade alta, de 0 a 1.
    pub steer_falloff: f32,
    /// Velocidade em que o esterçamento chega ao fechamento total, em m/s.
    ///
    /// Separada da `top_speed` porque a corrida acontece bem abaixo dela: medir
    /// o fechamento contra o teto de velocidade deixava a direção solta demais
    /// justamente na faixa em que o carro anda.
    pub steer_reference_speed: f32,
    /// Força de tração total com o acelerador no fundo, em N.
    pub engine_force: f32,
    pub brake_force: f32,
    /// Velocidade máxima em m/s.
    pub top_speed: f32,
    pub reverse_speed: f32,
    /// Fração da velocidade lateral anulada por passo, de 0 a 1.
    pub front_grip: f32,
    pub rear_grip: f32,
    /// Aderência traseira com o freio de mão acionado.
    pub handbrake_grip: f32,
    /// Downforce em N por (m/s)².
    pub downforce: f32,
    pub rolling_resistance: f32,
    pub gear_ratios: Vec<f32>,
    pub final_drive: f32,
    pub idle_rpm: f32,
    pub max_rpm: f32,
    pub shift_up_rpm: f32,
    pub shift_down_rpm: f32,
}

impl Default for CarConfig {
    fn default() -> Self {
        Self {
            mass: 1150.0,
            suspension_rest: 0.55,
            spring_strength: 46_000.0,
            damper_strength: 4_400.0,
            anti_roll: 12_000.0,
            wheel_radius: 0.36,
            max_steer: 0.42,
            steer_falloff: 0.85,
            steer_reference_speed: 45.0,
            engine_force: 15_500.0,
            brake_force: 20_000.0,
            top_speed: 78.0,
            reverse_speed: 14.0,
            front_grip: 0.92,
            rear_grip: 0.8,
            handbrake_grip: 0.14,
            downforce: 5.5,
            rolling_resistance: 340.0,
            gear_ratios: vec![3.45, 2.35, 1.72, 1.32, 1.05, 0.86],
            final_drive: 3.4,
            idle_rpm: 950.0,
            max_rpm: 8200.0,
            shift_up_rpm: 7400.0,
            shift_down_rpm: 3100.0,
        }
    }
}

/// Comandos brutos de direção, preenchidos pelo teclado ou pela IA.
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct DriveInput {
    pub throttle: f32,
    pub brake: f32,
    /// Negativo vira à esquerda, positivo à direita.
    pub steer: f32,
    pub handbrake: bool,
}

/// Estado derivado do carro, consumido por HUD, áudio e câmera.
#[derive(Component, Clone, Copy, Debug)]
pub struct EngineState {
    pub rpm: f32,
    pub gear: i32,
    /// Velocidade escalar à frente em m/s; negativa em marcha a ré.
    pub speed: f32,
    /// Acelerador já suavizado.
    pub throttle: f32,
    pub brake: f32,
    pub steer: f32,
    /// Ângulo de deriva normalizado em 0..1.
    pub drift: f32,
    pub wheels_on_ground: u32,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            rpm: 950.0,
            // Ponto morto: o câmbio engata sozinho quando o carro sai do lugar.
            gear: 0,
            speed: 0.0,
            throttle: 0.0,
            brake: 0.0,
            steer: 0.0,
            drift: 0.0,
            wheels_on_ground: 0,
        }
    }
}

impl EngineState {
    pub fn rpm_fraction(&self, config: &CarConfig) -> f32 {
        ((self.rpm - config.idle_rpm) / (config.max_rpm - config.idle_rpm)).clamp(0.0, 1.0)
    }
}

/// Aponta o chassi para as entidades das suas rodas.
#[derive(Component, Clone, Copy, Debug)]
pub struct CarChassis {
    pub wheels: [Entity; WHEEL_COUNT],
}

/// Uma roda presa ao chassi. Não é um corpo rígido: só um ponto de raycast.
#[derive(Component, Clone, Copy, Debug)]
pub struct Wheel {
    pub index: usize,
    /// Ponto de fixação no espaço local do chassi.
    pub anchor: Vec3,
    pub radius: f32,
    pub steered: bool,
    pub powered: bool,
}

impl Wheel {
    pub fn is_front(&self) -> bool {
        self.index < 2
    }
}

/// Resultado do raycast e do modelo de pneu, por roda.
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct WheelState {
    pub grounded: bool,
    /// Comprimento atual da suspensão, em metros.
    pub suspension_length: f32,
    /// Compressão normalizada em 0..1.
    pub compression: f32,
    pub contact: Vec3,
    pub normal: Vec3,
    pub surface_grip: f32,
    /// Ângulo de rotação acumulado da roda, em radianos.
    pub spin: f32,
    pub steer: f32,
    /// Deslizamento lateral em m/s.
    pub slip: f32,
}

/// Momento em que a física do carro roda, antes do passo do Avian.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VehicleSystems;

/// Publicação do [`PlayerSnapshot`]. Quem consome o estado do carro do
/// jogador — câmera, HUD, áudio — deve rodar depois deste conjunto.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SnapshotSystems;

pub struct VehiclePhysicsPlugin;

impl Plugin for VehiclePhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (gate_input_during_countdown, vehicle_dynamics)
                .chain()
                .in_set(VehicleSystems),
        )
        .add_systems(
            PostUpdate,
            publish_player_snapshot
                .in_set(SnapshotSystems)
                .before(TransformSystems::Propagate),
        )
        .add_systems(Update, animate_wheels);
    }
}

/// No grid os carros ficam travados no freio até a largada.
fn gate_input_during_countdown(phase: Res<State<RacePhase>>, mut inputs: Query<&mut DriveInput>) {
    if *phase.get() == RacePhase::Green {
        return;
    }

    for mut input in &mut inputs {
        input.throttle = 0.0;
        input.steer = 0.0;
        input.brake = 1.0;
        input.handbrake = true;
    }
}

/// Curva de torque normalizada: cheia no meio da faixa e caindo nas pontas.
///
/// O piso é generoso de propósito. Num carro de verdade quem tira o veículo da
/// inércia é a primeira marcha somada à embreagem; aqui, sem esse estágio, um
/// piso baixo deixava o carro engasgado por vários segundos depois de qualquer
/// toque que o fizesse parar.
fn torque_curve(rpm_fraction: f32) -> f32 {
    let x = rpm_fraction.clamp(0.0, 1.0);
    // Parábola com pico em torno de 65% da faixa útil.
    (1.0 - 1.9 * (x - 0.65).powi(2)).clamp(0.45, 1.0)
}

#[allow(clippy::type_complexity)]
fn vehicle_dynamics(
    time: Res<Time>,
    spatial: SpatialQuery,
    surfaces: Query<&SurfaceGrip>,
    mut cars: Query<(
        Entity,
        &CarConfig,
        &DriveInput,
        &mut EngineState,
        &CarChassis,
        &ComputedCenterOfMass,
        Forces,
    )>,
    mut wheels: Query<(&Wheel, &mut WheelState)>,
) {
    let dt = time.delta_secs();
    if dt <= f32::EPSILON {
        return;
    }

    for (entity, config, input, mut engine, chassis, center_of_mass, mut forces) in &mut cars {
        let position = forces.position().0;
        let rotation = forces.rotation().0;
        let linear = forces.linear_velocity();
        let angular = forces.angular_velocity();
        let com = position + rotation * center_of_mass.0;

        let up = rotation * Vec3::Y;
        let forward = rotation * Vec3::NEG_Z;
        let right = rotation * Vec3::X;

        let velocity_at = |point: Vec3| linear + angular.cross(point - com);

        // --- Estado do motor -------------------------------------------------
        let forward_speed = linear.dot(forward);
        let lateral_speed = linear.dot(right);
        engine.speed = forward_speed;
        engine.throttle = engine.throttle.lerp(input.throttle, (dt * 9.0).min(1.0));
        engine.brake = engine.brake.lerp(input.brake, (dt * 14.0).min(1.0));

        let steer_limit = 1.0
            - config.steer_falloff
                * (forward_speed.abs() / config.steer_reference_speed).clamp(0.0, 1.0);
        let steer_target = input.steer.clamp(-1.0, 1.0) * config.max_steer * steer_limit;
        engine.steer = engine.steer.lerp(steer_target, (dt * 8.0).min(1.0));

        update_gearbox(config, &mut engine, forward_speed, input.handbrake);

        // Em ré o próprio pedal do freio acelera, então ele não pode frear ao
        // mesmo tempo.
        let brake_pedal = if engine.gear < 0 { 0.0 } else { engine.brake };

        // Ângulo entre para onde o carro aponta e para onde de fato anda.
        engine.drift = if linear.length() > 4.0 {
            (lateral_speed.abs() / linear.length().max(1.0)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // --- Suspensão e pneus ----------------------------------------------
        let filter = SpatialQueryFilter::from_mask([GameLayer::Ground, GameLayer::Barrier])
            .with_excluded_entities([entity]);
        let ray_direction = Dir3::new(-up).unwrap_or(Dir3::NEG_Y);
        let max_distance = config.suspension_rest + config.wheel_radius;

        // Primeira passada: só o raycast, para a barra estabilizadora poder
        // comparar as duas rodas de um mesmo eixo antes de aplicar forças.
        let mut contacts: [Option<WheelContact>; WHEEL_COUNT] = [None; WHEEL_COUNT];
        let mut anchors = [Vec3::ZERO; WHEEL_COUNT];

        for &wheel_entity in &chassis.wheels {
            let Ok((wheel, mut state)) = wheels.get_mut(wheel_entity) else {
                continue;
            };

            let anchor = position + rotation * wheel.anchor;
            anchors[wheel.index] = anchor;

            let hit = spatial.cast_ray(anchor, ray_direction, max_distance, true, &filter);
            state.steer = if wheel.steered { engine.steer } else { 0.0 };

            match hit {
                Some(hit) => {
                    let length = (hit.distance - wheel.radius).clamp(0.0, config.suspension_rest);
                    let grip = surfaces.get(hit.entity).map_or(1.0, |surface| surface.0);

                    state.grounded = true;
                    state.suspension_length = length;
                    state.compression = 1.0 - length / config.suspension_rest;
                    state.contact = anchor - up * hit.distance;
                    state.normal = hit.normal;
                    state.surface_grip = grip;

                    contacts[wheel.index] = Some(WheelContact {
                        wheel: *wheel,
                        contact: state.contact,
                        normal: hit.normal,
                        compression: state.compression,
                        surface_grip: grip,
                    });
                }
                None => {
                    state.grounded = false;
                    state.suspension_length = config.suspension_rest;
                    state.compression = 0.0;
                    state.surface_grip = 0.0;
                    state.slip = 0.0;
                }
            }
        }

        engine.wheels_on_ground = contacts.iter().filter(|slot| slot.is_some()).count() as u32;

        // Barra estabilizadora: transfere carga entre as rodas do mesmo eixo,
        // o que reduz a rolagem em curva sem endurecer a suspensão.
        let anti_roll = [
            axle_roll(&contacts, 0, 1, config.anti_roll),
            axle_roll(&contacts, 2, 3, config.anti_roll),
        ];

        let powered_count = contacts
            .iter()
            .flatten()
            .filter(|contact| contact.wheel.powered)
            .count()
            .max(1) as f32;

        let drive_force = engine_force(config, &engine, forward_speed) / powered_count;
        let quarter_mass = config.mass * 0.25;

        for slot in contacts.iter().flatten() {
            let wheel = slot.wheel;
            let contact = slot.contact;
            let normal = slot.normal;

            // Mola e amortecedor ao longo da normal do chassi, não da
            // superfície: é o eixo em que a suspensão realmente trabalha.
            let anchor = anchors[wheel.index];
            let compression_metres = config.suspension_rest * slot.compression;
            let vertical_speed = velocity_at(anchor).dot(up);
            let axle = if wheel.is_front() { 0 } else { 1 };
            let roll = anti_roll[axle] * if wheel.index % 2 == 0 { 1.0 } else { -1.0 };

            let suspension = (config.spring_strength * compression_metres
                - config.damper_strength * vertical_speed
                + roll)
                .max(0.0);
            forces.apply_force_at_point(up * suspension, contact);

            // Eixos do pneu projetados no plano de contato. Esterçar à direita
            // é uma rotação negativa em torno de Y.
            let steer = if wheel.steered { engine.steer } else { 0.0 };
            let steer_rotation = Quat::from_axis_angle(up, -steer);
            let wheel_forward = project(steer_rotation * forward, normal);
            let wheel_right = project(steer_rotation * right, normal);
            if wheel_forward == Vec3::ZERO || wheel_right == Vec3::ZERO {
                continue;
            }

            let contact_velocity = velocity_at(contact);
            let lateral = contact_velocity.dot(wheel_right);
            let longitudinal = contact_velocity.dot(wheel_forward);

            // Carga sobre o pneu, normalizada em torno de 1 no repouso.
            let load = (suspension / (config.mass * 5.0)).clamp(0.2, 1.6);

            let base_grip = if wheel.is_front() {
                config.front_grip
            } else if input.handbrake {
                config.handbrake_grip
            } else {
                config.rear_grip
            };

            let grip = (base_grip * slot.surface_grip * load).clamp(0.0, 0.98);
            let correction = -lateral * grip;
            // O impulso sobe em direção à altura do centro de massa antes de
            // ser aplicado. No ponto de contato o braço de alavanca é o carro
            // inteiro e uma curva forte capota; subindo, sobra transferência de
            // peso sem o tombo.
            let grip_point = contact.lerp(contact.with_y(com.y), 0.6);
            forces
                .apply_linear_impulse_at_point(wheel_right * correction * quarter_mass, grip_point);

            if wheel.powered && !input.handbrake {
                forces
                    .apply_force_at_point(wheel_forward * drive_force * slot.surface_grip, contact);
            }

            // Frenagem e resistência ao rolamento agem contra o movimento, mas
            // nunca com mais força do que a necessária para parar a roda neste
            // passo — do contrário o carro sairia andando para trás.
            if longitudinal.abs() > 0.01 {
                let braking = if input.handbrake && !wheel.is_front() {
                    config.brake_force * 0.28
                } else {
                    config.brake_force * 0.25 * brake_pedal
                };
                let demand = braking + config.rolling_resistance * 0.25;
                let ceiling = longitudinal.abs() * quarter_mass / dt;
                let retarding = demand.min(ceiling) * longitudinal.signum();
                forces.apply_force_at_point(-wheel_forward * retarding, contact);
            }

            if let Ok((_, mut state)) = wheels.get_mut(chassis.wheels[wheel.index]) {
                state.slip = lateral;
                // A roda gira conforme o avanço real do ponto de contato.
                state.spin += longitudinal * dt / wheel.radius;
            }
        }

        // --- Estabilização ---------------------------------------------------
        let speed = linear.length();
        if engine.wheels_on_ground > 0 {
            forces.apply_force(-up * config.downforce * speed * speed);
        } else {
            // No ar, alinha o carro com a horizontal para não capotar no pouso.
            let tilt = up.cross(Vec3::Y);
            forces.apply_torque(tilt * config.mass * 2.4 - angular * config.mass * 0.5);
        }
    }
}

/// Contato resolvido de uma roda, guardado entre as duas passadas.
#[derive(Clone, Copy)]
struct WheelContact {
    wheel: Wheel,
    contact: Vec3,
    normal: Vec3,
    compression: f32,
    surface_grip: f32,
}

/// Força da barra estabilizadora de um eixo, positiva quando a roda esquerda
/// está mais comprimida que a direita.
fn axle_roll(
    contacts: &[Option<WheelContact>; WHEEL_COUNT],
    left: usize,
    right: usize,
    stiffness: f32,
) -> f32 {
    let compression = |index: usize| {
        contacts[index]
            .map(|contact| contact.compression)
            .unwrap_or(0.0)
    };
    (compression(right) - compression(left)) * stiffness
}

/// Projeta um vetor no plano de contato e normaliza.
fn project(vector: Vec3, normal: Vec3) -> Vec3 {
    (vector - normal * vector.dot(normal)).normalize_or_zero()
}

/// Tração disponível, já limitada pela velocidade máxima.
fn engine_force(config: &CarConfig, engine: &EngineState, forward_speed: f32) -> f32 {
    let reverse = engine.gear < 0;
    // Andando de ré é o freio que faz as vezes de acelerador.
    let demand = if reverse {
        engine.brake
    } else {
        engine.throttle
    };
    if demand <= 0.001 || engine.gear == 0 {
        return 0.0;
    }

    let limit = if reverse {
        config.reverse_speed
    } else {
        config.top_speed
    };

    // O corte é suave para o carro não bater num teto duro de velocidade.
    let headroom = (1.0 - (forward_speed.abs() / limit).clamp(0.0, 1.0)).powf(0.7);
    let direction = if reverse { -1.0 } else { 1.0 };

    direction * config.engine_force * demand * torque_curve(engine.rpm_fraction(config)) * headroom
}

/// Marcha automática. Parado e sem pedal o câmbio fica em ponto morto; insistir
/// no freio engata a ré, e daí em diante é o próprio freio que acelera para trás.
fn update_gearbox(config: &CarConfig, engine: &mut EngineState, forward_speed: f32, held: bool) {
    let top_gear = config.gear_ratios.len() as i32;
    let stopped = forward_speed.abs() < 0.6;
    let idle_pedals = engine.throttle < 0.05 && engine.brake < 0.05;

    if engine.gear < 0 {
        // Acelerar tira da ré, desde que o carro não esteja indo para trás.
        if engine.throttle > 0.05 && forward_speed > -0.6 {
            engine.gear = 1;
        }
    } else if stopped && idle_pedals {
        engine.gear = 0;
    } else if engine.gear == 0 && engine.throttle > 0.05 {
        engine.gear = 1;
    } else if forward_speed.abs() < 0.35 && !held && engine.brake > 0.5 && engine.throttle < 0.05 {
        engine.gear = -1;
    }

    let ratio = if engine.gear < 0 {
        config.gear_ratios[0]
    } else {
        config.gear_ratios[(engine.gear.max(1) - 1) as usize]
    };

    // Rotação a partir da velocidade das rodas, passando pela transmissão.
    let wheel_turns_per_second =
        forward_speed.abs() / (std::f32::consts::TAU * config.wheel_radius);
    let rpm = wheel_turns_per_second * ratio * config.final_drive * 60.0;
    engine.rpm = rpm.clamp(config.idle_rpm, config.max_rpm);

    if engine.gear > 0 {
        if engine.rpm > config.shift_up_rpm && engine.gear < top_gear {
            engine.gear += 1;
        } else if engine.rpm < config.shift_down_rpm && engine.gear > 1 {
            engine.gear -= 1;
        }
    }
}

/// Posiciona as rodas conforme a suspensão e as faz girar e esterçar.
fn animate_wheels(
    cars: Query<(&CarConfig, &CarChassis)>,
    mut wheels: Query<(&Wheel, &WheelState, &mut Transform)>,
) {
    for (config, chassis) in &cars {
        for &entity in &chassis.wheels {
            let Ok((wheel, state, mut transform)) = wheels.get_mut(entity) else {
                continue;
            };

            let drop = if state.grounded {
                state.suspension_length
            } else {
                config.suspension_rest
            };

            // Esterço em torno do eixo vertical, giro em torno do eixo da roda.
            transform.translation = wheel.anchor - Vec3::Y * drop;
            transform.rotation =
                Quat::from_rotation_y(-state.steer) * Quat::from_rotation_x(-state.spin);
        }
    }
}

/// Publica o estado do carro do jogador para os demais domínios.
fn publish_player_snapshot(
    mut snapshot: ResMut<PlayerSnapshot>,
    player: Query<(&Transform, &LinearVelocity, &EngineState), With<PlayerCar>>,
) {
    let Ok((transform, velocity, engine)) = player.single() else {
        return;
    };

    *snapshot = PlayerSnapshot {
        translation: transform.translation,
        rotation: transform.rotation,
        velocity: velocity.0,
        speed: engine.speed,
        rpm: engine.rpm,
        gear: engine.gear,
        throttle: engine.throttle,
        drift: engine.drift,
        grounded: engine.wheels_on_ground > 0,
    };
}
