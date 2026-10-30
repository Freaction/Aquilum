use masonry::core::{BrushIndex, StyleProperty};
use masonry::parley::{FontStyle, RangedBuilder};

use super::super::super::embed::panel::{self, PAD};
use super::super::super::highlight::{self, Lang, Token};
use super::super::Decor;
use super::super::blocks::Frame;
use super::super::brushes::{SECONDARY, SYNTAX, token_brush};
use super::{Context, indent};
use super::builder::{hide, push};

pub(super) fn style_code(builder: &mut RangedBuilder<'_, BrushIndex>, frame: &Frame, text: &str, (first, last, fence, lang): (bool, bool, bool, Option<Lang>), context: &Context, decor: &mut Decor) {
    let (fonts, inside) = (context.fonts, context.inside);
    let (lead, body) = (frame.lead, &text[frame.lead..]);
    let gutter = if fence { 0.0 } else { panel::gutter(context.gutter, fonts.ch) };
    indent(builder, frame, PAD + gutter, decor);
    decor.inset = decor.x + PAD + frame.right();
    let (header, math) = (panel::header_pad(fonts.line), lang == Some(Lang::Math));
    decor.pad = match (first, fence) {
        (true, true) if !math => (header, header + PAD),
        _ => (if first { PAD } else { 0.0 }, if last { PAD } else { 0.0 }),
    };
    decor.pad.1 += f64::from(frame.closes) * PAD;
    if !fence {
        decor.nowrap = !math && !context.print;
        push(builder, StyleProperty::FontFamily(fonts.mono.clone()), lead..text.len());
        if let Some(lang) = lang.filter(|_| !math) {
            for (range, token) in highlight::tokens(body, lang) {
                let range = range.start + lead..range.end + lead;
                push(builder, StyleProperty::Brush(token_brush(token)), range.clone());
                if token == Token::Comment {
                    push(builder, StyleProperty::FontStyle(FontStyle::Italic), range);
                }
            }
        }
        return;
    }
    let ticks = lead + body.len() - body.trim_start_matches(['`', '~', '$']).len();
    push(builder, StyleProperty::Brush(SECONDARY), ticks..text.len());
    if inside {
        push(builder, StyleProperty::Brush(SYNTAX), lead..ticks);
    } else {
        hide(builder, lead..ticks, 0.0);
    }
}
