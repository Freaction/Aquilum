use super::*;

fn edge_at(position: winit::dpi::PhysicalPosition<f64>, size: winit::dpi::PhysicalSize<u32>, scale: f64) -> Option<ResizeDirection> {
    let border = RESIZE_BORDER * scale;
    let corner = border * 2.0;
    let (w, h) = (f64::from(size.width), f64::from(size.height));
    let (x, y) = (position.x, position.y);
    let left = x < border;
    let right = x > w - border;
    let top = y < border;
    let bottom = y > h - border;
    let near_left = x < corner;
    let near_right = x > w - corner;
    let near_top = y < corner;
    let near_bottom = y > h - corner;
    if (top && near_left) || (left && near_top) {
        Some(ResizeDirection::NorthWest)
    } else if (top && near_right) || (right && near_top) {
        Some(ResizeDirection::NorthEast)
    } else if (bottom && near_left) || (left && near_bottom) {
        Some(ResizeDirection::SouthWest)
    } else if (bottom && near_right) || (right && near_bottom) {
        Some(ResizeDirection::SouthEast)
    } else if top {
        Some(ResizeDirection::North)
    } else if bottom {
        Some(ResizeDirection::South)
    } else if left {
        Some(ResizeDirection::West)
    } else if right {
        Some(ResizeDirection::East)
    } else {
        None
    }
}

impl Host {
    pub(super) fn update_resize_edge(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        let window = self.window();
        let framed = NATIVE_FRAME || !ui::titlebar::WINDOW_CHROME || window.is_maximized();
        let edge = if framed { None } else { edge_at(position, window.surface_size(), window.scale_factor()) };
        if edge != self.resize_edge {
            self.resize_edge = edge;
            let cursor = edge.map_or(self.cursor, CursorIcon::from);
            self.window().set_cursor(cursor.into());
        }
    }
}
