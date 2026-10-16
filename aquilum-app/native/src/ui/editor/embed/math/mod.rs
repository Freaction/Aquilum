mod glyph;
#[cfg(test)]
mod tests;

use masonry::imaging::record::Glyph;
use masonry::imaging::{PaintSink, Painter};
use masonry::kurbo::{Affine, BezPath, Line, Point, Rect, Shape, Stroke, Vec2};
use masonry::peniko::{Brush, Color};
use ratex_layout::{LayoutOptions, layout, to_display_list};
use ratex_types::{Color as Ink, DisplayItem, DisplayList, MathStyle, PathCommand};

use super::drawing::{Drawing, Item, paint_item};

const SCALE: f64 = 1.21;
const GAP: f64 = 0.5;
const STROKE: f64 = 0.0375;
const TOLERANCE: f64 = 0.1;

pub struct Formula {
    pub width: f32,
    pub ascent: f32,
    pub depth: f32,
    items: Vec<Item>,
}

impl Formula {
    pub fn paint(&self, painter: &mut Painter<'_, impl PaintSink + ?Sized>, at: Vec2, palette: &[Brush]) {
        for item in &self.items {
            paint_item(painter, item, Affine::translate(at), palette, None);
        }
    }
}

pub fn math(source: &str, size: f64, available: f64) -> Option<Drawing> {
    let list = display_list(source, MathStyle::Display)?;
    let em = (size * SCALE).min(available / list.width);
    let gap = size * GAP;
    let items = draw(&list, em, Vec2::new((available - list.width * em) / 2.0, gap));
    Some(Drawing::new((list.height + list.depth) * em + gap * 2.0, items))
}

pub fn formula(source: &str, size: f64, display: bool) -> Option<Formula> {
    let list = display_list(source, if display { MathStyle::Display } else { MathStyle::Text })?;
    let em = size * SCALE;
    Some(Formula { width: (list.width * em) as f32, ascent: (list.height * em) as f32, depth: (list.depth * em) as f32, items: draw(&list, em, Vec2::ZERO) })
}

fn display_list(source: &str, style: MathStyle) -> Option<DisplayList> {
    let nodes = ratex_parser::parser::parse(source).ok()?;
    let list = to_display_list(&layout(&nodes, &LayoutOptions { style, ..LayoutOptions::default() }));
    (!list.items.is_empty() && list.width > 0.0).then_some(list)
}

fn draw(list: &DisplayList, em: f64, origin: Vec2) -> Vec<Item> {
    let at = |x: f64, y: f64| Point::new(x * em, y * em) + origin;
    let mut items = Vec::new();
    for item in &list.items {
        match item {
            DisplayItem::GlyphPath { x, y, scale, font, char_code, color } => {
                let Some((face, id)) = glyph::glyph(font, *char_code) else { continue };
                let (size, color, p) = ((em * scale) as f32, tint(color), at(*x, *y));
                let glyph = Glyph { id, x: p.x as f32, y: p.y as f32 };
                match items.last_mut() {
                    Some(Item::Glyphs(run_face, run_size, run, run_color)) if run_face.data.id() == face.data.id() && *run_size == size && *run_color == color => run.push(glyph),
                    _ => items.push(Item::Glyphs(face, size, vec![glyph], color)),
                }
            }
            DisplayItem::Line { x, y, width, thickness, color, dashed: false } => {
                items.push(Item::Path(Rect::from_points(at(*x, y - thickness / 2.0), at(x + width, y + thickness / 2.0)).to_path(TOLERANCE), tint(color), None));
            }
            DisplayItem::Line { x, y, width, thickness, color, dashed: true } => {
                let dash = 3.0 * thickness * em;
                items.push(Item::Path(Line::new(at(*x, *y), at(x + width, *y)).to_path(TOLERANCE), tint(color), Some(Stroke::new(thickness * em).with_dashes(0.0, [dash, dash]))));
            }
            DisplayItem::Rect { x, y, width, height, color } => {
                items.push(Item::Path(Rect::from_points(at(*x, *y), at(x + width, y + height)).to_path(TOLERANCE), tint(color), None));
            }
            DisplayItem::Path { x, y, commands, fill, color } => {
                let stroke = (!fill).then(|| Stroke::new(STROKE * em));
                items.extend(subpaths(commands, |px, py| at(x + px, y + py), *fill).into_iter().map(|path| Item::Path(path, tint(color), stroke.clone())));
            }
        }
    }
    items
}

fn tint(color: &Ink) -> Option<Color> {
    (*color != Ink::BLACK).then(|| Color::new([color.r, color.g, color.b, color.a]))
}

fn subpaths(commands: &[PathCommand], at: impl Fn(f64, f64) -> Point, split: bool) -> Vec<BezPath> {
    let mut out = Vec::new();
    let mut path = BezPath::new();
    for command in commands {
        match *command {
            PathCommand::MoveTo { x, y } => {
                if split && !path.elements().is_empty() {
                    out.push(std::mem::take(&mut path));
                }
                path.move_to(at(x, y));
            }
            PathCommand::LineTo { x, y } => path.line_to(at(x, y)),
            PathCommand::QuadTo { x1, y1, x, y } => path.quad_to(at(x1, y1), at(x, y)),
            PathCommand::CubicTo { x1, y1, x2, y2, x, y } => path.curve_to(at(x1, y1), at(x2, y2), at(x, y)),
            PathCommand::Close => path.close_path(),
        }
    }
    if !path.elements().is_empty() {
        out.push(path);
    }
    out
}
