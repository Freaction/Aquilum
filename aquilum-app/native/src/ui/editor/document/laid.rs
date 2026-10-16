use std::borrow::Cow;

use masonry::core::{BrushIndex, StyleSet};
use masonry::parley::{Alignment, FontContext, Layout, LayoutContext};

use super::super::markdown::{self, Decor, Fonts};
use super::{Document, text};

pub(super) static PLAIN: Decor = Decor { x: 0.0, inset: 0.0, pad: (0.0, 0.0), hang: None, codes: Vec::new(), marks: Vec::new(), tags: Vec::new(), links: Vec::new(), caret: None, boxes: Vec::new(), pictures: Vec::new() };

pub(super) struct Laid {
    pub(super) layout: Layout<BrushIndex>,
    pub(super) decor: Decor,
}

impl Laid {
    pub(super) fn height(&self) -> f64 {
        f64::from(self.layout.height()) + self.decor.pad.0 + self.decor.pad.1
    }
}

impl Document {
    pub(super) fn context<'a>(&'a self, fonts: &'a Fonts) -> markdown::Context<'a> {
        markdown::Context { fonts, unresolved: &self.unresolved.0, resolver: self.resolver.as_deref() }
    }

    pub(super) fn local_caret(&self, line: usize, caret: Option<usize>) -> Option<usize> {
        let range = self.span(line);
        self.markdown.as_ref().and(caret).filter(|c| range.start <= *c && *c <= range.end).map(|c| c - range.start)
    }

    pub(super) fn lay(&self, line: usize, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, composing: Option<(usize, &str)>, caret: Option<usize>) -> Laid {
        let range = self.span(line);
        let source = &self.text[self.lines.range(line)];
        let (text, underline) = match composing {
            Some((at, preedit)) => {
                let local = at - range.start;
                (Cow::Owned(format!("{}{preedit}{}", &source[..local], &source[local..])), Some(local..local + preedit.len()))
            }
            None => (Cow::Borrowed(source), None),
        };
        let context = self.markdown.as_ref().map(|fonts| self.context(fonts));
        let markdown = context.as_ref().map(|context| (context, self.blocks.get(line), self.local_caret(line, caret)));
        let (layout, decor) = text::build(fcx, lcx, styles, &text, underline, self.width, Alignment::Start, markdown);
        Laid { layout, decor }
    }
}
