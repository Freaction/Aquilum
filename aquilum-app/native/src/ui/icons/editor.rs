use super::{Icon, Shape};

pub const TEXT_SELECT: Icon = Icon::lucide(&[
    Shape::Path("M5 3a2 2 0 0 0-2 2"),
    Shape::Path("M19 3a2 2 0 0 1 2 2"),
    Shape::Path("M21 19a2 2 0 0 1-2 2"),
    Shape::Path("M5 21a2 2 0 0 1-2-2"),
    Shape::Path("M9 3h1"),
    Shape::Path("M9 21h1"),
    Shape::Path("M14 3h1"),
    Shape::Path("M14 21h1"),
    Shape::Path("M3 9v1"),
    Shape::Path("M21 9v1"),
    Shape::Path("M3 14v1"),
    Shape::Path("M21 14v1"),
    Shape::Path("M7 8h10"),
    Shape::Path("M7 12h10"),
    Shape::Path("M7 16h6"),
]);
pub const TYPE: Icon = Icon::lucide(&[
    Shape::Path("M12 4v16"),
    Shape::Path("M4 7V5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v2"),
    Shape::Path("M9 20h6"),
]);
pub const TABLE_MERGE: Icon = Icon::lucide(&[
    Shape::Path("M12 21v-6"),
    Shape::Path("M12 9V3"),
    Shape::Path("M3 15h18"),
    Shape::Path("M3 9h18"),
    Shape::Rect { x: 3.0, y: 3.0, w: 18.0, h: 18.0, rx: 2.0 },
]);
pub const TABLE_SPLIT: Icon = Icon::lucide(&[
    Shape::Path("M12 15V9"),
    Shape::Path("M3 15h18"),
    Shape::Path("M3 9h18"),
    Shape::Rect { x: 3.0, y: 3.0, w: 18.0, h: 18.0, rx: 2.0 },
]);
pub const TABLE: Icon = Icon::lucide(&[
    Shape::Path("M12 3v18"),
    Shape::Path("M3 9h18"),
    Shape::Path("M3 15h18"),
    Shape::Rect { x: 3.0, y: 3.0, w: 18.0, h: 18.0, rx: 2.0 },
]);
pub const ROWS: Icon = Icon::lucide(&[Shape::Rect { x: 3.0, y: 3.0, w: 18.0, h: 18.0, rx: 2.0 }, Shape::Path("M21 9H3"), Shape::Path("M21 15H3")]);
pub const COLUMNS: Icon = Icon::lucide(&[Shape::Rect { x: 3.0, y: 3.0, w: 18.0, h: 18.0, rx: 2.0 }, Shape::Path("M9 3v18"), Shape::Path("M15 3v18")]);
pub const ZOOM_IN: Icon = Icon::lucide(&[Shape::Circle { cx: 11.0, cy: 11.0, r: 8.0 }, Shape::Path("M21 21 16.65 16.65"), Shape::Path("M11 8v6"), Shape::Path("M8 11h6")]);
pub const ALIGN_LEFT: Icon = Icon::lucide(&[Shape::Path("M21 5H3"), Shape::Path("M15 12H3"), Shape::Path("M17 19H3")]);
pub const ALIGN_CENTER: Icon = Icon::lucide(&[Shape::Path("M21 5H3"), Shape::Path("M17 12H7"), Shape::Path("M19 19H5")]);
pub const ALIGN_RIGHT: Icon = Icon::lucide(&[Shape::Path("M21 5H3"), Shape::Path("M21 12H9"), Shape::Path("M21 19H7")]);
pub const CROP: Icon = Icon::lucide(&[Shape::Path("M6 2v14a2 2 0 0 0 2 2h14"), Shape::Path("M18 22V8a2 2 0 0 0-2-2H2")]);
pub const CODE_XML: Icon = Icon::lucide(&[Shape::Path("m18 16 4-4-4-4"), Shape::Path("m6 8-4 4 4 4"), Shape::Path("m14.5 4-5 16")]);
pub const COPY: Icon = Icon::lucide(&[
    Shape::Rect { x: 8.0, y: 8.0, w: 14.0, h: 14.0, rx: 2.0 },
    Shape::Path("M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"),
]);
pub const CLIPBOARD: Icon = Icon::lucide(&[
    Shape::Rect { x: 8.0, y: 2.0, w: 8.0, h: 4.0, rx: 1.0 },
    Shape::Path("M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"),
]);
pub const PENCIL: Icon = Icon::lucide(&[
    Shape::Path("M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"),
    Shape::Path("m15 5 4 4"),
]);
pub const SQUARE_PEN: Icon = Icon::lucide(&[
    Shape::Path("M12 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"),
    Shape::Path("M18.375 2.625a1 1 0 0 1 3 3l-9.013 9.014a2 2 0 0 1-.853.505l-2.873.84a.5.5 0 0 1-.62-.62l.84-2.873a2 2 0 0 1 .506-.852z"),
]);
pub const TAG: Icon = Icon::lucide(&[
    Shape::Path("M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z"),
    Shape::Circle { cx: 7.5, cy: 7.5, r: 0.5 },
]);
