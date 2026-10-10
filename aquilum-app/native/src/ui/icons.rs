//! Иконки lucide 1.24 (ISC, © Lucide Contributors), как в `src/components/Common/Icon.tsx`:
//! контуры в квадрате 24×24, рисуются обводкой цвета текста со скруглёнными концами и стыками.

use masonry::imaging::Painter;
use masonry::imaging::PaintSink;
use masonry::kurbo::{Affine, BezPath, Cap, Circle, Join, Point, RoundedRect, Shape as _, Stroke};
use masonry::peniko::Color;

pub enum Shape {
    Path(&'static str),
    Rect { x: f64, y: f64, w: f64, h: f64, rx: f64 },
    Circle { cx: f64, cy: f64, r: f64 },
}

/// Иконка: фигуры на квадратной сетке `grid` (у lucide — 24×24).
#[derive(Clone, Copy)]
pub struct Icon {
    shapes: &'static [Shape],
    grid: f64,
}

impl Icon {
    /// Иконка lucide на сетке 24×24.
    pub const fn lucide(shapes: &'static [Shape]) -> Self {
        Icon { shapes, grid: 24.0 }
    }
}

pub const NETWORK: Icon = Icon::lucide(&[
    Shape::Rect { x: 16.0, y: 16.0, w: 6.0, h: 6.0, rx: 1.0 },
    Shape::Rect { x: 2.0, y: 16.0, w: 6.0, h: 6.0, rx: 1.0 },
    Shape::Rect { x: 9.0, y: 2.0, w: 6.0, h: 6.0, rx: 1.0 },
    Shape::Path("M5 16v-3a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v3"),
    Shape::Path("M12 12V8"),
]);
pub const ROTATE_CCW: Icon = Icon::lucide(&[Shape::Path("M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"), Shape::Path("M3 3v5h5")]);
pub const CHEVRON_LEFT: Icon = Icon::lucide(&[Shape::Path("m15 18-6-6 6-6")]);
pub const SETTINGS_2: Icon = Icon::lucide(&[
    Shape::Path("M14 17H5"),
    Shape::Path("M19 7h-9"),
    Shape::Circle { cx: 17.0, cy: 17.0, r: 3.0 },
    Shape::Circle { cx: 7.0, cy: 7.0, r: 3.0 },
]);
pub const CHEVRON_RIGHT: Icon = Icon::lucide(&[Shape::Path("m9 18 6-6-6-6")]);
pub const CHEVRON_DOWN: Icon = Icon::lucide(&[Shape::Path("m6 9 6 6 6-6")]);
pub const CHEVRON_UP: Icon = Icon::lucide(&[Shape::Path("m18 15-6-6-6 6")]);
pub const CHEVRONS_UP_DOWN: Icon = Icon::lucide(&[Shape::Path("m7 15 5 5 5-5"), Shape::Path("m7 9 5-5 5 5")]);
pub const PANEL_LEFT: Icon =
    Icon::lucide(&[Shape::Rect { x: 3.0, y: 3.0, w: 18.0, h: 18.0, rx: 2.0 }, Shape::Path("M9 3v18")]);
pub const PANEL_RIGHT: Icon =
    Icon::lucide(&[Shape::Rect { x: 3.0, y: 3.0, w: 18.0, h: 18.0, rx: 2.0 }, Shape::Path("M15 3v18")]);
pub const BACKLINKS: Icon = Icon {
    shapes: &[Shape::Path(LINK_PATH), Shape::Path("M14.1667 12.5L11.6667 15L14.1667 17.5M11.6667 15H17.5")],
    grid: 20.0,
};
pub const OUTGOING_LINKS: Icon = Icon {
    shapes: &[Shape::Path(LINK_PATH), Shape::Path("M15 12.5L17.5 15L15 17.5M17.5 15H11.6667")],
    grid: 20.0,
};
const LINK_PATH: &str = "M7.25391 9.99996C6.84716 9.70029 6.5106 9.31804 6.26705 8.87904C6.02351 8.44004 5.87869 7.95457 5.8424 7.4556C5.80612 6.95662 5.87922 6.4558 6.05675 5.98709C6.23429 5.51839 6.5121 5.09278 6.87134 4.7391L8.99677 2.64592C9.66485 2.01044 10.5597 1.65881 11.4884 1.66676C12.4173 1.67471 13.3057 2.0416 13.9625 2.68842C14.6193 3.33524 14.9919 4.21023 14.9999 5.12494C15.0079 6.03965 14.6509 6.92088 14.0056 7.57885L13.2512 8.33329M9.41277 6.66662C9.81952 6.96627 10.1561 7.34855 10.3997 7.78756C10.6432 8.22656 10.788 8.71204 10.8243 9.21096C10.8606 9.70996 10.7875 10.2108 10.6099 10.6795C10.4324 11.1482 10.1546 11.5738 9.79535 11.9275L7.66995 14.0206C7.00186 14.6561 6.10705 15.0078 5.17826 14.9998C4.24946 14.9919 3.361 14.625 2.70422 13.9781C2.04744 13.3314 1.6749 12.4564 1.66682 11.5416C1.65875 10.627 2.0158 9.74571 2.66107 9.08771L3.41551 8.33329";
pub const FILE: Icon = Icon::lucide(&[
    Shape::Path("M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z"),
    Shape::Path("M14 2v5a1 1 0 0 0 1 1h5"),
]);
pub const SEARCH_X: Icon = Icon::lucide(&[
    Shape::Path("m13.5 8.5-5 5"),
    Shape::Path("m8.5 8.5 5 5"),
    Shape::Circle { cx: 11.0, cy: 11.0, r: 8.0 },
    Shape::Path("m21 21-4.3-4.3"),
]);
pub const FOLDER_SEARCH: Icon = Icon::lucide(&[
    Shape::Path("M10.7 20H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H20a2 2 0 0 1 2 2v4.1"),
    Shape::Path("m21 21-1.9-1.9"),
    Shape::Circle { cx: 17.0, cy: 17.0, r: 3.0 },
]);
pub const DATABASE_ZAP: Icon = Icon::lucide(&[
    Shape::Path("M3 5a9 3 0 1 0 18 0a9 3 0 1 0 -18 0"),
    Shape::Path("M3 5V19A9 3 0 0 0 15 21.84"),
    Shape::Path("M21 5V8"),
    Shape::Path("M21 12L18 17H22L19 22"),
    Shape::Path("M3 12A9 3 0 0 0 14.59 14.87"),
]);
pub const ARROW_UP_DOWN: Icon = Icon::lucide(&[
    Shape::Path("m21 16-4 4-4-4"),
    Shape::Path("M17 20V4"),
    Shape::Path("m3 8 4-4 4 4"),
    Shape::Path("M7 4v16"),
]);
pub const CORNER_DOWN_LEFT: Icon = Icon::lucide(&[Shape::Path("M20 4v7a4 4 0 0 1-4 4H4"), Shape::Path("m9 10-5 5 5 5")]);
pub const ARROW_LEFT: Icon = Icon::lucide(&[Shape::Path("m12 19-7-7 7-7"), Shape::Path("M19 12H5")]);
pub const ARROW_RIGHT: Icon = Icon::lucide(&[Shape::Path("M5 12h14"), Shape::Path("m12 5 7 7-7 7")]);
pub const FOLDER_OPEN: Icon = Icon::lucide(&[Shape::Path(
    "m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2",
)]);
pub const LIBRARY: Icon = Icon::lucide(&[
    Shape::Path("m16 6 4 14"),
    Shape::Path("M12 6v14"),
    Shape::Path("M8 8v12"),
    Shape::Path("M4 4v16"),
]);
pub const HOUSE: Icon = Icon::lucide(&[
    Shape::Path("M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8"),
    Shape::Path("M3 10a2 2 0 0 1 .709-1.528l7-6a2 2 0 0 1 2.582 0l7 6A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"),
]);
pub const SETTINGS: Icon = Icon::lucide(&[
    Shape::Path(
        "M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 \
         2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 \
         1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 \
         2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915",
    ),
    Shape::Circle { cx: 12.0, cy: 12.0, r: 3.0 },
]);

pub const FILE_TEXT: Icon = Icon::lucide(&[
    Shape::Path("M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z"),
    Shape::Path("M14 2v5a1 1 0 0 0 1 1h5"),
    Shape::Path("M10 9H8"),
    Shape::Path("M16 13H8"),
    Shape::Path("M16 17H8"),
]);

pub const MONITOR: Icon = Icon::lucide(&[
    Shape::Rect { x: 2.0, y: 3.0, w: 20.0, h: 14.0, rx: 2.0 },
    Shape::Path("M8 21h8"),
    Shape::Path("M12 17v4"),
]);
pub const TYPE: Icon = Icon::lucide(&[
    Shape::Path("M12 4v16"),
    Shape::Path("M4 7V5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v2"),
    Shape::Path("M9 20h6"),
]);
pub const BOOK_OPEN: Icon = Icon::lucide(&[
    Shape::Path("M12 7v14"),
    Shape::Path("M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z"),
]);
pub const SEARCH: Icon = Icon::lucide(&[Shape::Path("m21 21-4.34-4.34"), Shape::Circle { cx: 11.0, cy: 11.0, r: 8.0 }]);
pub const PAPERCLIP: Icon = Icon::lucide(&[Shape::Path(
    "m16 6-8.414 8.586a2 2 0 0 0 2.829 2.829l8.414-8.586a4 4 0 1 0-5.657-5.657l-8.379 8.551a6 6 0 1 0 8.485 8.485l8.379-8.551",
)]);
pub const HISTORY: Icon = Icon::lucide(&[
    Shape::Path("M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"),
    Shape::Path("M3 3v5h5"),
    Shape::Path("M12 7v5l4 2"),
]);
pub const PLUG: Icon = Icon::lucide(&[
    Shape::Path("M12 22v-5"),
    Shape::Path("M15 8V2"),
    Shape::Path("M17 8a1 1 0 0 1 1 1v4a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V9a1 1 0 0 1 1-1z"),
    Shape::Path("M9 8V2"),
]);
pub const UPLOAD: Icon = Icon::lucide(&[
    Shape::Path("M12 3v12"),
    Shape::Path("m17 8-5-5-5 5"),
    Shape::Path("M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"),
]);
pub const SHUFFLE: Icon = Icon::lucide(&[
    Shape::Path("m18 14 4 4-4 4"),
    Shape::Path("m18 2 4 4-4 4"),
    Shape::Path("M2 18h1.973a4 4 0 0 0 3.3-1.7l5.454-8.6a4 4 0 0 1 3.3-1.7H22"),
    Shape::Path("M2 6h1.972a4 4 0 0 1 3.6 2.2"),
    Shape::Path("M22 18h-6.041a4 4 0 0 1-3.3-1.8l-.359-.45"),
]);
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
pub const TRASH: Icon = Icon::lucide(&[
    Shape::Path("M10 11v6"),
    Shape::Path("M14 11v6"),
    Shape::Path("M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"),
    Shape::Path("M3 6h18"),
    Shape::Path("M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"),
]);
pub const POWER: Icon = Icon::lucide(&[Shape::Path("M12 2v10"), Shape::Path("M18.4 6.6a9 9 0 1 1-12.77.04")]);
pub const KEYBOARD: Icon = Icon::lucide(&[
    Shape::Path("M10 8h.01"),
    Shape::Path("M12 12h.01"),
    Shape::Path("M14 8h.01"),
    Shape::Path("M16 12h.01"),
    Shape::Path("M18 8h.01"),
    Shape::Path("M6 8h.01"),
    Shape::Path("M7 16h10"),
    Shape::Path("M8 12h.01"),
    Shape::Rect { x: 2.0, y: 4.0, w: 20.0, h: 16.0, rx: 2.0 },
]);
pub const X: Icon = Icon::lucide(&[Shape::Path("M18 6 6 18"), Shape::Path("m6 6 12 12")]);
pub const CHECK: Icon = Icon::lucide(&[Shape::Path("M20 6 9 17l-5-5")]);
pub const MINUS: Icon = Icon::lucide(&[Shape::Path("M5 12h14")]);
pub const PLUS: Icon = Icon::lucide(&[Shape::Path("M5 12h14"), Shape::Path("M12 5v14")]);
pub const PENCIL: Icon = Icon::lucide(&[
    Shape::Path("M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"),
    Shape::Path("m15 5 4 4"),
]);
pub const COPY: Icon = Icon::lucide(&[
    Shape::Rect { x: 8.0, y: 8.0, w: 14.0, h: 14.0, rx: 2.0 },
    Shape::Path("M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"),
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
pub const ROWS: Icon = Icon::lucide(&[Shape::Rect { x: 3.0, y: 3.0, w: 18.0, h: 18.0, rx: 2.0 }, Shape::Path("M21 9H3"), Shape::Path("M21 15H3")]);
pub const COLUMNS: Icon = Icon::lucide(&[Shape::Rect { x: 3.0, y: 3.0, w: 18.0, h: 18.0, rx: 2.0 }, Shape::Path("M9 3v18"), Shape::Path("M15 3v18")]);
pub const ZOOM_IN: Icon = Icon::lucide(&[Shape::Circle { cx: 11.0, cy: 11.0, r: 8.0 }, Shape::Path("M21 21 16.65 16.65"), Shape::Path("M11 8v6"), Shape::Path("M8 11h6")]);
pub const ALIGN_LEFT: Icon = Icon::lucide(&[Shape::Path("M21 5H3"), Shape::Path("M15 12H3"), Shape::Path("M17 19H3")]);
pub const ALIGN_CENTER: Icon = Icon::lucide(&[Shape::Path("M21 5H3"), Shape::Path("M17 12H7"), Shape::Path("M19 19H5")]);
pub const ALIGN_RIGHT: Icon = Icon::lucide(&[Shape::Path("M21 5H3"), Shape::Path("M21 12H9"), Shape::Path("M21 19H7")]);
pub const CROP: Icon = Icon::lucide(&[Shape::Path("M6 2v14a2 2 0 0 0 2 2h14"), Shape::Path("M18 22V8a2 2 0 0 0-2-2H2")]);
pub const CODE_XML: Icon = Icon::lucide(&[Shape::Path("m18 16 4-4-4-4"), Shape::Path("m6 8-4 4 4 4"), Shape::Path("m14.5 4-5 16")]);
pub const MOVE_DIAGONAL: Icon = Icon::lucide(&[Shape::Path("M19 13v6h-6"), Shape::Path("M5 11V5h6"), Shape::Path("m5 5 14 14")]);

/// `GraphAnalysisIcon` из `src/components/Icons/LinkIcons.tsx`: сетка 20×20, обводка 1.2.
pub const GRAPH_ANALYSIS: Icon = Icon {
    shapes: &[Shape::Path(
        "M7.36309 11.3099C7.36309 13.0254 8.75382 14.4162 10.4694 14.4162C12.1849 14.4162 13.5757 13.0254 13.5757 11.3099C13.5757 9.59434 12.1849 8.20361 10.4694 8.20361C8.75382 8.20361 7.36309 9.59434 7.36309 11.3099ZM7.36309 11.3099H5.01001M5.01001 11.3099C5.01001 12.3377 4.17683 13.1709 3.14905 13.1709C2.12127 13.1709 1.28809 12.3377 1.28809 11.3099C1.28809 10.2821 2.12127 9.44893 3.14905 9.44893C4.17683 9.44893 5.01001 10.2821 5.01001 11.3099ZM15.7495 7.83659L13.0998 9.65685M8.00249 5.93358L9.08208 8.52984M13.2958 13.5142L15.1499 14.9855M9.44467 3.63543C9.44467 4.98627 8.3496 6.08135 6.99875 6.08135C5.6479 6.08135 4.55283 4.98627 4.55283 3.63543C4.55283 2.28458 5.6479 1.1895 6.99875 1.1895C8.3496 1.1895 9.44467 2.28458 9.44467 3.63543ZM18.3527 6.59846C18.3527 7.36053 17.7349 7.97832 16.9728 7.97832C16.2108 7.97832 15.593 7.36053 15.593 6.59846C15.593 5.83639 16.2108 5.2186 16.9728 5.2186C17.7349 5.2186 18.3527 5.83639 18.3527 6.59846ZM18.7346 16.4627C18.7346 17.4905 17.9014 18.3237 16.8736 18.3237C15.8458 18.3237 15.0126 17.4905 15.0126 16.4627C15.0126 15.4349 15.8458 14.6018 16.8736 14.6018C17.9014 14.6018 18.7346 15.4349 18.7346 16.4627Z",
    )],
    grid: 20.0,
};

impl Icon {
    /// Рисует иконку квадратом `size` с левым верхним углом в `origin`; `stroke_width` —
    /// в единицах сетки иконки, как атрибут `stroke-width` у SVG.
    pub fn draw(
        &self,
        painter: &mut Painter<'_, impl PaintSink + ?Sized>,
        origin: Point,
        size: f64,
        stroke_width: f64,
        color: Color,
    ) {
        let scale = size / self.grid;
        let transform = Affine::translate(origin.to_vec2()) * Affine::scale(scale);
        let stroke = Stroke::new(stroke_width * scale).with_caps(Cap::Round).with_join(Join::Round);
        for shape in self.shapes {
            let path: BezPath = match *shape {
                Shape::Path(d) => match BezPath::from_svg(d) {
                    Ok(path) => path,
                    Err(_) => continue,
                },
                Shape::Rect { x, y, w, h, rx } => RoundedRect::new(x, y, x + w, y + h, rx).to_path(0.1),
                Shape::Circle { cx, cy, r } => Circle::new((cx, cy), r).to_path(0.1),
            };
            painter.stroke(&(transform * path), &stroke, color).draw();
        }
    }
}
