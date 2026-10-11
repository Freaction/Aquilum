use std::cell::RefCell;
use std::time::Instant;

use masonry::core::WidgetId;

thread_local! {
    static TIMERS: RefCell<Vec<(WidgetId, Instant)>> = const { RefCell::new(Vec::new()) };
}

pub fn wake(id: WidgetId, at: Instant) {
    TIMERS.with_borrow_mut(|timers| {
        timers.retain(|(other, _)| *other != id);
        timers.push((id, at));
    });
}

pub fn next() -> Option<Instant> {
    TIMERS.with_borrow(|timers| timers.iter().map(|(_, at)| *at).min())
}

pub fn due(now: Instant) -> Vec<WidgetId> {
    TIMERS.with_borrow_mut(|timers| {
        let due = timers.iter().filter(|(_, at)| *at <= now).map(|(id, _)| *id).collect();
        timers.retain(|(_, at)| *at > now);
        due
    })
}
