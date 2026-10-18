use std::borrow::Cow;

use masonry::core::{BrushIndex, StyleSet};
use masonry::parley::{Alignment, FontContext, Layout, LayoutContext};

use super::super::embed::panel;
use super::super::highlight::Lang;
use super::super::markdown::{self, Block, Decor, Fonts};
use super::{Document, text};

pub(super) static PLAIN: Decor = Decor { x: 0.0, inset: 0.0, pad: (0.0, 0.0), hang: None, nowrap: false, codes: Vec::new(), marks: Vec::new(), tags: Vec::new(), links: Vec::new(), caret: None, boxes: Vec::new(), pictures: Vec::new() };

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
        markdown::Context { fonts, unresolved: &self.unresolved.0, resolver: self.resolver.as_deref(), inside: false, gutter: 0 }
    }

    pub(super) fn local_caret(&self, line: usize, caret: Option<usize>) -> Option<usize> {
        let range = self.span(line);
        self.markdown.as_ref().and(caret).filter(|c| range.start <= *c && *c <= range.end).map(|c| c - range.start)
    }

    fn gutter(&self, line: usize) -> usize {
        let block = |j: usize| &self.blocks.get(j).1;
        let body = |j: usize| matches!(block(j), Block::Code { fence: false, lang, .. } if *lang != Some(Lang::Math));
        if !body(line) {
            return 0;
        }
        let (first, last, digits) = self.gutters.get();
        if (first..=last).contains(&self.start(line)) {
            return digits;
        }
        let (first, last) = self.around(line, body, |j| matches!(block(j), Block::Code { first: true, .. }));
        let digits = panel::digits(last - first + 1);
        self.gutters.set((self.start(first), self.start(last), digits));
        digits
    }

    pub(super) fn lay(&self, line: usize, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, composing: Option<(usize, &str)>, caret: Option<usize>) -> Laid {
        let range = self.span(line);
        let source = self.line(line);
        let (text, underline) = match composing {
            Some((at, preedit)) => {
                let local = at - range.start;
                (Cow::Owned(format!("{}{preedit}{}", &source[..local], &source[local..])), Some(local..local + preedit.len()))
            }
            None => (Cow::Borrowed(source), None),
        };
        let inside = self.inside(line);
        let gutter = self.gutter(line);
        let context = self.markdown.as_ref().map(|fonts| markdown::Context { inside, gutter, ..self.context(fonts) });
        let (frame, block) = (self.view(line), &self.blocks.get(line).1);
        let markdown = context.as_ref().map(|context| (context, &frame, block, self.local_caret(line, caret)));
        let (layout, decor) = text::build(fcx, lcx, styles, &text, underline, self.width, Alignment::Start, markdown);
        Laid { layout, decor }
    }
}
