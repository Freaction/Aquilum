use super::{Pages, MIN_PAGE};

fn read_all(text: &str) -> (Vec<super::Page>, [u8; 32]) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.md");
    std::fs::write(&path, text).unwrap();
    let mut pages = Pages::open(&path).unwrap();
    let mut all = Vec::new();
    while let Some(page) = pages.next().unwrap() {
        all.push(page);
    }
    let joined: String = all.iter().map(|page| page.text.as_str()).collect();
    (all, *blake3::hash(joined.as_bytes()).as_bytes())
}

#[test]
fn a_small_note_is_one_page_with_the_hash_of_its_text() {
    let (pages, hash) = read_all("строка\r\nвторая\n");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].text, "строка\nвторая\n");
    assert_eq!(hash, *blake3::hash("строка\nвторая\n".as_bytes()).as_bytes());
    assert_eq!(read_all("").0.len(), 1, "у пустой заметки есть страница для названия");
}

#[test]
fn pages_cover_the_text_and_never_cut_a_code_block() {
    let block = |i: usize| format!("Абзац {i} с текстом.\n\n```\n{}\n\nконец кода\n```\n\n", "код\n".repeat(64));
    let text: String = (0..MIN_PAGE * 6 / block(0).len()).map(block).collect();
    let (pages, hash) = read_all(&text);
    assert!(pages.len() > 2, "{}", pages.len());
    assert_eq!(pages.iter().map(|page| page.text.as_str()).collect::<String>(), text);
    assert_eq!(hash, *blake3::hash(text.as_bytes()).as_bytes());
    for page in &pages[1..] {
        assert!(page.text.starts_with("Абзац") || page.text.starts_with("```"), "{:?}", &page.text[..20]);
        assert_eq!(page.start_line, text[..page.start].matches('\n').count());
    }
}

#[test]
fn an_edit_moves_only_the_pages_around_it() {
    let paragraphs: String = (0..60_000).map(|i| format!("Абзац {i} с текстом, который повторяется.\n\n")).collect();
    let tasks: String = (0..120_000).map(|i| format!("- [ ] дело {i} без пустых строк\n")).collect();
    for text in [paragraphs, tasks] {
        let (before, _) = read_all(&text);
        for at in [0, text.len() / 2 + text.as_bytes()[text.len() / 2..].iter().position(|byte| *byte == b'\n').unwrap() + 1] {
            let mut edited = text.clone();
            edited.insert_str(at, "вставка ");
            let (after, _) = read_all(&edited);
            let same = after.iter().filter(|page| before.iter().any(|old| old.hash == page.hash)).count();
            assert!(after.len() > 4 && after.len() - same <= 3, "изменилось {} из {}", after.len() - same, after.len());
        }
    }
}

#[test]
fn a_line_longer_than_the_limit_is_read_in_parts_without_breaking_letters() {
    let text = format!("{}\r\n{}\n", "a".repeat(super::MAX_LINE - 1), "€".repeat(400_000));
    let (pages, hash) = read_all(&text);
    let normalized = text.replace("\r\n", "\n");
    assert_eq!(pages.iter().map(|page| page.text.as_str()).collect::<String>(), normalized);
    assert_eq!(hash, *blake3::hash(normalized.as_bytes()).as_bytes());
    assert_eq!(pages.last().unwrap().chars.end, normalized.chars().count());
}
