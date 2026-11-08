mod app;
mod arrows;
mod editor;
mod files;
mod links;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use masonry::imaging::{PaintSink, Painter};
use masonry::kurbo::{Affine, BezPath, Cap, Circle, Join, Point, RoundedRect, Shape as _, Stroke};
use masonry::peniko::Color;

pub use app::*;
pub use arrows::*;
pub use editor::*;
pub use files::*;
pub use links::*;

pub enum Shape {
    Path(&'static str),
    Rect { x: f64, y: f64, w: f64, h: f64, rx: f64 },
    Circle { cx: f64, cy: f64, r: f64 },
}

#[derive(Clone, Copy)]
pub struct Icon {
    shapes: &'static [Shape],
    grid: f64,
    turn: f64,
}

thread_local! {
    static OUTLINES: RefCell<HashMap<(usize, usize), Rc<BezPath>>> = RefCell::new(HashMap::new());
}

impl Icon {
    pub const fn lucide(shapes: &'static [Shape]) -> Self {
        Icon { shapes, grid: 24.0, turn: 0.0 }
    }

    pub const fn on_grid(shapes: &'static [Shape], grid: f64) -> Self {
        Icon { shapes, grid, turn: 0.0 }
    }

    pub fn turned(self, turn: f64) -> Self {
        Icon { turn, ..self }
    }

    pub fn draw(
        &self,
        painter: &mut Painter<'_, impl PaintSink + ?Sized>,
        origin: Point,
        size: f64,
        stroke_width: f64,
        color: Color,
    ) {
        let scale = size / self.grid;
        let center = Point::new(self.grid / 2.0, self.grid / 2.0);
        let transform = Affine::translate(origin.to_vec2()) * Affine::scale(scale) * Affine::rotate_about(self.turn, center);
        let stroke = Stroke::new(stroke_width * scale).with_caps(Cap::Round).with_join(Join::Round);
        let outline = self.outline();
        painter.stroke(&(transform * outline.as_ref()), &stroke, color).draw();
    }

    fn outline(&self) -> Rc<BezPath> {
        OUTLINES.with_borrow_mut(|outlines| Rc::clone(outlines.entry((self.shapes.as_ptr() as usize, self.shapes.len())).or_insert_with(|| {
            let mut outline = BezPath::new();
            for shape in self.shapes {
                let path = match *shape {
                    Shape::Path(d) => match BezPath::from_svg(d) {
                        Ok(path) => path,
                        Err(_) => continue,
                    },
                    Shape::Rect { x, y, w, h, rx } => RoundedRect::new(x, y, x + w, y + h, rx).to_path(0.1),
                    Shape::Circle { cx, cy, r } => Circle::new((cx, cy), r).to_path(0.1),
                };
                outline.extend(path);
            }
            Rc::new(outline)
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_geometry_is_shared_across_repaints_and_turns_cover_audit() {
        for icon in [CHEVRON_DOWN, MORE_HORIZONTAL, FILE, UPLOAD] {
            let first = icon.outline();
            assert!(!first.elements().is_empty());
            assert!(Rc::ptr_eq(&first, &icon.outline()));
            assert!(Rc::ptr_eq(&first, &icon.turned(0.75).outline()));
        }
    }
}
