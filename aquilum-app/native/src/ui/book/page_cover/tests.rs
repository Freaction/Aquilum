use super::*;
use masonry::peniko::{Extend, ImageAlphaType};

fn solid(width: u32, height: u32, color: [u8; 4]) -> ImageBrush {
    crate::render::from_rgba(color.repeat((width * height) as usize), width, height, ImageAlphaType::AlphaPremultiplied).unwrap()
}

fn frame(art: &mut art::Art, visual: &Visual, size: Size, scale: f64, position: f64, pattern: impl FnOnce() -> Option<cover::Pattern>) -> ImageBrush {
    bake(size, scale, |painter| art.paint(painter, size.to_rect(), scale, None, visual, position, false, pattern)).unwrap()
}

fn pixel(image: &ImageBrush, x: usize, y: usize) -> [u8; 4] {
    let at = (y * image.image.width as usize + x) * 4;
    image.image.data.data()[at..at + 4].try_into().unwrap()
}

#[test]
fn a_ready_pattern_stays_visible_during_resize_and_scale_changes_cover_audit() {
    let _lock = theme::test_lock();
    let mut art = art::Art::default();
    let visual = Visual::Pattern("origami");
    let image = solid(8, 8, [10, 170, 240, 255]).with_x_extend(Extend::Repeat).with_y_extend(Extend::Repeat);
    frame(&mut art, &visual, Size::new(400.0, 120.0), 1.0, 50.0, || Some(cover::Pattern { image, exact: true }));
    for (width, height, scale) in [(600.0, 160.0, 1.0), (300.0, 90.0, 2.625), (500.0, 120.0, 1.75)] {
        let image = frame(&mut art, &visual, Size::new(width, height), scale, 50.0, || None);
        let sample = pixel(&image, (40.0 * scale) as usize, (height * scale / 2.0) as usize);
        assert!(sample[0].abs_diff(10) <= 2 && sample[1].abs_diff(170) <= 2 && sample[2].abs_diff(240) <= 2, "ready cover disappeared at {width}x{height}@{scale}: {sample:?}");
    }
    let replacement = solid(8, 8, [220, 30, 80, 255]).with_x_extend(Extend::Repeat).with_y_extend(Extend::Repeat);
    let size = Size::new(500.0, 120.0);
    let image = frame(&mut art, &visual, size, 1.75, 50.0, || Some(cover::Pattern { image: replacement, exact: true }));
    assert_eq!(pixel(&image, 70, 105), [220, 30, 80, 255], "the exact new cover did not replace the preview");
    frame(&mut art, &visual, size, 1.75, 50.0, || panic!("queried the pattern cache for an unchanged ready frame"));
}

#[test]
fn a_photo_updates_its_crop_and_size_in_the_first_frame_cover_audit() {
    let _lock = theme::test_lock();
    let (width, height) = (120u32, 240u32);
    let pixels: Vec<u8> = (0..height).flat_map(|y| [y as u8, 0, 255 - y as u8, 255].repeat(width as usize)).collect();
    let visual = Visual::Image(crate::render::from_rgba(pixels, width, height, ImageAlphaType::AlphaPremultiplied).unwrap());
    let mut art = art::Art::default();
    for (size, scale, position) in [(Size::new(400.0, 100.0), 1.0, 0.0), (Size::new(600.0, 140.0), 1.75, 100.0), (Size::new(300.0, 80.0), 2.625, 50.0)] {
        let image = frame(&mut art, &visual, size, scale, position, || None);
        let expected = bake(size, scale, |painter| art::body(painter, size.to_rect(), &visual, position, None, scale)).unwrap();
        let actual = image.image.data.data();
        let expected = expected.image.data.data();
        let differ = actual.iter().zip(expected).filter(|(a, b)| a.abs_diff(**b) > 2).count();
        assert!(differ < actual.len() / 100, "photo crop or size lagged behind: {differ} channels");
    }
}
