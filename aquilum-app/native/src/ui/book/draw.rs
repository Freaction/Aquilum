use super::*;

pub(super) type BakeKey = (u16, u16, usize, u64, i64);

pub(super) fn theme_id() -> usize {
    theme::generation()
}

pub(super) fn bake(size: Size, scale: f64, draw: impl FnOnce(&mut Painter<'_>)) -> Option<ImageBrush> {
    let mut scene = Scene::new();
    {
        let mut recorder = Painter::new(&mut scene);
        draw(&mut recorder.as_dyn());
    }
    let (dw, dh) = device(Rect::from_origin_size(Point::ORIGIN, size), scale);
    crate::render::offscreen(dw, dh, |painter| replay_transformed(&scene, painter.sink_mut(), Affine::scale(scale)))
}

pub(super) fn cover_body(painter: &mut Painter<'_>, bounds: Rect, visual: &Visual, position: f64, pattern: Option<&ImageBrush>, scale: f64) {
    let tm = theme::current();
    let shape = RoundedRect::from_rect(bounds, size::BOOK_COVER_RADIUS);
    painter.with_fill_clip(shape, |painter| {
        painter.fill(bounds, tm.bg_sunken).draw();
        match visual {
            Visual::Pattern(id) => {
                painter.fill(bounds, cover::base_color(id)).draw();
                if let Some(image) = pattern {
                    let (w, h) = (f64::from(image.image.width).max(1.0), f64::from(image.image.height).max(1.0));
                    let (kx, ky) = ((bounds.width() * scale / w).max(1.0) / scale, (bounds.height() * scale / h).max(1.0) / scale);
                    painter.draw_image(image, Affine::translate(bounds.origin().to_vec2()) * Affine::scale_non_uniform(kx, ky));
                }
            }
            Visual::Image(image) => {
                let (w, h) = (f64::from(image.image.width), f64::from(image.image.height));
                let k = (bounds.width() / w).max(bounds.height() / h);
                let (dw, dh) = (w * k, h * k);
                let x = bounds.x0 + (bounds.width() - dw) / 2.0;
                let y = bounds.y0 + (bounds.height() - dh) * position / 100.0;
                painter.draw_image(image, Affine::translate((x, y)) * Affine::scale(k));
            }
        }
        if !theme::is_dark() {
            paint_rays(painter, Rect::new(bounds.x1 - RAYS_WIDTH, bounds.y0, bounds.x1, bounds.y1), scale);
        }
    });
}

pub(super) fn linear(from: Point, to: Point, stops: &[(f32, Color)]) -> Gradient {
    Gradient::new_linear(from, to).with_stops(stops.iter().map(|(o, c)| ColorStop::from((*o, *c))).collect::<Vec<_>>().as_slice())
}

pub(super) fn cached(kind: u8, width: u32, height: u32, make: impl FnOnce() -> Option<ImageBrush>) -> Option<ImageBrush> {
    static CACHE: std::sync::Mutex<Vec<((u8, u32, u32), ImageBrush)>> = std::sync::Mutex::new(Vec::new());
    let key = (kind, width, height);
    if let Some((_, image)) = CACHE.lock().ok()?.iter().find(|(k, _)| *k == key) {
        return Some(image.clone());
    }
    let image = make()?;
    let mut cache = CACHE.lock().ok()?;
    if cache.len() >= 8 {
        cache.remove(0);
    }
    cache.push((key, image.clone()));
    Some(image)
}

pub(super) fn device(rect: Rect, scale: f64) -> (u16, u16) {
    ((rect.width() * scale).ceil().clamp(1.0, 8192.0) as u16, (rect.height() * scale).ceil().clamp(1.0, 8192.0) as u16)
}

pub(super) fn paint_rays(painter: &mut Painter<'_, impl PaintSink + ?Sized>, rect: Rect, scale: f64) {
    let transform = Affine::translate(rect.origin().to_vec2()) * Affine::scale_non_uniform(rect.width() / 311.0, rect.height() / 186.0);
    let white = Color::WHITE;
    let overlay = Composite::new(BlendMode::new(Mix::Overlay, Compose::SrcOver), 0.5);
    painter.with_group(GroupRef::new().with_composite(overlay), |painter| {
        let gradient = linear(Point::new(311.0, 93.0), Point::new(0.0, 93.0), &[(0.0, white), (1.0, white.with_alpha(0.0))]);
        painter.fill(Rect::new(0.0, 0.0, 311.0, 186.0), &gradient).transform(transform).draw();
    });
    let (dw, dh) = device(rect, scale);
    let Some(rays) = cached(0, u32::from(dw), u32::from(dh), || {
        crate::render::offscreen(dw, dh, |painter| {
            let local = Affine::scale_non_uniform(f64::from(dw) / 311.0, f64::from(dh) / 186.0);
            let gray = (Color::from_rgb8(0x83, 0x83, 0x83), Color::from_rgb8(0x4d, 0x4d, 0x4d));
            let light = (Color::WHITE, Color::from_rgb8(0x8c, 0x8c, 0x8c));
            let rays: [(&str, f32, f32, (f64, f64, f64, f64), (Color, Color)); 7] = [
                ("M394.487 -2.94443L237.769 -19.0234L189.579 186.356L394.487 -2.94443Z", 30.0, 1.0, (324.26, -47.1334, 360.017, 133.647), gray),
                ("M298.919 -11.7232L266.071 -63.7713L75.1033 167.198L298.919 -11.7232Z", 20.0, 1.0, (316.004, -78.4749, 325.394, 134.579), gray),
                ("M285.505 -41.0939L270.508 -95.0064L6.92272 126.088L285.505 -41.0939Z", 20.0, 1.0, (324.323, -107.045, 322.222, 107.479), gray),
                ("M353.567 -17.1869L300.139 -18.4095L180.048 181.563L353.567 -17.1869Z", 18.7118, 1.0, (314.1, -51.9018, 349.819, 129.038), light),
                ("M392.156 -33.344L369.785 -44.0036L290.1 175.465L392.156 -33.344Z", 9.0, 0.6, (380.534, -81.8194, 411.882, 135.464), light),
                ("M309.19 -44.1356L294.329 -56.8787L86.597 152.525L309.19 -44.1356Z", 10.0, 0.6, (327.289, -93.2525, 336.494, 120.126), light),
                ("M306.296 -73.0126L295.939 -86.5797L23.0478 112.006L306.296 -73.0126Z", 7.5, 0.6, (340.278, -121.243, 337.971, 93.6527), light),
            ];
            for (path, blur, opacity, (x1, y1, x2, y2), (a, b)) in rays {
                let Ok(path) = BezPath::from_svg(path) else { continue };
                let filters = [Filter::blur(blur * (f64::from(dh) / 186.0) as f32)];
                let group = GroupRef::new().with_filters(&filters).with_composite(Composite::new(BlendMode::default(), opacity));
                painter.with_group(group, |painter| {
                    let gradient = linear(Point::new(x1, y1), Point::new(x2, y2), &[(0.0, a), (1.0, b)]);
                    painter.fill(&path, &gradient).transform(local).draw();
                });
            }
        })
    }) else {
        return;
    };
    let dodge = Composite::new(BlendMode::new(Mix::ColorDodge, Compose::SrcOver), 0.7);
    painter.with_group(GroupRef::new().with_composite(dodge).with_clip(ClipRef::fill(rect)), |painter| {
        painter.draw_image(&rays, Affine::translate(rect.origin().to_vec2()) * Affine::scale(1.0 / scale));
    });
}
