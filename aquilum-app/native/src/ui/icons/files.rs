use super::{Icon, Shape};

const FILE_OUTLINE: &str = "M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z";
const FILE_FOLD: &str = "M14 2v5a1 1 0 0 0 1 1h5";
const FOLDER_OUTLINE: &str = "M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z";
const ROTATE_ARC: &str = "M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8";
const ROTATE_ARROW: &str = "M3 3v5h5";

pub const PALETTE: Icon = Icon::lucide(&[
    Shape::Path("M12 22a1 1 0 0 1 0-20 10 9 0 0 1 10 9 5 5 0 0 1-5 5h-2.25a1.75 1.75 0 0 0-1.4 2.8l.3.4a1.75 1.75 0 0 1-1.4 2.8z"),
    Shape::Path("M13.5 6.5h.01"),
    Shape::Path("M17.5 10.5h.01"),
    Shape::Path("M6.5 12.5h.01"),
    Shape::Path("M8.5 7.5h.01"),
]);
pub const ROTATE_CCW: Icon = Icon::lucide(&[Shape::Path(ROTATE_ARC), Shape::Path(ROTATE_ARROW)]);
pub const HISTORY: Icon = Icon::lucide(&[Shape::Path(ROTATE_ARC), Shape::Path(ROTATE_ARROW), Shape::Path("M12 7v5l4 2")]);
pub const FILE: Icon = Icon::lucide(&[Shape::Path(FILE_OUTLINE), Shape::Path(FILE_FOLD)]);
pub const FILE_TEXT: Icon = Icon::lucide(&[
    Shape::Path(FILE_OUTLINE),
    Shape::Path(FILE_FOLD),
    Shape::Path("M10 9H8"),
    Shape::Path("M16 13H8"),
    Shape::Path("M16 17H8"),
]);
pub const FILE_X: Icon = Icon::lucide(&[
    Shape::Path("M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"),
    Shape::Path("M14 2v4a2 2 0 0 0 2 2h4"),
    Shape::Path("m14.5 12.5-5 5"),
    Shape::Path("m9.5 12.5 5 5"),
]);
pub const FOLDER: Icon = Icon::lucide(&[Shape::Path(FOLDER_OUTLINE)]);
pub const FOLDER_PLUS: Icon = Icon::lucide(&[Shape::Path("M12 10v6"), Shape::Path("M9 13h6"), Shape::Path(FOLDER_OUTLINE)]);
pub const FOLDER_SEARCH: Icon = Icon::lucide(&[
    Shape::Path("M10.7 20H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H20a2 2 0 0 1 2 2v4.1"),
    Shape::Path("m21 21-1.9-1.9"),
    Shape::Circle { cx: 17.0, cy: 17.0, r: 3.0 },
]);
pub const FOLDER_OPEN: Icon = Icon::lucide(&[Shape::Path(
    "m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2",
)]);
pub const LIBRARY: Icon = Icon::lucide(&[
    Shape::Path("m16 6 4 14"),
    Shape::Path("M12 6v14"),
    Shape::Path("M8 8v12"),
    Shape::Path("M4 4v16"),
]);
pub const BOOK_OPEN: Icon = Icon::lucide(&[
    Shape::Path("M12 7v14"),
    Shape::Path("M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z"),
]);
pub const PAPERCLIP: Icon = Icon::lucide(&[Shape::Path(
    "m16 6-8.414 8.586a2 2 0 0 0 2.829 2.829l8.414-8.586a4 4 0 1 0-5.657-5.657l-8.379 8.551a6 6 0 1 0 8.485 8.485l8.379-8.551",
)]);
pub const DATABASE_ZAP: Icon = Icon::lucide(&[
    Shape::Path("M3 5a9 3 0 1 0 18 0a9 3 0 1 0 -18 0"),
    Shape::Path("M3 5V19A9 3 0 0 0 15 21.84"),
    Shape::Path("M21 5V8"),
    Shape::Path("M21 12L18 17H22L19 22"),
    Shape::Path("M3 12A9 3 0 0 0 14.59 14.87"),
]);
pub const UPLOAD: Icon = Icon::lucide(&[
    Shape::Path("M12 3v12"),
    Shape::Path("m17 8-5-5-5 5"),
    Shape::Path("M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"),
]);
pub const TRASH: Icon = Icon::lucide(&[
    Shape::Path("M10 11v6"),
    Shape::Path("M14 11v6"),
    Shape::Path("M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"),
    Shape::Path("M3 6h18"),
    Shape::Path("M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"),
]);
