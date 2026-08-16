//! Configurações do jogador e recorde de volta, persistidos em disco.
//!
//! Cada opção é um recurso próprio porque o menu usa um sistema genérico
//! `setting_button::<T>` que exige `T: Resource + Component`.

use std::fs;
use std::path::PathBuf;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// `#[derive(Resource)]` já implementa `Component`, então cada opção serve
// tanto como estado global quanto como marcador nos botões do menu.
#[derive(Resource, Debug, PartialEq, Eq, Clone, Copy, Default, Serialize, Deserialize)]
pub enum DisplayQuality {
    Low,
    #[default]
    Medium,
    High,
}

impl DisplayQuality {
    /// Amostras e ângulo de obturador do motion blur para cada preset.
    pub fn motion_blur(self) -> (u32, f32) {
        match self {
            DisplayQuality::Low => (1, 0.2),
            DisplayQuality::Medium => (4, 0.5),
            DisplayQuality::High => (8, 0.75),
        }
    }

    /// Quantidade de adereços decorativos espalhados pelo circuito.
    pub fn scenery_density(self) -> f32 {
        match self {
            DisplayQuality::Low => 0.35,
            DisplayQuality::Medium => 0.7,
            DisplayQuality::High => 1.0,
        }
    }

    pub fn shadows_enabled(self) -> bool {
        self != DisplayQuality::Low
    }
}

/// Exibição do contador de FPS.
#[derive(Resource, Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct ShowFps(pub bool);

impl Default for ShowFps {
    fn default() -> Self {
        Self(true)
    }
}

/// Sensibilidade da direção em cinco passos, do mais suave ao mais direto.
#[derive(Resource, Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct SteerSensitivity(pub u32);

impl SteerSensitivity {
    pub const MAX: u32 = 4;

    /// Multiplicador aplicado ao comando de direção, de 0.6x a 1.4x.
    pub fn multiplier(self) -> f32 {
        0.6 + self.0.min(Self::MAX) as f32 * 0.2
    }

    pub fn label(self) -> &'static str {
        match self.0.min(Self::MAX) {
            0 => "Mínima",
            1 => "Baixa",
            2 => "Padrão",
            3 => "Alta",
            _ => "Máxima",
        }
    }
}

impl Default for SteerSensitivity {
    fn default() -> Self {
        Self(2)
    }
}

/// Volume mestre em passos de 0 a 9, como no menu.
#[derive(Resource, Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct Volume(pub u32);

impl Volume {
    pub const MAX: u32 = 9;

    pub fn linear(self) -> f32 {
        self.0 as f32 / Self::MAX as f32
    }
}

impl Default for Volume {
    fn default() -> Self {
        Self(6)
    }
}

/// Número de voltas da corrida.
#[derive(Resource, Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct LapCount(pub u32);

impl Default for LapCount {
    fn default() -> Self {
        Self(3)
    }
}

/// Quantidade de adversários controlados pela IA.
#[derive(Resource, Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct OpponentCount(pub u32);

impl Default for OpponentCount {
    fn default() -> Self {
        Self(5)
    }
}

/// Recordes acumulados entre sessões.
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Records {
    /// Melhor volta em segundos.
    pub best_lap: Option<f32>,
    pub races_finished: u32,
}

/// Perfis gravados por versões anteriores não têm os campos mais novos, então
/// cada um cai no padrão em vez de invalidar o arquivo inteiro.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SaveFile {
    #[serde(default)]
    quality: DisplayQuality,
    #[serde(default)]
    show_fps: ShowFps,
    #[serde(default)]
    steer: SteerSensitivity,
    #[serde(default)]
    volume: Volume,
    #[serde(default)]
    laps: LapCount,
    #[serde(default)]
    opponents: OpponentCount,
    #[serde(default)]
    records: Records,
}

fn save_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
        })?;
    Some(base.join("racing-car").join("profile.ron"))
}

fn load() -> SaveFile {
    let Some(path) = save_path() else {
        return SaveFile::default();
    };
    let Ok(text) = fs::read_to_string(&path) else {
        return SaveFile::default();
    };
    match ron::from_str::<SaveFile>(&text) {
        Ok(save) => save,
        Err(error) => {
            warn!("perfil salvo em {} é inválido: {error}", path.display());
            SaveFile::default()
        }
    }
}

fn store(save: &SaveFile) {
    let Some(path) = save_path() else {
        return;
    };
    if let Some(parent) = path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        warn!("não foi possível criar {}: {error}", parent.display());
        return;
    }
    match ron::ser::to_string_pretty(save, ron::ser::PrettyConfig::default()) {
        Ok(text) => {
            if let Err(error) = fs::write(&path, text) {
                warn!("não foi possível salvar {}: {error}", path.display());
            }
        }
        Err(error) => warn!("não foi possível serializar o perfil: {error}"),
    }
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        let save = load();
        app.insert_resource(save.quality)
            .insert_resource(save.show_fps)
            .insert_resource(save.steer)
            .insert_resource(save.volume)
            .insert_resource(save.laps)
            .insert_resource(save.opponents)
            .insert_resource(save.records)
            .add_systems(PostUpdate, persist.run_if(settings_changed));
    }
}

fn settings_changed(
    quality: Res<DisplayQuality>,
    show_fps: Res<ShowFps>,
    steer: Res<SteerSensitivity>,
    volume: Res<Volume>,
    laps: Res<LapCount>,
    opponents: Res<OpponentCount>,
    records: Res<Records>,
) -> bool {
    // `is_added` filtra o primeiro quadro, em que tudo acabou de ser inserido.
    let touched = quality.is_changed()
        || show_fps.is_changed()
        || steer.is_changed()
        || volume.is_changed()
        || laps.is_changed()
        || opponents.is_changed()
        || records.is_changed();
    let fresh = quality.is_added()
        && show_fps.is_added()
        && steer.is_added()
        && volume.is_added()
        && laps.is_added()
        && opponents.is_added()
        && records.is_added();
    touched && !fresh
}

fn persist(
    quality: Res<DisplayQuality>,
    show_fps: Res<ShowFps>,
    steer: Res<SteerSensitivity>,
    volume: Res<Volume>,
    laps: Res<LapCount>,
    opponents: Res<OpponentCount>,
    records: Res<Records>,
) {
    store(&SaveFile {
        quality: *quality,
        show_fps: *show_fps,
        steer: *steer,
        volume: *volume,
        laps: *laps,
        opponents: *opponents,
        records: records.clone(),
    });
}
