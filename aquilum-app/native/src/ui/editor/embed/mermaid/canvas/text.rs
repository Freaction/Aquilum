use masonry::core::BrushIndex;
use masonry::kurbo::{Point, Rect};
use masonry::parley::Alignment;

use super::super::super::math::formula;
use super::super::super::super::markdown::SECONDARY;
use super::FONT;
use crate::ui::tokens::size;

pub(super) const CHIP: f64 = 13.0;

fn plain(text: &str, size: f64) -> f64 {
    let measured = mermaid_rs_renderer::measure_text_width(text, size as f32, "");
    f64::from(measured.unwrap_or(text.chars().count() as f32 * size as f32 * 0.6))
}

pub(super) fn pieces(line: &str) -> impl Iterator<Item = (bool, &str)> {
    let parts: Vec<&str> = line.split("$$").collect();
    let n = parts.len();
    parts.into_iter().enumerate().map(move |(i, s)| (i % 2 == 1 && i + 1 < n, s))
}

pub(in super::super) fn width(text: &str, size: f64) -> f64 {
    if !text.contains("$$") {
        return plain(text, size);
    }
    pieces(text).map(|(math, s)| math.then(|| formula(s, size, true)).flatten().map_or_else(|| plain(s, size), |f| f64::from(f.width))).sum()
}

pub(super) fn tall(line: &str, size: f64) -> f64 {
    let maths = pieces(line).filter(|(math, _)| *math).filter_map(|(_, s)| formula(s, size, true));
    maths.map(|f| f64::from(f.ascent + f.depth) + size * 0.5).fold(size * 1.5, f64::max)
}

pub(super) fn aligned(align: Alignment, x: f64, w: f64) -> f64 {
    match align {
        Alignment::Start => x,
        Alignment::End => x - w,
        _ => x - w / 2.0,
    }
}

#[derive(Clone, Debug, Default)]
pub(in super::super) struct Text {
    pub lines: Vec<String>,
    pub width: f64,
}

impl Text {
    pub fn height(&self) -> f64 {
        self.lines.iter().map(|l| tall(l, FONT)).sum()
    }
}

pub(in super::super) fn wrap(text: &str, max: f64) -> Text {
    let mut lines = Vec::new();
    let text = text.replace("<br/>", "\n").replace("<br>", "\n");
    let text = if text.contains("$$") { text } else { text.replace("\\n", "\n") };
    for line in text.split('\n') {
        if line.contains("$$") {
            lines.push(line.trim().to_owned());
            continue;
        }
        let mut current = String::new();
        for word in line.split_whitespace() {
            let longer = if current.is_empty() { word.to_owned() } else { format!("{current} {word}") };
            if current.is_empty() || width(&longer, FONT) <= max {
                current = longer;
            } else {
                lines.push(std::mem::replace(&mut current, word.to_owned()));
            }
        }
        lines.push(current);
    }
    let widest = lines.iter().map(|l| width(l, FONT)).fold(0.0, f64::max);
    Text { lines, width: widest }
}

pub(in super::super) fn single(text: &str) -> Text {
    Text { lines: vec![text.to_owned()], width: width(text, FONT) }
}

pub(in super::super) fn chip_rect(at: Point, text: &Text) -> Rect {
    Label::block(at, text, CHIP, SECONDARY).rect().inflate(size::GAP_MD, size::GAP_XS)
}

pub(in super::super) fn clean(line: &str) -> String {
    let line = line.trim().trim_matches('"');
    if line.matches('~').count() % 2 != 0 || line.contains("$$") {
        return line.to_owned();
    }
    let mut open = false;
    line.chars().map(|c| if c == '~' { open = !open; if open { '<' } else { '>' } } else { c }).collect()
}

pub(in super::super) struct Label {
    pub at: Point,
    pub text: String,
    pub size: f64,
    pub brush: BrushIndex,
    pub bold: bool,
    pub align: Alignment,
    pub width: f64,
    pub chip: bool,
}

impl Label {
    pub fn new(at: Point, text: impl Into<String>, size: f64, brush: BrushIndex) -> Self {
        Label { at, text: text.into(), size, brush, bold: false, align: Alignment::Center, width: 0.0, chip: false }
    }

    pub fn block(at: Point, block: &Text, size: f64, brush: BrushIndex) -> Self {
        let text = block.lines.iter().map(|line| clean(line)).collect::<Vec<_>>().join("\n");
        Label { width: block.width * size / FONT, ..Label::new(at, text, size, brush) }
    }

    pub fn bold(self) -> Self {
        Label { bold: true, ..self }
    }

    pub fn start(self, width: f64) -> Self {
        Label { align: Alignment::Start, width, ..self }
    }

    pub fn beside(self, toward: f64) -> Self {
        Label { align: if toward < 0.0 { Alignment::End } else { Alignment::Start }, ..self }
    }

    pub fn rect(&self) -> Rect {
        let longest = self.text.lines().map(|line| line.chars().count()).max().unwrap_or(0) as f64;
        let w = match (self.width > 0.0, self.text.contains("$$")) {
            (true, _) => self.width,
            (false, true) => self.text.lines().map(|l| width(l, self.size)).fold(0.0, f64::max),
            (false, false) => longest * self.size * 0.6,
        };
        let h = self.text.lines().map(|l| tall(l, self.size)).sum::<f64>().max(self.size * 1.5);
        let x = aligned(self.align, self.at.x, w);
        Rect::new(x, self.at.y - h / 2.0, x + w, self.at.y + h / 2.0)
    }
}
