//! Paleta, tipografia e fábricas de widget compartilhadas pela interface.
//!
//! O tema é grafite com âmbar: escuro o bastante para não competir com a
//! pista e com um destaque quente que remete a painel de box.

use bevy::asset::AssetId;
use bevy::prelude::*;
use bevy::text::Font;

/// A fonte embutida no Bevy é um subconjunto sem acentuação, então ela é
/// substituída pela Fira Sans logo na inicialização. Trocar o asset padrão faz
/// todo o texto do jogo — inclusive o overlay de FPS — usar a fonte nova sem
/// precisar carregar um handle em cada widget.
pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, install_default_font);
    }
}

fn install_default_font(mut fonts: ResMut<Assets<Font>>) {
    let path = crate::asset_root().join("fonts/FiraSans-Bold.ttf");
    match std::fs::read(&path) {
        Ok(bytes) => {
            if let Err(error) = fonts.insert(AssetId::default(), Font::from_bytes(bytes)) {
                warn!("não foi possível instalar a fonte padrão: {error}");
            }
        }
        Err(error) => warn!("não foi possível ler {}: {error}", path.display()),
    }
}

pub const BACKDROP: Color = Color::srgb(0.055, 0.062, 0.078);
pub const PANEL: Color = Color::srgb(0.101, 0.112, 0.136);
pub const PANEL_DEEP: Color = Color::srgb(0.071, 0.079, 0.098);
pub const BORDER: Color = Color::srgb(0.19, 0.205, 0.245);

pub const ACCENT: Color = Color::srgb(0.98, 0.66, 0.13);
pub const ACCENT_DEEP: Color = Color::srgb(0.42, 0.27, 0.05);
pub const TEXT: Color = Color::srgb(0.90, 0.91, 0.93);
pub const TEXT_DIM: Color = Color::srgb(0.60, 0.63, 0.68);
pub const DANGER: Color = Color::srgb(0.93, 0.28, 0.24);
pub const SUCCESS: Color = Color::srgb(0.36, 0.86, 0.47);

pub const BUTTON_IDLE: Color = Color::srgb(0.145, 0.158, 0.19);
pub const BUTTON_HOVER: Color = Color::srgb(0.22, 0.24, 0.29);
pub const BUTTON_PRESSED: Color = ACCENT_DEEP;
pub const BUTTON_SELECTED: Color = Color::srgb(0.58, 0.39, 0.08);

/// Tela cheia, com o conteúdo centralizado.
pub fn screen() -> Node {
    Node {
        width: percent(100),
        height: percent(100),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

/// Painel com moldura, usado como caixa de qualquer tela do menu.
pub fn panel() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::axes(px(46), px(30)),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(10)),
            row_gap: px(6),
            ..default()
        },
        BackgroundColor(PANEL),
        BorderColor::all(BORDER),
    )
}

pub fn button_node() -> Node {
    Node {
        width: px(320),
        height: px(58),
        margin: UiRect::all(px(8)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        border: UiRect::all(px(2)),
        border_radius: BorderRadius::all(px(6)),
        ..default()
    }
}

/// Botão pequeno, para escolher entre valores de uma mesma opção.
pub fn chip_node(width: f32) -> Node {
    Node {
        width: px(width),
        height: px(52),
        margin: UiRect::all(px(5)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        border: UiRect::all(px(2)),
        border_radius: BorderRadius::all(px(6)),
        ..default()
    }
}

pub fn button_visuals() -> impl Bundle {
    (BackgroundColor(BUTTON_IDLE), BorderColor::all(BORDER))
}

pub fn text(size: f32, color: Color) -> impl Bundle + Clone {
    (
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

pub fn button_text() -> impl Bundle + Clone {
    text(28.0, TEXT)
}

pub fn heading() -> impl Bundle + Clone {
    text(58.0, ACCENT)
}

pub fn subheading() -> impl Bundle + Clone {
    text(34.0, TEXT)
}

pub fn caption() -> impl Bundle + Clone {
    text(20.0, TEXT_DIM)
}

/// Faixa quadriculada que enfeita o topo e o rodapé dos painéis.
pub fn checkered_strip(squares: usize) -> impl Bundle {
    let cells: Vec<_> = (0..squares)
        .map(|index| {
            (
                Node {
                    width: px(18),
                    height: px(18),
                    ..default()
                },
                BackgroundColor(if index % 2 == 0 {
                    Color::srgb(0.93, 0.93, 0.93)
                } else {
                    Color::srgb(0.08, 0.08, 0.09)
                }),
            )
        })
        .collect();

    (
        Node {
            flex_direction: FlexDirection::Row,
            margin: UiRect::vertical(px(14)),
            ..default()
        },
        Children::spawn(bevy::ecs::spawn::SpawnIter(cells.into_iter())),
    )
}
