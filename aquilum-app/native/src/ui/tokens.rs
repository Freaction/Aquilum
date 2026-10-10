//! Токены дизайн-системы, сгенерированные `build.rs` из CSS-токенов фронтенда
//! (`src/styles/tokens/*.css`, `src/styles/themes/dark.css`). Размеры — в логических пикселях
//! при масштабе интерфейса 1 (1rem = 16 px); масштаб применяется ко всему окну сразу.
//!
//! Это вся дизайн-система целиком: интерфейс использует её по мере переноса.
#![allow(dead_code)]

use masonry::peniko::Color;

/// Тень из токенов (`box-shadow`): смещение, размытие, растяжение — в логических пикселях.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub x: f64,
    pub y: f64,
    pub blur: f64,
    pub spread: f64,
    pub color: Color,
}

fn accent_mix(base: [f32; 4], weights: [f32; 3], accent: [[f32; 4]; 3]) -> Color {
    let mut premultiplied = base;
    for (weight, color) in weights.iter().zip(accent) {
        for (channel, value) in premultiplied.iter_mut().zip([color[0] * color[3], color[1] * color[3], color[2] * color[3], color[3]]) {
            *channel += weight * value;
        }
    }
    let alpha = premultiplied[3].clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return Color::TRANSPARENT;
    }
    let channel = |i: usize| (premultiplied[i] / alpha).clamp(0.0, 1.0);
    Color::new([channel(0), channel(1), channel(2), alpha])
}

include!(concat!(env!("OUT_DIR"), "/tokens.rs"));
