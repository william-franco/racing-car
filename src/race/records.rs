//! Recordes do jogador, guardados junto das configurações.

use bevy::prelude::*;

use crate::core::settings::Records;
use crate::core::state::RacePhase;

/// Registra uma volta do jogador, promovendo-a a recorde se for a melhor.
///
/// É chamada por `run_system_cached_with` no momento em que a volta fecha.
pub fn record_lap(In(lap_time): In<f32>, mut records: ResMut<Records>) {
    let improved = records.best_lap.is_none_or(|best| lap_time < best);
    if improved {
        records.best_lap = Some(lap_time);
    }
}

/// Conta uma corrida concluída.
pub fn count_finished_race(mut records: ResMut<Records>) {
    records.races_finished += 1;
}

pub struct RecordsPlugin;

impl Plugin for RecordsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(RacePhase::Finished), count_finished_race);
    }
}

/// Formata um tempo como `m:ss.mmm`, o padrão de painéis de corrida.
pub fn format_lap_time(seconds: f32) -> String {
    let minutes = (seconds / 60.0).floor() as u32;
    let remainder = seconds - minutes as f32 * 60.0;
    format!("{minutes}:{remainder:06.3}")
}
