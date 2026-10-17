use crate::search::pages::Pages;
use crate::search::headings::{headings, matching};
use crate::text::utf16::utf16_len;

pub fn anchor_offset(text: &str, anchor: &str) -> Option<usize> {
    let anchor = anchor.trim();
    let start = match anchor.strip_prefix('^') {
        Some(id) => block_start(text, id)?,
        None => heading_start(text, anchor)?,
    };
    Some(utf16_len(&text[..start]))
}

pub fn anchor_offset_in(pages: &mut Pages<'_>, anchor: &str) -> std::io::Result<Option<usize>> {
    while let Some(page) = pages.next()? {
        if let Some(local) = anchor_offset(&page.text, anchor) {
            return Ok(Some(page.start_utf16 + local));
        }
    }
    Ok(None)
}

fn heading_start(text: &str, anchor: &str) -> Option<usize> {
    let wanted = anchor.rsplit('#').next().unwrap_or(anchor);
    let outline = headings(text);
    matching(&outline, wanted).first().map(|heading| heading.start)
}

fn block_start(text: &str, id: &str) -> Option<usize> {
    let mark = format!("^{id}");
    let mut offset = 0;
    for line in text.split('\n') {
        let own = line
            .trim_end()
            .strip_suffix(&mark)
            .is_some_and(|rest| rest.is_empty() || rest.ends_with(' '));
        if own {
            return Some(offset);
        }
        offset += line.len() + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::anchor_offset;

    const NOTE: &str = "# Один\n\nтекст ^блок\n\n## Два\n\nещё\n\n### Три\n\n## Два\n";

    #[test]
    fn finds_headings_ignoring_case_and_nesting() {
        assert_eq!(anchor_offset(NOTE, "Два"), Some("# Один\n\nтекст ^блок\n\n".encode_utf16().count()));
        assert_eq!(anchor_offset(NOTE, "один"), Some(0));
        assert_eq!(anchor_offset(NOTE, "Два#Три"), anchor_offset(NOTE, "Три"));
        assert_eq!(anchor_offset(NOTE, "Нет такого"), None);
    }

    #[test]
    fn finds_block_ids_at_the_end_of_a_line() {
        assert_eq!(anchor_offset(NOTE, "^блок"), Some("# Один\n\n".encode_utf16().count()));
        assert_eq!(anchor_offset(NOTE, "^нет"), None);
        assert_eq!(anchor_offset("абв^x", "^x"), None);
    }
}
