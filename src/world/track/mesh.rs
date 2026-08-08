//! Geração das malhas da pista: asfalto, faixas, zebras e acostamento.
//!
//! Todas as superfícies são "fitas": duas bordas que acompanham o traçado e
//! são costuradas com quadriláteros. As coordenadas UV usam o comprimento de
//! arco, então as texturas nunca esticam nas curvas.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;

use super::spline::{TrackLayout, TrackSample};

/// Altura do terreno em volta da pista.
pub const GROUND_Y: f32 = -0.35;

/// Espessura da zebra medida a partir da borda do asfalto.
pub const KERB_WIDTH: f32 = 1.8;

/// Largura do acostamento (escape) fora das zebras.
pub const VERGE_WIDTH: f32 = 12.0;

/// Comprimento de cada bloco de zebra, em metros.
const KERB_BLOCK: f32 = 4.0;

/// A partir desta curvatura a zebra aparece.
const KERB_CURVATURE: f32 = 0.006;

/// Uma borda da fita, resolvida em espaço de mundo.
fn edge(sample: &TrackSample, lateral: f32, lift: f32) -> Vec3 {
    sample.center + sample.banked_right() * lateral + Vec3::Y * lift
}

/// Costura uma fita fechada ao longo de todo o circuito.
///
/// `edges` devolve o par (borda esquerda, borda direita) de cada seção.
fn ribbon<F>(layout: &TrackLayout, uv_scale: Vec2, mut edges: F) -> Mesh
where
    F: FnMut(&TrackSample) -> (Vec3, Vec3),
{
    let samples = layout.samples();
    let count = samples.len();

    let mut positions = Vec::with_capacity((count + 1) * 2);
    let mut uvs = Vec::with_capacity((count + 1) * 2);
    let mut indices = Vec::with_capacity(count * 6);

    // A seção inicial é repetida no fim para que a UV não volte a zero
    // exatamente na emenda do circuito.
    for step in 0..=count {
        let sample = &samples[step % count];
        let (left, right) = edges(sample);
        let along = if step == count {
            layout.length()
        } else {
            sample.distance
        };

        positions.push(left);
        positions.push(right);
        uvs.push([0.0, along * uv_scale.y]);
        uvs.push([uv_scale.x, along * uv_scale.y]);
    }

    for step in 0..count {
        let base = (step * 2) as u32;
        indices.extend_from_slice(&[base, base + 1, base + 3, base, base + 3, base + 2]);
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
    .with_computed_smooth_normals()
}

/// Costura blocos soltos, sem continuidade entre eles.
///
/// Usado nas zebras, em que cada bloco é uma cor diferente.
fn blocks(quads: &[[Vec3; 4]]) -> Mesh {
    let mut positions = Vec::with_capacity(quads.len() * 4);
    let mut uvs = Vec::with_capacity(quads.len() * 4);
    let mut indices = Vec::with_capacity(quads.len() * 6);

    for (block, corners) in quads.iter().enumerate() {
        let base = (block * 4) as u32;
        positions.extend_from_slice(corners);
        uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]]);
        indices.extend_from_slice(&[base, base + 1, base + 3, base, base + 3, base + 2]);
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
    .with_computed_smooth_normals()
}

/// O asfalto, seguindo a largura e a inclinação de cada seção.
pub fn asphalt(layout: &TrackLayout) -> Mesh {
    ribbon(layout, Vec2::new(4.0, 0.06), |sample| {
        (
            edge(sample, -sample.half_width, 0.0),
            edge(sample, sample.half_width, 0.0),
        )
    })
}

/// Faixa contínua branca junto a uma das bordas.
pub fn edge_line(layout: &TrackLayout, side: f32) -> Mesh {
    ribbon(layout, Vec2::new(1.0, 0.25), |sample| {
        let outer = side * (sample.half_width - 0.15);
        let inner = side * (sample.half_width - 0.45);
        (
            edge(sample, inner.min(outer), 0.012),
            edge(sample, inner.max(outer), 0.012),
        )
    })
}

/// Acostamento que devolve o nível da pista ao do terreno.
pub fn verge(layout: &TrackLayout, side: f32) -> Mesh {
    ribbon(layout, Vec2::new(3.0, 0.05), |sample| {
        let inner_lateral = side * sample.half_width;
        let outer_lateral = side * (sample.half_width + VERGE_WIDTH);

        let inner = edge(sample, inner_lateral, -0.01);
        // A borda externa aterrissa no nível do terreno, desfazendo a
        // inclinação da pista aos poucos.
        let mut outer = edge(sample, outer_lateral, 0.0);
        outer.y = GROUND_Y + 0.03;

        if side < 0.0 {
            (outer, inner)
        } else {
            (inner, outer)
        }
    })
}

/// Zebras vermelhas e brancas alternadas, só nas curvas.
///
/// Devolve `(blocos vermelhos, blocos brancos)`.
pub fn kerbs(layout: &TrackLayout, side: f32) -> (Mesh, Mesh) {
    let samples = layout.samples();
    let count = samples.len();
    let spacing = layout.spacing();
    let per_block = (KERB_BLOCK / spacing).round().max(1.0) as usize;

    let mut red = Vec::new();
    let mut white = Vec::new();

    let mut step = 0;
    let mut parity = 0;
    while step < count {
        let end = (step + per_block).min(count);
        let in_corner = (step..end).any(|i| samples[i].curvature.abs() > KERB_CURVATURE);

        if in_corner {
            let target = if parity % 2 == 0 {
                &mut red
            } else {
                &mut white
            };
            for i in step..end {
                let a = &samples[i];
                let b = &samples[(i + 1) % count];
                let inner = |sample: &TrackSample| edge(sample, side * sample.half_width, 0.02);
                let outer = |sample: &TrackSample| {
                    edge(sample, side * (sample.half_width + KERB_WIDTH), -0.02)
                };

                // Os vértices precisam ir da esquerda para a direita para que
                // a normal do quadrilátero aponte para cima.
                if side < 0.0 {
                    target.push([outer(a), inner(a), outer(b), inner(b)]);
                } else {
                    target.push([inner(a), outer(a), inner(b), outer(b)]);
                }
            }
            parity += 1;
        } else {
            parity = 0;
        }

        step = end;
    }

    (blocks(&red), blocks(&white))
}

/// Quadriculado da linha de chegada, atravessando a pista na distância zero.
pub fn start_line(layout: &TrackLayout) -> (Mesh, Mesh) {
    const COLUMNS: usize = 12;
    const ROWS: usize = 2;
    const DEPTH: f32 = 3.0;

    let mut dark = Vec::new();
    let mut light = Vec::new();

    for row in 0..ROWS {
        let near = layout.wrap_distance(-DEPTH * 0.5 + row as f32 * DEPTH / ROWS as f32);
        let far = layout.wrap_distance(-DEPTH * 0.5 + (row + 1) as f32 * DEPTH / ROWS as f32);
        let near_sample = layout.sample_at_distance(near);
        let far_sample = layout.sample_at_distance(far);

        for column in 0..COLUMNS {
            let t0 = column as f32 / COLUMNS as f32 * 2.0 - 1.0;
            let t1 = (column + 1) as f32 / COLUMNS as f32 * 2.0 - 1.0;

            let quad = [
                edge(&near_sample, t0 * near_sample.half_width, 0.015),
                edge(&near_sample, t1 * near_sample.half_width, 0.015),
                edge(&far_sample, t0 * far_sample.half_width, 0.015),
                edge(&far_sample, t1 * far_sample.half_width, 0.015),
            ];

            if (row + column) % 2 == 0 {
                dark.push(quad);
            } else {
                light.push(quad);
            }
        }
    }

    (blocks(&dark), blocks(&light))
}
