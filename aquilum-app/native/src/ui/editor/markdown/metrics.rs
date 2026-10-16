use masonry::parley::FontFamily;

pub const LIST_START: f64 = 12.0;
pub const LIST_LEVEL: f64 = 24.0;
pub const QUOTE_BAR: f64 = 3.0;
pub(super) const CODE_PADDING: f64 = 12.0;
pub(super) const INLINE_CODE_PADDING: f32 = 4.0;
pub(super) const TAG_PADDING: f32 = 6.0;
pub(super) const COLLAPSED: f32 = 0.001;
pub const CHECKBOX: f32 = 20.0;
pub(super) const BOX_CHARS: usize = 3;
pub(super) const TAB_SIZE: f32 = 4.0;
pub(super) const SCRIPT: f32 = 0.75;
pub(super) const H1: f32 = 24.0 / 14.0;
pub(super) const H2: f32 = 16.0 / 14.0;

pub struct Fonts {
    pub size: f32,
    pub mono: FontFamily<'static>,
    pub ch: f32,
    pub callouts: bool,
    pub tags: bool,
}
