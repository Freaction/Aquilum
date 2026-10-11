#[derive(Clone, Copy, PartialEq)]
pub enum Mark {
    Raster,
    Live,
    Shift,
    Region,
    Layer,
    Sprite,
}
