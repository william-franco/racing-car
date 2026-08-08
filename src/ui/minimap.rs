//! Minimapa: o traçado é rasterizado uma vez numa textura e os carros são
//! pontos de UI posicionados sobre ela.
//!
//! Desenhar a pista numa imagem custa uma vez só, na entrada da corrida, em
//! vez de milhares de linhas de gizmo por quadro.

use avian3d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::core::state::{GameState, PlayerCar};
use crate::vehicle::CarIdentity;
use crate::world::track::TrackLayout;

use super::theme;

/// Lado da textura do minimapa, em pixels.
const TEXTURE_SIZE: u32 = 320;

/// Lado do minimapa na tela, em pixels de UI.
const WIDGET_SIZE: f32 = 220.0;

const DOT_SIZE: f32 = 9.0;

/// Converte coordenadas do mundo (XZ) em coordenadas do minimapa (0..1).
#[derive(Resource, Clone, Copy, Debug)]
pub struct MinimapProjection {
    origin: Vec2,
    scale: f32,
}

impl MinimapProjection {
    fn normalized(&self, world: Vec3) -> Vec2 {
        (Vec2::new(world.x, world.z) - self.origin) * self.scale
    }
}

#[derive(Component)]
struct MinimapFrame;

/// Ponto que representa um carro; guarda a entidade que ele segue.
#[derive(Component)]
struct MinimapDot(Entity);

pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Racing), spawn_minimap)
            .add_systems(
                Update,
                (spawn_dots, move_dots)
                    .chain()
                    .run_if(in_state(GameState::Racing)),
            );
    }
}

fn spawn_minimap(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    layout: Res<TrackLayout>,
) {
    let (projection, texture) = render_track(&layout);
    commands.insert_resource(projection);

    commands.spawn((
        DespawnOnExit(GameState::Racing),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(16),
            left: px(16),
            width: px(WIDGET_SIZE),
            height: px(WIDGET_SIZE),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(8)),
            ..default()
        },
        BackgroundColor(theme::PANEL_DEEP.with_alpha(0.72)),
        BorderColor::all(theme::BORDER.with_alpha(0.7)),
        Pickable::IGNORE,
        MinimapFrame,
        children![(
            ImageNode::new(images.add(texture)),
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
        )],
    ));
}

/// Rasteriza o traçado numa textura quadrada e devolve a projeção usada.
fn render_track(layout: &TrackLayout) -> (MinimapProjection, Image) {
    let samples = layout.samples();

    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for sample in samples {
        let point = Vec2::new(sample.center.x, sample.center.z);
        min = min.min(point);
        max = max.max(point);
    }

    // Um quadrado com folga para o traçado não encostar na moldura.
    let span = (max - min).max_element().max(1.0) * 1.1;
    let center = (min + max) * 0.5;
    let origin = center - Vec2::splat(span * 0.5);
    let projection = MinimapProjection {
        origin,
        scale: 1.0 / span,
    };

    let size = TEXTURE_SIZE as usize;
    let mut pixels = vec![0u8; size * size * 4];

    let mut plot = |point: Vec2, radius: i32, color: [u8; 4]| {
        let x = (point.x * TEXTURE_SIZE as f32) as i32;
        let y = (point.y * TEXTURE_SIZE as f32) as i32;

        for offset_y in -radius..=radius {
            for offset_x in -radius..=radius {
                if offset_x * offset_x + offset_y * offset_y > radius * radius {
                    continue;
                }
                let (px, py) = (x + offset_x, y + offset_y);
                if px < 0 || py < 0 || px >= size as i32 || py >= size as i32 {
                    continue;
                }
                let index = (py as usize * size + px as usize) * 4;
                pixels[index..index + 4].copy_from_slice(&color);
            }
        }
    };

    const ASPHALT: [u8; 4] = [190, 194, 205, 255];
    const START: [u8; 4] = [250, 168, 33, 255];

    // Interpola entre amostras para a linha não sair pontilhada.
    for index in 0..samples.len() {
        let a = projection.normalized(samples[index].center);
        let b = projection.normalized(samples[(index + 1) % samples.len()].center);
        let steps = ((b - a).length() * TEXTURE_SIZE as f32).ceil().max(1.0) as i32;

        for step in 0..=steps {
            plot(a.lerp(b, step as f32 / steps as f32), 3, ASPHALT);
        }
    }

    plot(projection.normalized(samples[0].center), 6, START);

    let image = Image::new(
        Extent3d {
            width: TEXTURE_SIZE,
            height: TEXTURE_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );

    (projection, image)
}

/// Cria um ponto para cada carro que ainda não tem um.
fn spawn_dots(
    mut commands: Commands,
    frame: Single<Entity, With<MinimapFrame>>,
    cars: Query<(Entity, Has<PlayerCar>), With<CarIdentity>>,
    dots: Query<&MinimapDot>,
) {
    let tracked: Vec<Entity> = dots.iter().map(|dot| dot.0).collect();

    for (car, is_player) in &cars {
        if tracked.contains(&car) {
            continue;
        }

        let (color, size) = if is_player {
            (theme::ACCENT, DOT_SIZE + 3.0)
        } else {
            (theme::TEXT, DOT_SIZE)
        };

        commands.spawn((
            ChildOf(*frame),
            MinimapDot(car),
            Node {
                position_type: PositionType::Absolute,
                width: px(size),
                height: px(size),
                border_radius: BorderRadius::all(px(size * 0.5)),
                ..default()
            },
            BackgroundColor(color),
            Pickable::IGNORE,
        ));
    }
}

fn move_dots(
    projection: Option<Res<MinimapProjection>>,
    cars: Query<&Position>,
    mut dots: Query<(&MinimapDot, &mut Node, &mut Visibility)>,
) {
    let Some(projection) = projection else {
        return;
    };

    for (dot, mut node, mut visibility) in &mut dots {
        let Ok(position) = cars.get(dot.0) else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let point = projection.normalized(position.0) * WIDGET_SIZE;
        node.left = px(point.x - DOT_SIZE * 0.5);
        node.top = px(point.y - DOT_SIZE * 0.5);
        *visibility = Visibility::Inherited;
    }
}
