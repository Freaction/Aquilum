mod build;
mod item;
mod layer;
mod place;
mod shadow;
mod sub;
#[cfg(test)]
mod tests;

use super::icons;

pub use layer::Menu;
pub use shadow::{box_shadow, shadow_reach};
pub use sub::{Submenu, Swatch};

#[derive(Debug)]
pub enum MenuChoice {
    Item(usize),
    Hover(usize),
    Sub(usize),
    Dismissed,
}

pub struct Entry<'a> {
    pub label: &'a str,
    pub icon: Option<icons::Icon>,
    pub disabled: bool,
    pub divided: bool,
}
