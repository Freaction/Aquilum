mod sink;

use krilla::Document;
use krilla::color::rgb;
use krilla::geom::{PathBuilder, Rect as KRect};
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{Fill, FillRule};
use masonry::app::{VisualLayerKind, VisualLayerPlan};
use masonry::imaging::record::replay_transformed;
use masonry::kurbo::{Affine, Rect};
use masonry::peniko::Color;

use sink::{Fonts, PdfSink};

const PAGE: (f64, f64) = (595.28, 841.89);
const MARGIN: f64 = 40.0;
const POINT: f64 = 0.75;

pub fn content_width() -> f64 {
    (PAGE.0 - 2.0 * MARGIN) / POINT
}

pub fn page_height() -> f64 {
    (PAGE.1 - 2.0 * MARGIN) / POINT
}

pub fn pages(breaks: &[f64], height: f64) -> Vec<(f64, f64)> {
    let page = page_height();
    let mut out = Vec::new();
    let mut start = 0.0;
    while start < height - 0.5 {
        let limit = start + page;
        let end = if limit >= height { height } else { breaks.iter().copied().filter(|b| *b > start + 1.0 && *b <= limit).fold(limit, |_, b| b) };
        out.push((start, end));
        start = end;
    }
    out
}

fn rect(area: Rect) -> Option<krilla::geom::Path> {
    let mut builder = PathBuilder::new();
    builder.push_rect(KRect::from_ltrb(area.x0 as f32, area.y0 as f32, area.x1 as f32, area.y1 as f32)?);
    builder.finish()
}

pub fn write(layers: &VisualLayerPlan, pages: &[(f64, f64)], background: Color) -> Result<Vec<u8>, String> {
    let Some(VisualLayerKind::Scene(scene)) = layers.root_layer().map(|l| &l.kind) else { return Err("нет сцены заметки".to_owned()) };
    let mut document = Document::new();
    let mut fonts = Fonts::new();
    let c = background.to_rgba8();
    for &(start, end) in pages {
        let settings = PageSettings::from_wh(PAGE.0 as f32, PAGE.1 as f32).ok_or("размер страницы")?;
        let mut page = document.start_page_with(settings);
        let mut surface = page.surface();
        if let Some(path) = rect(Rect::new(0.0, 0.0, PAGE.0, PAGE.1)) {
            surface.set_fill(Some(Fill { paint: rgb::Color::new(c.r, c.g, c.b).into(), opacity: NormalizedF32::ONE, rule: FillRule::NonZero }));
            surface.draw_path(&path);
        }
        let area = Rect::new(MARGIN, MARGIN, PAGE.0 - MARGIN, MARGIN + (end - start) * POINT);
        let clip = rect(area);
        if let Some(clip) = &clip {
            surface.push_clip_path(clip, &FillRule::NonZero);
        }
        let base = Affine::translate((MARGIN, MARGIN)) * Affine::scale(POINT) * Affine::translate((0.0, -start));
        replay_transformed(scene, &mut PdfSink::new(&mut surface, base, area, &mut fonts), Affine::IDENTITY);
        if clip.is_some() {
            surface.pop();
        }
        surface.finish();
        page.finish();
    }
    document.finish().map_err(|e| format!("{e:?}"))
}
