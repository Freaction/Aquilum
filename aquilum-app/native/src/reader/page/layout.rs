use masonry::core::BrushIndex;
use masonry::kurbo::Size;
use masonry::parley::{Alignment, AlignmentOptions, FontContext, LayoutContext};
use masonry::peniko::ImageBrush;

use super::text::{BOLD, TextStyle, wrapped};
use super::{Item, Settings};
use crate::reader::flow::{Align, Block, Role};

fn heading_scale(level: u8) -> f32 {
    match level {
        1 => 2.0,
        2 => 1.5,
        3 => 1.17,
        5 => 0.83,
        6 => 0.67,
        _ => 1.0,
    }
}

fn alignment(align: Option<Align>, role: Role, justify: bool) -> Alignment {
    match (align, role) {
        (Some(Align::Center), _) | (None, Role::Heading(_)) => Alignment::Center,
        (Some(Align::End), _) => Alignment::End,
        (Some(Align::Start), _) => Alignment::Start,
        (None, Role::Pre) => Alignment::Start,
        _ if justify => Alignment::Justify,
        _ => Alignment::Start,
    }
}

fn spacing(role: Role, size: f32, em: f64) -> (f64, f64) {
    match role {
        Role::Heading(_) => (f64::from(size) * 0.67, f64::from(size) * 0.67),
        Role::Pre | Role::Item | Role::Cell => (em * 0.25, em * 0.25),
        Role::Paragraph => (0.0, 0.0),
    }
}

fn image_item(image: Option<ImageBrush>, width: f64, height: f64, em: f64) -> Item {
    let size = image.as_ref().map_or(Size::ZERO, |i| {
        let (w, h) = (f64::from(i.image.width), f64::from(i.image.height));
        let k = (width / w).min(height / h).min(1.0);
        Size::new(w * k, h * k)
    });
    Item::Image { image, size, before: em * 0.5, after: em * 0.5 }
}

pub fn layout_blocks(
    blocks: &[Block],
    images: &mut dyn FnMut(&str) -> Option<ImageBrush>,
    fcx: &mut FontContext,
    lcx: &mut LayoutContext<BrushIndex>,
    settings: &Settings,
    width: f64,
    height: f64,
) -> Vec<Item> {
    let family = crate::ui::theme::editor_font(&settings.family);
    let mono = crate::ui::theme::editor_font("iA Writer Mono");
    let em = f64::from(settings.size);
    blocks
        .iter()
        .map(|block| match block {
            Block::Text { text, spans, role, depth, indent, align, .. } => {
                let size = match role {
                    Role::Heading(level) => settings.size * heading_scale(*level),
                    _ => settings.size,
                };
                let x = f64::from(*depth) * em * 1.5;
                let wrap = (width - x).max(1.0) as f32;
                let style = TextStyle {
                    font: if *role == Role::Pre { mono.clone() } else { family.clone() },
                    mono: mono.clone(),
                    size,
                    weight: if matches!(role, Role::Heading(_)) { BOLD } else { settings.weight },
                    line_height: settings.line_height,
                    indent: indent.then_some(settings.size),
                };
                let hyphenated = settings.hyphenate && matches!(role, Role::Paragraph | Role::Item | Role::Cell);
                let (mut layout, breaks) = wrapped(fcx, lcx, text, spans, hyphenated, &style, wrap);
                layout.align(Some(wrap), alignment(*align, *role, settings.justify), AlignmentOptions::default());
                let (before, after) = spacing(*role, size, em);
                Item::Text { layout, x, before, after, breaks }
            }
            Block::Image { src, .. } => image_item(images(src), width, height, em),
            Block::Rule => Item::Rule,
        })
        .collect()
}
