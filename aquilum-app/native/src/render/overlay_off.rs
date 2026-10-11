use super::stamp::Stamps;
use crate::output::{Area, Canvas};

pub use super::mark::Mark;

#[derive(Default)]
pub struct Overlay {
    pub on: bool,
    pub pending: bool,
}

impl Overlay {
    pub fn flash(&mut self, _: Area, _: Mark) {}

    pub fn show(&mut self, _: &mut dyn Canvas, _: &mut Stamps, _: &[(Area, Mark)]) -> Option<Area> {
        None
    }
}
