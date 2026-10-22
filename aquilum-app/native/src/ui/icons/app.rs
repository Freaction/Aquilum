use super::{Icon, Shape};

pub const SETTINGS_2: Icon = Icon::lucide(&[
    Shape::Path("M14 17H5"),
    Shape::Path("M19 7h-9"),
    Shape::Circle { cx: 17.0, cy: 17.0, r: 3.0 },
    Shape::Circle { cx: 7.0, cy: 7.0, r: 3.0 },
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
pub const PANEL_LEFT: Icon =
    Icon::lucide(&[Shape::Rect { x: 3.0, y: 3.0, w: 18.0, h: 18.0, rx: 2.0 }, Shape::Path("M9 3v18")]);
pub const PANEL_RIGHT: Icon =
    Icon::lucide(&[Shape::Rect { x: 3.0, y: 3.0, w: 18.0, h: 18.0, rx: 2.0 }, Shape::Path("M15 3v18")]);
pub const SEARCH: Icon = Icon::lucide(&[Shape::Path("m21 21-4.34-4.34"), Shape::Circle { cx: 11.0, cy: 11.0, r: 8.0 }]);
pub const SEARCH_X: Icon = Icon::lucide(&[
    Shape::Path("m13.5 8.5-5 5"),
    Shape::Path("m8.5 8.5 5 5"),
    Shape::Circle { cx: 11.0, cy: 11.0, r: 8.0 },
    Shape::Path("m21 21-4.3-4.3"),
]);
pub const HOUSE: Icon = Icon::lucide(&[
    Shape::Path("M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8"),
    Shape::Path("M3 10a2 2 0 0 1 .709-1.528l7-6a2 2 0 0 1 2.582 0l7 6A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"),
]);
pub const MONITOR: Icon = Icon::lucide(&[
    Shape::Rect { x: 2.0, y: 3.0, w: 20.0, h: 14.0, rx: 2.0 },
    Shape::Path("M8 21h8"),
    Shape::Path("M12 17v4"),
]);
pub const PLUG: Icon = Icon::lucide(&[
    Shape::Path("M12 22v-5"),
    Shape::Path("M15 8V2"),
    Shape::Path("M17 8a1 1 0 0 1 1 1v4a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V9a1 1 0 0 1 1-1z"),
    Shape::Path("M9 8V2"),
]);
pub const CLOUD: Icon = Icon::lucide(&[Shape::Path("M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z")]);
pub const REFRESH_CW: Icon = Icon::lucide(&[
    Shape::Path("M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"),
    Shape::Path("M21 3v5h-5"),
    Shape::Path("M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"),
    Shape::Path("M8 16H3v5"),
]);
pub const POWER: Icon =Icon::lucide(&[Shape::Path("M12 2v10"), Shape::Path("M18.4 6.6a9 9 0 1 1-12.77.04")]);
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
pub const MORE_HORIZONTAL: Icon = Icon::lucide(&[
    Shape::Circle { cx: 12.0, cy: 12.0, r: 1.0 },
    Shape::Circle { cx: 19.0, cy: 12.0, r: 1.0 },
    Shape::Circle { cx: 5.0, cy: 12.0, r: 1.0 },
]);
