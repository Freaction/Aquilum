use std::ops::Range;

use masonry::core::BrushIndex;
use masonry::parley::Alignment;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Font {
    Editor,
    Ui(u8),
    Mono(u8),
}

pub struct Spec<'a> {
    pub text: &'a str,
    pub font: Font,
    pub brush: BrushIndex,
    pub bold: bool,
    pub weight: Option<f32>,
    pub links: &'a [Range<usize>],
    pub underline: bool,
    pub wrap: Option<f64>,
    pub align: Alignment,
}

impl<'a> Spec<'a> {
    pub fn plain(text: &'a str, font: Font, brush: BrushIndex) -> Self {
        Spec { text, font, brush, bold: false, weight: None, links: &[], underline: false, wrap: None, align: Alignment::Start }
    }
}
