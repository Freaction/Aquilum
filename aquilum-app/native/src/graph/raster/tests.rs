use super::{Disc, Edge, Sprite, Stamp, bands};

fn at(pixels: &[u8], width: usize, x: usize, y: usize) -> [u8; 3] {
    let i = (y * width + x) * 4;
    [pixels[i], pixels[i + 1], pixels[i + 2]]
}

#[test]
fn draws_discs_lines_and_sprites_in_every_band() {
    let sprite = Sprite { width: 2, height: 2, fill: vec![255; 4], halo: vec![0; 4] };
    let edge = Edge { from: [10.5, 10.5], to: [290.5, 190.5], color: [0, 255, 0], alpha: 1.0 };
    let disc = Disc { center: [150.0, 100.0], radius: 20.0, body: [255, 0, 0], rim: [255, 255, 255], inner: 18.0, alpha: 1.0 };
    let stamp = Stamp { sprite: &sprite, x: 5, y: 130, fill: [0, 0, 255], halo: [0, 0, 0], alpha: 1.0 };
    let mut pixels = vec![0u8; 300 * 200 * 4];
    bands(300, 200, &mut pixels, |band| {
        band.line(&edge);
        band.disc(&disc);
        band.stamp(&stamp);
    });
    assert_eq!(at(&pixels, 300, 150, 100), [255, 0, 0]);
    assert_eq!(at(&pixels, 300, 150, 81), [255, 255, 255]);
    assert_eq!(at(&pixels, 300, 5, 130), [0, 0, 255]);
    assert!(at(&pixels, 300, 270, 177)[1] > 100 || at(&pixels, 300, 270, 178)[1] > 100);
    assert_eq!(at(&pixels, 300, 299, 0), [0, 0, 0]);
}
