use std::collections::HashSet;

use crate::stats::mb;

use super::super::embed::View;
use super::{Document, Embed};

impl Document {
    pub fn embeds(&self) -> impl Iterator<Item = &Embed> {
        self.embeds.iter().map(|(_, embed)| embed)
    }

    pub fn probe_stats(&self) -> String {
        let laid: Vec<_> = self.laid.iter().map(|(_, laid)| laid).collect();
        let lines: usize = laid.iter().map(|l| l.layout.len()).sum();
        let glyphs: usize = laid.iter().flat_map(|l| l.layout.lines()).map(|line| line.runs().map(|run| run.clusters().count()).sum::<usize>()).sum();
        let views: Vec<&View> = self.embeds().map(|e| &e.view).collect();
        let tables = views.iter().filter(|v| matches!(v, View::Table(_))).count();
        let media = views.iter().filter(|v| matches!(v, View::Media(_))).count();
        let drawings = views.iter().filter(|v| matches!(v, View::Drawing(_))).count();
        let decor: usize = laid.iter().map(|l| l.decor.codes.len() + l.decor.tags.len() + l.decor.links.len()).sum();
        let mut seen = HashSet::new();
        let pixels: usize = views.iter().filter_map(|v| match v { View::Media(m) => m.image.as_ref(), _ => None }).filter(|i| seen.insert(i.image.data.id())).map(|i| i.image.data.len()).sum();
        let columns = self.lines.bytes() + self.blocks.bytes() + self.heights.bytes();
        format!(
            "редактор: текст {:.1} МБ · строк {} (столбцы {:.1} МБ) · сверстано {} · строк вёрстки {} · кластеров {} · вставок: таблиц {tables}, картинок {media}, рисунков {drawings} · картинок в памяти {} шт. {:.1} МБ · отметок оформления {decor}",
            mb(self.text.len()),
            self.lines.count(),
            mb(columns),
            laid.len(),
            lines,
            glyphs,
            seen.len(),
            mb(pixels),
        )
    }

    pub fn probe_evict(&mut self, keep: (f64, f64), embeds: bool) {
        let heights = &self.heights;
        self.laid.retain(|i| heights.near(i, keep));
        if embeds {
            self.embeds.retain(|i| heights.near(i, keep));
        }
    }
}
