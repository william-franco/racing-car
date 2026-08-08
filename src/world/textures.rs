//! Texturas geradas em runtime, para o jogo não depender de arquivos de imagem.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Ruído determinístico e barato, no intervalo `0..1`.
fn noise(x: u32, y: u32, seed: u32) -> f32 {
    let mut hash = x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263) ^ seed;
    hash = (hash ^ (hash >> 13)).wrapping_mul(1_274_126_177);
    ((hash ^ (hash >> 16)) & 0xFFFF) as f32 / 65_535.0
}

fn build(size: u32, repeat: bool, mut pixel: impl FnMut(u32, u32) -> [u8; 4]) -> Image {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            data.extend_from_slice(&pixel(x, y));
        }
    }

    let mut image = Image::new_fill(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );

    if repeat {
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            mag_filter: ImageFilterMode::Linear,
            ..ImageSamplerDescriptor::linear()
        });
    }

    image
}

/// Asfalto: cinza escuro granulado, com pontos mais claros de brita.
pub fn asphalt() -> Image {
    build(128, true, |x, y| {
        let grain = noise(x, y, 7);
        let patch = noise(x / 16, y / 16, 91);
        let base = 46.0 + grain * 26.0 + patch * 10.0;
        let value = base.clamp(0.0, 255.0) as u8;
        [value, value, (value as f32 * 1.04).min(255.0) as u8, 255]
    })
}

/// Grama: verde irregular com manchas mais escuras.
pub fn grass() -> Image {
    build(128, true, |x, y| {
        let grain = noise(x, y, 23);
        let patch = noise(x / 8, y / 8, 57);
        let shade = 0.72 + grain * 0.2 + patch * 0.16;
        [
            (54.0 * shade).clamp(0.0, 255.0) as u8,
            (108.0 * shade).clamp(0.0, 255.0) as u8,
            (46.0 * shade).clamp(0.0, 255.0) as u8,
            255,
        ]
    })
}

/// Brita das áreas de escape.
pub fn gravel() -> Image {
    build(128, true, |x, y| {
        let grain = noise(x, y, 131);
        let shade = 0.8 + grain * 0.35;
        [
            (150.0 * shade).clamp(0.0, 255.0) as u8,
            (140.0 * shade).clamp(0.0, 255.0) as u8,
            (120.0 * shade).clamp(0.0, 255.0) as u8,
            255,
        ]
    })
}
