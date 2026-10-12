mod item;
mod layer;
mod shadow;

use super::icons;

pub use layer::Menu;
pub use shadow::{box_shadow, shadow_reach};

#[derive(Debug)]
pub enum MenuChoice {
    Item(usize),
    Dismissed,
}

pub struct Entry<'a> {
    pub label: &'a str,
    pub icon: Option<icons::Icon>,
    pub disabled: bool,
}
