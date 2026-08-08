//! Som de motor sintetizado em tempo real.
//!
//! Em vez de tocar uma amostra gravada e mexer no `speed` do sink — o que
//! soa metálico —, o motor é gerado amostra a amostra: uma serra somada a
//! harmônicos e a um pouco de ruído, com a frequência acompanhando o RPM.
//!
//! O sistema de física e a thread de áudio conversam por um par de valores
//! atômicos, que é o único jeito seguro de atravessar essa fronteira.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use bevy::audio::{ChannelCount, Decodable, SampleRate, Source};
use bevy::math::ops;
use bevy::prelude::*;
use bevy::reflect::TypePath;

const SAMPLE_RATE: u32 = 44_100;

/// Frequência de rotação mais baixa que o motor produz, em Hz.
const IDLE_HZ: f32 = 34.0;

/// Frequência no corte de giro.
const REDLINE_HZ: f32 = 168.0;

/// Canal compartilhado entre o jogo e a thread de áudio.
///
/// Os dois valores são guardados como bits de `f32` num `AtomicU32`, o que
/// evita qualquer trava no caminho crítico do áudio.
#[derive(Resource, Clone, Debug, Default)]
pub struct EngineSignal {
    inner: Arc<EngineSignalInner>,
}

#[derive(Debug)]
struct EngineSignalInner {
    /// Rotação normalizada em 0..1.
    rpm: AtomicU32,
    /// Amplitude final, já com o volume das configurações aplicado.
    gain: AtomicU32,
}

impl Default for EngineSignalInner {
    fn default() -> Self {
        Self {
            rpm: AtomicU32::new(0.0f32.to_bits()),
            gain: AtomicU32::new(0.0f32.to_bits()),
        }
    }
}

impl EngineSignal {
    pub fn set(&self, rpm_fraction: f32, gain: f32) {
        self.inner
            .rpm
            .store(rpm_fraction.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
        self.inner
            .gain
            .store(gain.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    fn read(&self) -> (f32, f32) {
        (
            f32::from_bits(self.inner.rpm.load(Ordering::Relaxed)),
            f32::from_bits(self.inner.gain.load(Ordering::Relaxed)),
        )
    }
}

/// Fonte de áudio do motor, registrada como asset.
#[derive(Asset, TypePath, Clone)]
pub struct EngineAudio {
    signal: EngineSignal,
}

impl EngineAudio {
    pub fn new(signal: EngineSignal) -> Self {
        Self { signal }
    }
}

pub struct EngineDecoder {
    signal: EngineSignal,
    /// Fase de cada harmônico, em voltas (0..1).
    phases: [f32; 3],
    /// Frequência suavizada, para a troca de marcha não estalar.
    frequency: f32,
    gain: f32,
    noise: u32,
}

impl EngineDecoder {
    fn new(signal: EngineSignal) -> Self {
        Self {
            signal,
            phases: [0.0; 3],
            frequency: IDLE_HZ,
            gain: 0.0,
            noise: 0x1234_5678,
        }
    }

    /// Ruído branco barato, para dar textura ao escapamento.
    fn white(&mut self) -> f32 {
        self.noise ^= self.noise << 13;
        self.noise ^= self.noise >> 17;
        self.noise ^= self.noise << 5;
        (self.noise as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

impl Iterator for EngineDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let (rpm, gain) = self.signal.read();

        // Suavização por amostra: a 44,1 kHz isso é uma rampa de poucos ms.
        let target = IDLE_HZ + (REDLINE_HZ - IDLE_HZ) * rpm;
        self.frequency += (target - self.frequency) * 0.0006;
        self.gain += (gain - self.gain) * 0.0008;

        // Harmônicos ímpares dão o ronco encorpado de um motor a combustão.
        const HARMONICS: [f32; 3] = [1.0, 2.0, 3.0];
        const WEIGHTS: [f32; 3] = [0.55, 0.3, 0.15];

        let mut sample = 0.0;
        for (index, (multiple, weight)) in HARMONICS.iter().zip(WEIGHTS).enumerate() {
            self.phases[index] =
                (self.phases[index] + self.frequency * multiple / SAMPLE_RATE as f32) % 1.0;
            // Serra suavizada: dente de serra puro soa áspero demais.
            let phase = self.phases[index];
            let saw = 2.0 * phase - 1.0;
            let smooth = ops::sin(std::f32::consts::TAU * phase);
            sample += weight * (0.65 * saw + 0.35 * smooth);
        }

        let breath = self.white() * 0.05 * (0.4 + rpm);
        Some((sample * 0.35 + breath) * self.gain)
    }
}

impl Source for EngineDecoder {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        ChannelCount::new(1).expect("mono é um número de canais válido")
    }

    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(SAMPLE_RATE).expect("44,1 kHz é uma taxa válida")
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Decodable for EngineAudio {
    type Decoder = EngineDecoder;

    fn decoder(&self) -> Self::Decoder {
        EngineDecoder::new(self.signal.clone())
    }
}
