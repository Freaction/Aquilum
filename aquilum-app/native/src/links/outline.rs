use aquilum_core::markdown::fence::Fence;
use aquilum_core::markdown::line::heading;
use aquilum_core::search::headings::heading_at;
use aquilum_core::text::cancel::Cancel;
use crop::RopeSlice;

use super::{Item, Listing, Target};
use crate::i18n::{t, t_with};

const LIMIT: usize = 2000;

pub fn outline(generation: u64, body: RopeSlice<'_>, cancel: &Cancel) -> Option<Listing> {
    let (mut fence, mut at, mut found) = (None, 0, Vec::new());
    crate::rope::each_line(body, |line| {
        let start = at;
        at += line.len() + 1;
        if !matches!(line.trim_start_matches([' ', '\t']).bytes().next(), Some(b'#' | b'`' | b'~')) {
            return !cancel.is_cancelled();
        }
        let inside = fence.is_some();
        fence = Fence::step(fence, line, usize::MAX);
        if !inside && fence.is_none()
            && let Some((level, _)) = heading(line)
            && let Some(title) = heading_at(line, 0).filter(|title| !title.is_empty())
        {
            found.push((level, title, start..start + line.trim_end().len()));
        }
        found.len() < LIMIT && !cancel.is_cancelled()
    });
    if cancel.is_cancelled() {
        return None;
    }
    let top = found.iter().map(|(level, ..)| *level).min().unwrap_or(1);
    let notice = match found.len() {
        0 => Some(t("outline.empty")),
        LIMIT => Some(t_with("outline.limited", &[("count", &LIMIT.to_string())])),
        _ => None,
    };
    let items = found
        .into_iter()
        .map(|(level, title, range)| Item { title, counter: None, target: Target::Spot(range), reasons: Vec::new(), depth: usize::from(level - top) })
        .collect();
    Some(Listing { generation, items, notice })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(text: &str) -> Vec<(String, usize, String)> {
        let rope = crop::Rope::from(text);
        outline(1, rope.byte_slice(..), &Cancel::never())
            .unwrap()
            .items
            .into_iter()
            .map(|item| match item.target {
                Target::Spot(range) => (item.title, item.depth, text[range].to_owned()),
                other => panic!("{other:?}"),
            })
            .collect()
    }

    #[test]
    fn headings_outside_code_with_levels_from_the_top_one() {
        let text = "## Глава\nтекст\n```\n# не заголовок\n```\n### **Жирный** [[ссылка]]\r\n#тег\n## \n## Вторая";
        assert_eq!(
            rows(text),
            [
                ("Глава".to_owned(), 0, "## Глава".to_owned()),
                ("Жирный ссылка".to_owned(), 1, "### **Жирный** [[ссылка]]".to_owned()),
                ("Вторая".to_owned(), 0, "## Вторая".to_owned()),
            ]
        );
    }

    #[test]
    fn a_note_without_headings_says_so() {
        let listing = outline(7, crop::Rope::from("просто текст").byte_slice(..), &Cancel::never()).unwrap();
        assert_eq!((listing.generation, listing.items.len(), listing.notice.is_some()), (7, 0, true));
    }

    #[test]
    fn a_newer_request_stops_the_older_scan() {
        let generation = aquilum_core::text::cancel::Generation::default();
        let older = generation.next();
        let _newer = generation.next();
        assert!(outline(1, crop::Rope::from("# Заголовок").byte_slice(..), &older).is_none());
    }
}
