use std::collections::HashSet;

use masonry::core::BrushIndex;
use masonry::parley::Layout;

use super::super::embed::View;
use super::{Document, Paragraph};

impl Document {
    pub fn probe_stats(&self) -> String {
        let laid: Vec<&Layout<BrushIndex>> = self.paragraphs.iter().filter_map(|p| p.layout.as_deref()).collect();
        let lines: usize = laid.iter().map(|l| l.len()).sum();
        let glyphs: usize = laid.iter().flat_map(|l| l.lines()).map(|line| line.runs().map(|run| run.clusters().count()).sum::<usize>()).sum();
        let embeds: Vec<&View> = self.paragraphs.iter().filter_map(|p| p.embed.as_deref()).map(|e| &e.view).collect();
        let tables = embeds.iter().filter(|v| matches!(v, View::Table(_))).count();
        let media = embeds.iter().filter(|v| matches!(v, View::Media(_))).count();
        let drawings = embeds.iter().filter(|v| matches!(v, View::Drawing(_))).count();
        let decor: usize = self.paragraphs.iter().map(|p| p.decor().codes.len() + p.decor().tags.len() + p.decor().links.len()).sum();
        let mut seen = HashSet::new();
        let pixels: usize = embeds.iter().filter_map(|v| match v { View::Media(m) => m.image.as_ref(), _ => None }).filter(|i| seen.insert(i.image.data.id())).map(|i| i.image.data.len()).sum();
        format!(
            "редактор: текст {:.1} МБ · абзацев {} (по {} Б, всего {:.1} МБ) · сверстано {} · строк {} · кластеров {} · вставок: таблиц {tables}, картинок {media}, рисунков {drawings} · картинок в памяти {} шт. {:.1} МБ · отметок оформления {decor}",
            self.text.len() as f64 / 1_048_576.0,
            self.paragraphs.len(),
            size_of::<Paragraph>(),
            (self.paragraphs.capacity() * size_of::<Paragraph>()) as f64 / 1_048_576.0,
            laid.len(),
            lines,
            glyphs,
            seen.len(),
            pixels as f64 / 1_048_576.0,
        )
    }

    pub fn probe_evict(&mut self, keep: (f64, f64), embeds: bool) {
        for p in &mut self.paragraphs {
            if p.top + p.height < keep.0 || p.top > keep.1 {
                p.unbuild();
                if embeds {
                    p.embed = None;
                }
            }
        }
        if embeds {
            self.paragraphs.shrink_to_fit();
        }
    }
}
