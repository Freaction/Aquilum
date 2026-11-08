use super::{Area, History};

#[test]
fn software_canvas_releases_excess_capacity_and_keeps_its_background_cover_audit() {
    let mut pixels = super::Pixels::new();
    pixels.set_background(0xff123456);
    pixels.resize(2048, 1024);
    let capacity = pixels.data.capacity();
    pixels.resize(2048, 900);
    assert_eq!(pixels.data.capacity(), capacity, "small resizes must reuse the allocation");
    pixels.resize(320, 240);
    assert!(pixels.sized(320, 240));
    assert_eq!(pixels.data.capacity(), pixels.data.len());
    let mut target = vec![0; 321 * 240];
    pixels.copy_to(&mut target, 321, Area { x: 0, y: 0, w: 320, h: 240 });
    assert_eq!(target[319], 0xff123456);
    assert_eq!(target[320], 0, "stride padding was overwritten");
    pixels.trim();
    assert_eq!(pixels.data.capacity(), 0);
    assert!(pixels.fresh);
}

#[test]
fn repeated_damage_is_copied_once_scroll_audit() {
    let column = Area { x: 20, y: 40, w: 700, h: 900 };
    let mut history = History::default();
    history.frame(&[column], 0);
    history.frame(&[column], 0);
    let (_, missing) = history.frame(&[column], 3);
    assert_eq!(missing, Some(vec![column]), "the same column was uploaded more than once");
}

#[test]
fn overlapping_damage_keeps_every_changed_pixel_scroll_audit() {
    let mut history = History::default();
    let areas = [Area { x: 10, y: 20, w: 100, h: 200 }, Area { x: 10, y: 120, w: 100, h: 200 }, Area { x: 10, y: 220, w: 100, h: 200 }];
    for area in &areas[..2] { history.frame(&[*area], 0); }
    let (_, missing) = history.frame(&areas[2..], 3);
    assert_eq!(missing, Some(vec![Area { x: 10, y: 20, w: 100, h: 400 }]));
}

#[cfg(target_os = "windows")]
#[test]
fn canvas_capacity_releases_a_large_window_with_hysteresis_scroll_audit() {
    use super::device::capacity;
    assert_eq!(capacity(1000, 4096), 1024);
    assert_eq!(capacity(2000, 2048), 2048);
    assert_eq!(capacity(2100, 2048), 2304);
    assert_eq!(capacity(1100, 2048), 2048, "ordinary resize must not churn textures");
}
