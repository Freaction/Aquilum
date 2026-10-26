use std::time::{Duration, Instant};

use super::{Fling, Trail};

const FRAME: Duration = Duration::from_micros(16667);

fn after(start: Instant, seconds: f64) -> Instant {
    start + Duration::from_secs_f64(seconds)
}

fn steps(mut fling: Fling, from: Instant) -> Vec<f64> {
    (1..1000).map_while(|k| fling.advance(from + FRAME * k)).map(|(_, y)| y).collect()
}

#[test]
fn quadratic_motion_gives_the_speed_at_the_last_sample() {
    let start = Instant::now();
    let mut trail = Trail::touch();
    for k in 0..6 {
        let t = 0.008 * f64::from(k);
        trail.push(after(start, t), 0.0, 2000.0 * t + 30000.0 * t * t);
    }
    let (vx, vy) = trail.velocity(after(start, 0.04)).expect("скорость по шести точкам");
    assert!(vx.abs() < 1e-6 && (vy - 4400.0).abs() < 1.0, "скорость {vy} вместо 4400 (производная в последней точке)");
}

#[test]
fn two_samples_give_a_straight_line() {
    let start = Instant::now();
    let mut trail = Trail::touch();
    trail.push(start, 0.0, 0.0);
    trail.push(after(start, 0.01), 30.0, 0.0);
    let (vx, _) = trail.velocity(after(start, 0.01)).expect("скорость по двум точкам");
    assert!((vx - 3000.0).abs() < 0.5, "скорость {vx} вместо 3000");
    assert!(Trail::touch().velocity(start).is_none());
}

#[test]
fn a_pause_before_release_means_the_finger_stopped() {
    let start = Instant::now();
    let mut trail = Trail::touch();
    for k in 0..5 {
        trail.push(after(start, 0.01 * f64::from(k)), 0.0, 40.0 * f64::from(k));
    }
    assert!(trail.velocity(after(start, 0.04)).is_some());
    assert!(trail.velocity(after(start, 0.081)).is_none(), "палец стоял дольше 40 мс, а скорость осталась");
}

#[test]
fn a_pause_forgets_the_movement_before_it() {
    let start = Instant::now();
    let mut trail = Trail::touch();
    for k in 0..5 {
        trail.push(after(start, 0.01 * f64::from(k)), 0.0, 40.0 * f64::from(k));
    }
    trail.push(after(start, 0.09), 0.0, 160.0);
    trail.push(after(start, 0.1), 0.0, 160.0);
    let (_, vy) = trail.velocity(after(start, 0.1)).expect("скорость после паузы");
    assert!(vy.abs() < 1e-6, "после паузы скорость {vy} взялась из старого движения");
}

#[test]
fn a_sample_at_the_same_moment_replaces_the_previous_one() {
    let start = Instant::now();
    let mut trail = Trail::touch();
    trail.push(start, 0.0, 0.0);
    trail.push(after(start, 0.01), 0.0, 10.0);
    trail.push(after(start, 0.01), 0.0, 20.0);
    let (_, vy) = trail.velocity(after(start, 0.01)).expect("скорость");
    assert!((vy - 2000.0).abs() < 0.5, "скорость {vy} вместо 2000");
}

#[test]
fn a_fling_travels_as_far_and_as_long_as_on_android() {
    let start = Instant::now();
    let steps = steps(Fling::spline(start, (0.0, 1000.0), 1.0), start);
    let path: f64 = steps.iter().sum();
    let ms = steps.len() as f64 * FRAME.as_secs_f64() * 1000.0;
    assert!((path - 194.3).abs() < 0.5, "путь {path} вместо 194,3 (OverScroller.getSplineFlingDistance)");
    assert!((555.0..=572.0).contains(&ms), "длительность {ms} мс вместо 555 (OverScroller.getSplineFlingDuration)");
}

#[test]
fn a_fling_starts_at_the_speed_of_the_finger_and_slows_down_smoothly() {
    let start = Instant::now();
    let steps = steps(Fling::spline(start, (0.0, 3000.0), 2.0), start);
    let first = 3000.0 * FRAME.as_secs_f64();
    assert!((0.95..=1.0).contains(&(steps[0] / first)), "первый шаг {} вместо {first}", steps[0]);
    assert!(steps.windows(2).all(|w| w[1] <= w[0] && w[1] >= 0.0), "шаги не убывают: {steps:?}");
    assert!(*steps.last().unwrap() < 1.0, "остановка обрывом: {:?}", &steps[steps.len() - 3..]);
}

#[test]
fn a_fling_keeps_its_direction() {
    let start = Instant::now();
    let mut fling = Fling::spline(start, (-300.0, 400.0), 1.0);
    let (x, y) = fling.advance(start + FRAME).expect("шаг");
    assert!((x / y + 0.75).abs() < 1e-9, "направление {x}:{y} вместо -3:4");
}
