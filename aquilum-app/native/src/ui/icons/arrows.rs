use super::{Icon, Shape};

pub const CHEVRON_LEFT: Icon = Icon::lucide(&[Shape::Path("m15 18-6-6 6-6")]);
pub const CHEVRON_RIGHT: Icon = Icon::lucide(&[Shape::Path("m9 18 6-6-6-6")]);
pub const CHEVRON_DOWN: Icon = Icon::lucide(&[Shape::Path("m6 9 6 6 6-6")]);
pub const CHEVRON_UP: Icon = Icon::lucide(&[Shape::Path("m18 15-6-6-6 6")]);
pub const CHEVRONS_UP_DOWN: Icon = Icon::lucide(&[Shape::Path("m7 15 5 5 5-5"), Shape::Path("m7 9 5-5 5 5")]);
pub const CHEVRONS_DOWN_UP: Icon = Icon::lucide(&[Shape::Path("m7 20 5-5 5 5"), Shape::Path("m7 4 5 5 5-5")]);
pub const ARROW_UP_NARROW_WIDE: Icon = Icon::lucide(&[
    Shape::Path("m3 8 4-4 4 4"),
    Shape::Path("M7 4v16"),
    Shape::Path("M11 12h4"),
    Shape::Path("M11 16h7"),
    Shape::Path("M11 20h10"),
]);
pub const CORNER_DOWN_LEFT: Icon = Icon::lucide(&[Shape::Path("M20 4v7a4 4 0 0 1-4 4H4"), Shape::Path("m9 10-5 5 5 5")]);
pub const ARROW_UP: Icon = Icon::lucide(&[Shape::Path("m5 12 7-7 7 7"), Shape::Path("M12 19V5")]);
pub const ARROW_DOWN: Icon = Icon::lucide(&[Shape::Path("M12 5v14"), Shape::Path("m19 12-7 7-7-7")]);
pub const ARROW_LEFT: Icon = Icon::lucide(&[Shape::Path("m12 19-7-7 7-7"), Shape::Path("M19 12H5")]);
pub const ARROW_RIGHT: Icon = Icon::lucide(&[Shape::Path("M5 12h14"), Shape::Path("m12 5 7 7-7 7")]);
pub const UNFOLD_VERTICAL: Icon = Icon::lucide(&[
    Shape::Path("M12 22v-6"),
    Shape::Path("M12 8V2"),
    Shape::Path("M4 12H2"),
    Shape::Path("M10 12H8"),
    Shape::Path("M16 12h-2"),
    Shape::Path("M22 12h-2"),
    Shape::Path("m15 19-3 3-3-3"),
    Shape::Path("m15 5-3-3-3 3"),
]);
pub const MOVE_HORIZONTAL: Icon = Icon::lucide(&[Shape::Path("m18 8 4 4-4 4"), Shape::Path("M2 12h20"), Shape::Path("m6 8-4 4 4 4")]);
pub const MOVE_DIAGONAL: Icon = Icon::lucide(&[Shape::Path("M19 13v6h-6"), Shape::Path("M5 11V5h6"), Shape::Path("m5 5 14 14")]);
pub const SHUFFLE: Icon = Icon::lucide(&[
    Shape::Path("m18 14 4 4-4 4"),
    Shape::Path("m18 2 4 4-4 4"),
    Shape::Path("M2 18h1.973a4 4 0 0 0 3.3-1.7l5.454-8.6a4 4 0 0 1 3.3-1.7H22"),
    Shape::Path("M2 6h1.972a4 4 0 0 1 3.6 2.2"),
    Shape::Path("M22 18h-6.041a4 4 0 0 1-3.3-1.8l-.359-.45"),
]);
