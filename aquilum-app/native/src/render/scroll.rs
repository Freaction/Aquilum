//! Области прокрутки, о которых знает поиск изменений: окно просмотра в координатах окна и
//! смещение содержимого. Благодаря им прокрутка сдвигает уже нарисованные пиксели, а
//! растрируется только открывшаяся полоса.

use std::cell::RefCell;

use masonry::kurbo::Rect;

#[derive(Clone, Copy)]
pub struct Region {
    pub id: u64,
    pub viewport: Rect,
    pub offset: f64,
}

thread_local! {
    static REGIONS: RefCell<Vec<Region>> = const { RefCell::new(Vec::new()) };
}

pub fn publish(id: u64, viewport: Rect, offset: f64) {
    REGIONS.with_borrow_mut(|regions| match regions.iter_mut().find(|r| r.id == id) {
        Some(region) => *region = Region { id, viewport, offset },
        None => regions.push(Region { id, viewport, offset }),
    });
}

pub fn forget(id: u64) {
    REGIONS.with_borrow_mut(|regions| regions.retain(|r| r.id != id));
}

pub(super) fn snapshot() -> Vec<Region> {
    REGIONS.with_borrow(Clone::clone)
}

pub(super) fn snap(rect: Rect) -> Rect {
    Rect::new(rect.x0.round(), rect.y0.round(), rect.x1.round(), rect.y1.round())
}

pub(super) fn close(a: Rect, b: Rect) -> bool {
    (a.x0 - b.x0).abs() < 0.5 && (a.y0 - b.y0).abs() < 0.5 && (a.x1 - b.x1).abs() < 0.5 && (a.y1 - b.y1).abs() < 0.5
}

pub(super) fn shift_rows(pixels: &mut [u32], stride: usize, shift: super::damage::Move) {
    let (x, w) = (usize::from(shift.x), usize::from(shift.w));
    let (top, height) = (usize::from(shift.y), usize::from(shift.h));
    let distance = shift.dy.unsigned_abs() as usize;
    let rows = height - distance;
    let row = |y: usize| (top + y) * stride + x;
    if shift.dy > 0 {
        for y in (0..rows).rev() {
            pixels.copy_within(row(y)..row(y) + w, row(y + distance));
        }
    } else {
        for y in 0..rows {
            pixels.copy_within(row(y + distance)..row(y + distance) + w, row(y));
        }
    }
}
