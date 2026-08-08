//! O traçado do circuito, gerado a partir de uma spline Catmull-Rom fechada.
//!
//! Tudo que precisa saber "onde fica a pista" — malha, guard-rails,
//! checkpoints, grid de largada, IA, minimapa e respawn — lê da mesma
//! [`TrackLayout`], reamostrada em intervalos constantes de comprimento de arco.

use bevy::math::ops;
use bevy::prelude::*;

/// Altura do plano da pista. O terreno fica logo abaixo.
pub const TRACK_Y: f32 = 0.0;

/// Distância entre duas amostras consecutivas, em metros.
const SAMPLE_SPACING: f32 = 2.0;

/// Subdivisões por segmento da spline ao construir a tabela de arco.
const DENSE_SUBDIVISIONS: usize = 48;

const BASE_HALF_WIDTH: f32 = 7.0;
const MAX_BANKING: f32 = 0.16;

/// Pontos de controle do circuito, em metros. O traçado é fechado: o último
/// ponto liga de volta ao primeiro.
///
/// A reta principal fica ao longo de `-Z`, com a linha de chegada na origem.
const CONTROL_POINTS: [Vec3; 23] = [
    Vec3::new(0.0, TRACK_Y, -190.0),
    Vec3::new(78.0, TRACK_Y, -186.0),
    Vec3::new(142.0, TRACK_Y, -158.0),
    Vec3::new(178.0, TRACK_Y, -104.0),
    Vec3::new(182.0, TRACK_Y, -44.0),
    Vec3::new(150.0, TRACK_Y, 2.0),
    Vec3::new(96.0, TRACK_Y, 12.0),
    Vec3::new(58.0, TRACK_Y, 44.0),
    Vec3::new(74.0, TRACK_Y, 92.0),
    Vec3::new(126.0, TRACK_Y, 118.0),
    Vec3::new(152.0, TRACK_Y, 168.0),
    Vec3::new(124.0, TRACK_Y, 214.0),
    Vec3::new(62.0, TRACK_Y, 226.0),
    Vec3::new(4.0, TRACK_Y, 208.0),
    Vec3::new(-34.0, TRACK_Y, 166.0),
    Vec3::new(-30.0, TRACK_Y, 116.0),
    Vec3::new(10.0, TRACK_Y, 86.0),
    Vec3::new(-16.0, TRACK_Y, 44.0),
    Vec3::new(-86.0, TRACK_Y, 34.0),
    Vec3::new(-146.0, TRACK_Y, -4.0),
    Vec3::new(-168.0, TRACK_Y, -70.0),
    Vec3::new(-140.0, TRACK_Y, -142.0),
    Vec3::new(-76.0, TRACK_Y, -184.0),
];

/// Uma seção transversal da pista.
#[derive(Clone, Copy, Debug)]
pub struct TrackSample {
    /// Centro da pista, sobre o plano do asfalto.
    pub center: Vec3,
    /// Direção de avanço, normalizada e horizontal.
    pub tangent: Vec3,
    /// Perpendicular horizontal apontando para a direita de quem corre.
    pub right: Vec3,
    /// Normal da superfície, já considerando a inclinação lateral.
    pub up: Vec3,
    pub half_width: f32,
    /// Inclinação lateral em radianos (positiva inclina para a esquerda).
    pub banking: f32,
    /// Curvatura com sinal, em 1/m. Positiva quando a pista vira à direita.
    pub curvature: f32,
    /// Distância acumulada desde a linha de chegada.
    pub distance: f32,
}

impl TrackSample {
    /// Vetor lateral que acompanha a inclinação da pista.
    pub fn banked_right(&self) -> Vec3 {
        Quat::from_axis_angle(self.tangent, self.banking) * self.right
    }

    pub fn edge(&self, lateral: f32) -> Vec3 {
        self.center + self.banked_right() * lateral
    }

    pub fn left_edge(&self) -> Vec3 {
        self.edge(-self.half_width)
    }

    pub fn right_edge(&self) -> Vec3 {
        self.edge(self.half_width)
    }

    /// Rotação que alinha `-Z` com a direção de avanço e `Y` com a normal.
    pub fn orientation(&self) -> Quat {
        Quat::from_mat3(&Mat3::from_cols(
            self.banked_right(),
            self.up,
            -self.tangent,
        ))
    }
}

/// O circuito reamostrado em intervalos constantes de comprimento de arco.
#[derive(Resource, Debug)]
pub struct TrackLayout {
    samples: Vec<TrackSample>,
    length: f32,
}

impl TrackLayout {
    /// Constrói o traçado a partir dos pontos de controle do circuito.
    pub fn circuit() -> Self {
        Self::from_control_points(&CONTROL_POINTS)
    }

    pub fn from_control_points(points: &[Vec3]) -> Self {
        let curve = CubicCardinalSpline::new(0.5, points.to_vec())
            .to_curve_cyclic()
            .expect("o circuito precisa de pelo menos dois pontos de controle");

        // Amostragem densa só para medir o comprimento de arco com precisão.
        let dense: Vec<Vec3> = curve
            .iter_positions(points.len() * DENSE_SUBDIVISIONS)
            .collect();
        let dense = dedup_consecutive(dense);

        let mut cumulative = Vec::with_capacity(dense.len() + 1);
        let mut total = 0.0;
        cumulative.push(0.0);
        for i in 0..dense.len() {
            total += dense[i].distance(dense[(i + 1) % dense.len()]);
            cumulative.push(total);
        }

        // Reamostra em passos iguais para que índice e distância andem juntos.
        let count = (total / SAMPLE_SPACING).round().max(16.0) as usize;
        let spacing = total / count as f32;
        let centers: Vec<Vec3> = (0..count)
            .map(|i| resample(&dense, &cumulative, i as f32 * spacing))
            .collect();

        let mut samples = Vec::with_capacity(count);
        for i in 0..count {
            let previous = centers[(i + count - 1) % count];
            let next = centers[(i + 1) % count];
            let tangent = (next - previous).with_y(0.0).normalize_or(Vec3::NEG_Z);
            let right = tangent.cross(Vec3::Y).normalize_or(Vec3::X);

            samples.push(TrackSample {
                center: centers[i],
                tangent,
                right,
                up: Vec3::Y,
                half_width: BASE_HALF_WIDTH,
                banking: 0.0,
                curvature: 0.0,
                distance: i as f32 * spacing,
            });
        }

        compute_curvature(&mut samples, spacing);
        smooth_curvature(&mut samples);
        apply_profile(&mut samples);

        Self {
            samples,
            length: total,
        }
    }

    pub fn samples(&self) -> &[TrackSample] {
        &self.samples
    }

    /// Comprimento total do circuito, em metros.
    pub fn length(&self) -> f32 {
        self.length
    }

    pub fn spacing(&self) -> f32 {
        self.length / self.samples.len() as f32
    }

    pub fn wrap_index(&self, index: i32) -> usize {
        let count = self.samples.len() as i32;
        (((index % count) + count) % count) as usize
    }

    pub fn sample(&self, index: i32) -> &TrackSample {
        &self.samples[self.wrap_index(index)]
    }

    /// Normaliza uma distância para o intervalo `[0, length)`.
    pub fn wrap_distance(&self, distance: f32) -> f32 {
        distance.rem_euclid(self.length)
    }

    /// Índice da amostra mais próxima de uma distância percorrida.
    ///
    /// Como as amostras são igualmente espaçadas em comprimento de arco, isso
    /// é uma divisão — não precisa de busca.
    pub fn index_at_distance(&self, distance: f32) -> usize {
        let step = (self.wrap_distance(distance) / self.spacing()).round() as i32;
        self.wrap_index(step)
    }

    /// Interpola a seção transversal na distância pedida.
    pub fn sample_at_distance(&self, distance: f32) -> TrackSample {
        let spacing = self.spacing();
        let position = self.wrap_distance(distance) / spacing;
        let index = position.floor() as i32;
        let t = position - index as f32;

        let a = self.sample(index);
        let b = self.sample(index + 1);

        TrackSample {
            center: a.center.lerp(b.center, t),
            tangent: a.tangent.lerp(b.tangent, t).normalize_or(a.tangent),
            right: a.right.lerp(b.right, t).normalize_or(a.right),
            up: a.up.lerp(b.up, t).normalize_or(Vec3::Y),
            half_width: a.half_width.lerp(b.half_width, t),
            banking: a.banking.lerp(b.banking, t),
            curvature: a.curvature.lerp(b.curvature, t),
            distance: self.wrap_distance(distance),
        }
    }

    /// Índice da amostra mais próxima de `position`.
    ///
    /// Com um `hint` (o índice do quadro anterior) a busca fica local e custa
    /// algumas dezenas de comparações; sem ele, varre o circuito inteiro.
    pub fn nearest_index(&self, position: Vec3, hint: Option<usize>) -> usize {
        let mut best = hint.unwrap_or(0);
        let mut best_distance = f32::MAX;
        let consider = |index: usize, best: &mut usize, best_distance: &mut f32| {
            let distance = self.samples[index].center.distance_squared(position);
            if distance < *best_distance {
                *best_distance = distance;
                *best = index;
            }
        };

        match hint {
            // A janela cobre bem mais do que um carro anda entre dois quadros.
            Some(hint) => {
                for offset in -24..=24i32 {
                    let index = self.wrap_index(hint as i32 + offset);
                    consider(index, &mut best, &mut best_distance);
                }
            }
            None => {
                for index in 0..self.samples.len() {
                    consider(index, &mut best, &mut best_distance);
                }
            }
        }

        best
    }

    /// Distância percorrida e desvio lateral de um ponto qualquer.
    ///
    /// O desvio é negativo à esquerda do eixo da pista e positivo à direita.
    pub fn locate(&self, position: Vec3, hint: Option<usize>) -> TrackLocation {
        let index = self.nearest_index(position, hint);
        let sample = self.samples[index];
        let offset = position - sample.center;
        let along = offset.dot(sample.tangent);

        TrackLocation {
            index,
            distance: self.wrap_distance(sample.distance + along),
            lateral: offset.dot(sample.right),
            sample,
        }
    }
}

/// Resultado de projetar um ponto do mundo sobre o traçado.
#[derive(Clone, Copy, Debug)]
pub struct TrackLocation {
    pub index: usize,
    pub distance: f32,
    pub lateral: f32,
    pub sample: TrackSample,
}

impl TrackLocation {
    /// Quanto o ponto está fora dos limites do asfalto, em metros.
    pub fn off_track(&self) -> f32 {
        (self.lateral.abs() - self.sample.half_width).max(0.0)
    }
}

fn dedup_consecutive(points: Vec<Vec3>) -> Vec<Vec3> {
    let mut result: Vec<Vec3> = Vec::with_capacity(points.len());
    for point in points {
        if result.last().is_none_or(|last| last.distance(point) > 1e-4) {
            result.push(point);
        }
    }
    // A curva cíclica repete o ponto inicial no fim.
    if result.len() > 1
        && let (Some(first), Some(last)) = (result.first().copied(), result.last().copied())
        && first.distance(last) < 1e-4
    {
        result.pop();
    }
    result
}

/// Encontra o ponto a `target` metros do início, andando pela tabela de arco.
fn resample(dense: &[Vec3], cumulative: &[f32], target: f32) -> Vec3 {
    // `partition_point` devolve o primeiro índice cuja distância excede o alvo.
    let upper = cumulative.partition_point(|&d| d <= target).max(1);
    let lower = upper - 1;

    let span = cumulative[upper] - cumulative[lower];
    let t = if span > f32::EPSILON {
        (target - cumulative[lower]) / span
    } else {
        0.0
    };

    let a = dense[lower % dense.len()];
    let b = dense[upper % dense.len()];
    a.lerp(b, t)
}

/// Curvatura com sinal a partir da variação angular da tangente.
fn compute_curvature(samples: &mut [TrackSample], spacing: f32) {
    let count = samples.len();
    for i in 0..count {
        let previous = samples[(i + count - 1) % count].tangent;
        let next = samples[(i + 1) % count].tangent;

        // Ângulo com sinal entre as tangentes vizinhas, medido no plano XZ.
        let sin = next.cross(previous).dot(Vec3::Y);
        let cos = next.dot(previous);
        let angle = ops::atan2(sin, cos);

        samples[i].curvature = angle / (2.0 * spacing);
    }
}

/// Suaviza a curvatura para que largura e inclinação não oscilem.
fn smooth_curvature(samples: &mut [TrackSample]) {
    let count = samples.len();
    const PASSES: usize = 6;

    for _ in 0..PASSES {
        let source: Vec<f32> = samples.iter().map(|sample| sample.curvature).collect();
        for i in 0..count {
            let previous = source[(i + count - 1) % count];
            let next = source[(i + 1) % count];
            samples[i].curvature = 0.25 * previous + 0.5 * source[i] + 0.25 * next;
        }
    }
}

/// Deriva largura, inclinação e normal a partir da curvatura já suavizada.
fn apply_profile(samples: &mut [TrackSample]) {
    for sample in samples.iter_mut() {
        // 1/40 m⁻¹ corresponde a uma curva bem fechada.
        let intensity = (sample.curvature.abs() * 40.0).clamp(0.0, 1.0);

        // Numa curva à direita a borda esquerda é a externa, e é ela que sobe.
        sample.half_width = BASE_HALF_WIDTH + 2.5 * (1.0 - intensity);
        sample.banking = sample.curvature.signum() * intensity * MAX_BANKING;
        sample.up = Quat::from_axis_angle(sample.tangent, sample.banking) * Vec3::Y;
    }
}
