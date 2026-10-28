mod city;

pub use city::forget as forget_tile;
mod dreams;
mod target;
mod tunnel;

pub use target::{Pixels, Target, over};
use target::pack;

pub fn fps(id: &str) -> f64 {
    match id {
        "city" => 20.0,
        "dreams" => 30.0,
        _ => 45.0,
    }
}

pub fn animated(id: &str) -> bool {
    matches!(id, "city" | "dreams" | "tunnel")
}

pub fn render(id: &str, target: &mut Target<'_>, seconds: f64, viewport: f64) {
    match id {
        "city" => city::draw(target, seconds),
        "dreams" => dreams::draw(target, seconds, viewport),
        "tunnel" => tunnel::draw(target, seconds),
        _ => {}
    }
}
