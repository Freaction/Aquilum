use super::damage::Damage;
use super::renderer::Renderer;
use crate::output::{Area, Canvas};

#[derive(Default)]
pub struct Overlay;

impl Overlay {
    pub fn busy(&self) -> bool {
        false
    }

    pub fn damage(&mut self, _: &Damage) {}

    pub fn live(&mut self, _: Area) {}
}

impl Renderer {
    pub(super) fn decorate(&mut self, _: &mut dyn Canvas) -> Option<Area> {
        None
    }
}
