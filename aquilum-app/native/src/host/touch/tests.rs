use std::time::{Duration, Instant};

use masonry::kurbo::{Point, Vec2};

use super::detector::Touch;
use crate::ui::keymap::gesture::LONG_PRESS;

const DP: f64 = 2.0;
const FRAME: Duration = Duration::from_millis(8);

fn ms(start: Instant, ms: u64) -> Instant {
    start + Duration::from_millis(ms)
}

fn scrolled(touch: &mut Touch, at: Instant) -> Vec2 {
    touch.frame(at).scroll.map_or(Vec2::ZERO, |(_, delta)| delta)
}

fn near(a: Vec2, b: Vec2) -> bool {
    (a - b).hypot() < 1e-9
}

fn tap(touch: &mut Touch, at: Instant, x: f64) -> Option<u8> {
    touch.down(1, Point::new(x, 100.0), at, DP);
    touch.up(1, Point::new(x, 100.0), at + Duration::from_millis(60)).map(|(_, count)| count)
}

fn flick(touch: &mut Touch, start: Instant, step: f64) -> Instant {
    touch.down(1, Point::new(100.0, 100.0), start, 1.0);
    for k in 1..=10 {
        touch.moved(1, Point::new(100.0, 100.0 + step * f64::from(k)), start + FRAME * k);
        touch.frame(start + FRAME * k);
    }
    let end = start + FRAME * 10;
    assert_eq!(touch.up(1, Point::new(100.0, 100.0 + step * 10.0), end), None, "после прокрутки нет нажатия");
    end
}

#[test]
fn a_tap_clicks_where_the_finger_went_down() {
    let start = Instant::now();
    let mut touch = Touch::default();
    touch.down(1, Point::new(100.0, 100.0), start, DP);
    touch.moved(1, Point::new(104.0, 103.0), ms(start, 40));
    assert_eq!(touch.up(1, Point::new(104.0, 103.0), ms(start, 80)), Some((Point::new(100.0, 100.0), 1)));
    assert_eq!(scrolled(&mut touch, ms(start, 90)), Vec2::ZERO);
}

#[test]
fn quick_taps_in_one_place_count_up() {
    let start = Instant::now();
    let mut touch = Touch::default();
    assert_eq!(tap(&mut touch, start, 100.0), Some(1));
    assert_eq!(tap(&mut touch, ms(start, 300), 140.0), Some(2));
    assert_eq!(tap(&mut touch, ms(start, 600), 100.0), Some(3));
    assert_eq!(tap(&mut touch, ms(start, 1000), 100.0), Some(1), "пауза дольше 300 мс начинает счёт заново");
    assert_eq!(tap(&mut touch, ms(start, 1200), 400.0), Some(1), "касание дальше 100 dp начинает счёт заново");
}

#[test]
fn a_drag_past_the_slop_scrolls_with_the_finger_without_a_jump() {
    let start = Instant::now();
    let mut touch = Touch::default();
    touch.down(1, Point::new(100.0, 100.0), start, DP);
    touch.moved(1, Point::new(100.0, 110.0), ms(start, 8));
    assert_eq!(scrolled(&mut touch, ms(start, 8)), Vec2::ZERO, "в пределах 8 dp это ещё нажатие");
    touch.moved(1, Point::new(100.0, 130.0), ms(start, 16));
    let frame = touch.frame(ms(start, 16));
    let (anchor, delta) = frame.scroll.expect("прокрутка");
    assert_eq!(anchor, Point::new(100.0, 100.0), "прокрутка идёт в точке касания");
    assert!(near(delta, Vec2::new(0.0, 14.0)), "первый шаг {delta:?} вместо 30 - 16");
    touch.moved(1, Point::new(100.0, 150.0), ms(start, 24));
    assert!(near(scrolled(&mut touch, ms(start, 24)), Vec2::new(0.0, 20.0)));
    assert_eq!(touch.deadline(), None, "прокрутка отменяет долгое нажатие");
    assert_eq!(touch.up(1, Point::new(100.0, 150.0), ms(start, 100)), None);
    assert_eq!(scrolled(&mut touch, ms(start, 108)), Vec2::ZERO, "палец стоял перед отрывом — инерции нет");
    assert!(!touch.active());
}

#[test]
fn holding_still_opens_the_context_menu_once_and_does_not_click() {
    let start = Instant::now();
    let mut touch = Touch::default();
    touch.down(1, Point::new(50.0, 60.0), start, DP);
    assert_eq!(touch.deadline(), Some(start + LONG_PRESS));
    touch.moved(1, Point::new(53.0, 62.0), ms(start, 200));
    assert_eq!(touch.long_press(ms(start, 399)), None);
    assert_eq!(touch.long_press(ms(start, 400)), Some(Point::new(50.0, 60.0)));
    assert_eq!(touch.long_press(ms(start, 500)), None);
    touch.moved(1, Point::new(100.0, 160.0), ms(start, 600));
    assert_eq!(scrolled(&mut touch, ms(start, 600)), Vec2::ZERO, "после долгого нажатия палец не прокручивает");
    assert_eq!(touch.up(1, Point::new(100.0, 160.0), ms(start, 700)), None);
}

#[test]
fn a_second_finger_cancels_the_tap_and_pinches_after_the_span_slop() {
    let start = Instant::now();
    let mut touch = Touch::default();
    touch.down(1, Point::new(0.0, 0.0), start, 1.0);
    touch.down(2, Point::new(200.0, 0.0), ms(start, 10), 1.0);
    assert_eq!(touch.deadline(), None);
    touch.moved(2, Point::new(210.0, 0.0), ms(start, 20));
    assert!(touch.frame(ms(start, 20)).pinch.is_none(), "разлёт в пределах 16 dp ещё не щипок");
    touch.moved(2, Point::new(240.0, 0.0), ms(start, 30));
    touch.moved(2, Point::new(300.0, 0.0), ms(start, 40));
    let frame = touch.frame(ms(start, 40));
    let (focus, delta) = frame.pinch.expect("щипок");
    assert!((delta - 0.25).abs() < 1e-9, "масштаб {delta} вместо 300/240 - 1");
    assert_eq!(focus, Point::new(150.0, 0.0));
    assert!(near(frame.scroll.expect("сдвиг центра").1, Vec2::new(45.0, 0.0)));
    assert_eq!(touch.up(2, Point::new(300.0, 0.0), ms(start, 50)), None);
    assert_eq!(touch.up(1, Point::new(0.0, 0.0), ms(start, 60)), None, "два пальца — не нажатие");
}

#[test]
fn a_flick_keeps_scrolling_at_the_speed_of_the_finger_and_stops() {
    let start = Instant::now();
    let mut touch = Touch::default();
    let end = flick(&mut touch, start, 10.0);
    let steps: Vec<f64> = (1..400).map(|k| scrolled(&mut touch, end + FRAME * k).y).take_while(|&y| y > 0.0).collect();
    assert!((steps[0] / 10.0 - 1.0).abs() < 0.05, "первый шаг инерции {} вместо 10", steps[0]);
    assert!(steps.windows(2).all(|w| w[1] <= w[0]), "инерция не замедляется плавно: {steps:?}");
    assert!(!touch.active(), "инерция не закончилась");
}

#[test]
fn touching_a_running_fling_stops_it_without_a_click() {
    let start = Instant::now();
    let mut touch = Touch::default();
    let end = flick(&mut touch, start, 10.0);
    assert!(scrolled(&mut touch, end + FRAME).y > 0.0);
    let at = end + FRAME * 2;
    touch.down(1, Point::new(300.0, 300.0), at, 1.0);
    assert_eq!(touch.deadline(), None);
    assert_eq!(touch.up(1, Point::new(300.0, 300.0), at + FRAME), None);
    assert_eq!(scrolled(&mut touch, at + FRAME * 2), Vec2::ZERO);
}

#[test]
fn a_cancelled_touch_neither_clicks_nor_presses_long() {
    let start = Instant::now();
    let mut touch = Touch::default();
    touch.down(1, Point::new(10.0, 10.0), start, DP);
    touch.cancel(1);
    assert_eq!(touch.deadline(), None);
    assert_eq!(touch.up(1, Point::new(10.0, 10.0), ms(start, 50)), None);
}
