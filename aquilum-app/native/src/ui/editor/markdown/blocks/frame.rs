use aquilum_core::markdown::indent::Indent;
use aquilum_core::markdown::quote as syntax;

use super::super::super::embed::panel::PAD;
use super::super::metrics::{Fonts, LIST_LEVEL, LIST_START, QUOTE_BAR};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Nest {
    pub level: usize,
    pub width: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Frame {
    pub list: Option<Nest>,
    pub quote: u8,
    pub panels: u8,
    pub panel: u8,
    pub outer: u8,
    pub closes: u8,
    pub inset: usize,
    pub marker: usize,
    pub lead: usize,
}

impl Frame {
    pub fn follow(&self, line: &str) -> Option<Frame> {
        let inset = if self.inset == 0 { 0 } else { Indent::of(line).bytes.min(self.inset) };
        let depth = usize::from(self.quote);
        let marker = match depth {
            0 => inset,
            _ => match syntax::markers(&line[inset..], depth) {
                (marker, found) if found == depth => inset + marker,
                _ => return None,
            },
        };
        Some(Frame { inset, marker, lead: marker, ..*self })
    }

    pub fn plain(self) -> Frame {
        Frame { panels: 0, panel: 0, outer: 0, closes: 0, ..self }
    }

    fn base(&self, fonts: &Fonts) -> f64 {
        self.list.map_or(0.0, |nest| LIST_START + LIST_LEVEL * nest.level as f64 + f64::from(fonts.ch) * nest.width as f64)
    }

    fn at(&self, line: &str, level: u8) -> usize {
        line[self.inset..].match_indices('>').nth(usize::from(level).saturating_sub(1)).map_or(0, |(at, _)| at)
    }

    pub fn cut(&self, line: &str) -> usize {
        match self.panel {
            0 => self.inset,
            panel => self.inset + syntax::markers(&line[self.inset..], usize::from(panel)).0,
        }
    }

    pub fn shell(&self, line: &str, k: u8, fonts: &Fonts) -> f64 {
        self.base(fonts) + self.at(line, self.outer) as f64 * f64::from(fonts.ch) + f64::from(k.saturating_sub(1)) * PAD
    }

    fn left(&self, line: &str, fonts: &Fonts) -> f64 {
        match self.panels {
            0 => self.base(fonts),
            panels => self.shell(line, panels + 1, fonts),
        }
    }

    pub fn origin(&self, line: &str, fonts: &Fonts) -> f64 {
        self.left(line, fonts) + if self.quote > self.panel { QUOTE_BAR } else { 0.0 }
    }

    pub fn right(&self) -> f64 {
        f64::from(self.panels) * PAD
    }

    fn prefix(&self, line: &str, fonts: &Fonts) -> f64 {
        let marks = usize::from(self.quote - self.panel);
        f64::from(fonts.mark) * marks as f64 + f64::from(fonts.space) * (self.marker - self.cut(line) - marks) as f64
    }

    pub fn x(&self, line: &str, fonts: &Fonts) -> f64 {
        self.origin(line, fonts) + self.prefix(line, fonts)
    }

    pub fn bar(&self, line: &str, level: u8, fonts: &Fonts) -> f64 {
        self.left(line, fonts) + (self.inset + self.at(line, level)).saturating_sub(self.cut(line)) as f64 * f64::from(fonts.ch)
    }
}
