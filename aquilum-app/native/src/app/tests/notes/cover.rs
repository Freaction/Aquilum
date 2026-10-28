use super::*;

#[test]
fn cover_actions_rewrite_frontmatter() {
    use crate::ui::widgets::Pressed;
    let mut h = Harness::new();
    let note = h.write_note("Книга.md", "---\ncover: true\ntype: book\nPage_cover: pattern:polka\nBook_cover: Files/x.png\n---\nтекст");
    h.open(note.clone());
    let hero = |h: &Harness| h.app.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()).map(|(_, ids)| (ids.random, ids.remove, ids.thumbnail.map(|t| t.id())));
    let (random, _, _) = hero(&h).expect("обложка и книга");
    h.act(random.unwrap(), Pressed);
    let fields = crate::frontmatter::parse(&h.app.text.to_string()).unwrap();
    let cover = fields.text(crate::frontmatter::PAGE_COVER).unwrap().to_owned();
    let id = cover.strip_prefix("pattern:").expect("узор записан с префиксом, как в Таури");
    assert!(crate::cover::PATTERN_IDS.contains(&id) && id != "polka");
    h.save_now();
    assert_eq!(std::fs::read_to_string(&note).unwrap(), h.app.text.to_string());

    let (_, remove, _) = hero(&h).unwrap();
    h.act(remove.unwrap(), Pressed);
    assert!(!h.app.text.to_string().contains("cover: true") && !h.app.text.to_string().contains("Page_cover"));
    let (_, _, thumbnail) = hero(&h).unwrap();
    h.act(thumbnail.unwrap(), crate::ui::book::ThumbAction::Remove);
    assert_eq!(h.app.text.to_string(), "---\ntype: book\n---\nтекст");
    assert!(hero(&h).is_some_and(|(random, _, _)| random.is_none()));
}

#[test]
fn editing_cover_fields_in_metadata_keeps_rendering() {
    let mut h = Harness::new();
    let text = "---\ncover: true\ntype: book\nPage_cover: pattern:polka\nBook_cover: Files/x.png\n---\nтекст";
    let note = h.write_note("Книга.md", text);
    h.open(note);
    let toggle = h.app.note_ids.as_ref().unwrap().metadata.id();
    h.act(toggle, crate::ui::note::MetadataToggled);
    let mut renderer = crate::render::Renderer::new(2);
    let mut current = text.to_owned();
    for field in ["Page_cover: pattern:polka\n", "Book_cover: Files/x.png\n", "cover: true\n", "type: book\n"] {
        let start = current.find(field).unwrap();
        for _ in 0..field.chars().count() {
            let ch = current[start..].chars().next().unwrap();
            current.replace_range(start..start + ch.len_utf8(), "");
            h.replace_body(&current);
            h.app.tick(&mut h.root, std::time::Instant::now() + std::time::Duration::from_secs(3600));
            for _ in 0..3 {
                std::thread::sleep(std::time::Duration::from_millis(15));
                h.app.on_core_events(&mut h.root);
                let (layers, _) = h.root.redraw();
                renderer.render(&layers, W as u16, H as u16, 1.0, theme::background()).expect("кадр");
            }
        }
    }
    assert_eq!(h.app.text.to_string(), "---\n---\nтекст");
}

#[test]
fn cover_disappears_when_its_fields_are_removed() {
    let mut h = Harness::new();
    let text = "---\ncover: true\nPage_cover: tunnel\n---\nтекст";
    let note = h.write_note("Книга.md", text);
    h.open(note);
    let toggle = h.app.note_ids.as_ref().unwrap().metadata.id();
    h.act(toggle, crate::ui::note::MetadataToggled);
    let body = h.app.note_ids.as_ref().unwrap().body;
    h.root.focus_on(Some(body.id()));
    let erase = |h: &mut Harness, range: std::ops::Range<usize>| {
        body.edit(&mut h.root, |mut b| crate::ui::editor::TextEditor::select_byte_range(&mut b, range.start, range.end));
        h.key(Key::Named(NamedKey::Backspace), Modifiers::empty());
    };
    let shown = |h: &Harness| h.app.hero_shown.as_ref().map(|key| key[0].clone());
    assert_eq!(shown(&h).as_deref(), Some("tunnel"));
    let end = text.find("tunnel").unwrap() + "tunnel".len();
    erase(&mut h, end..end);
    assert_eq!(shown(&h).as_deref(), Some("tunne"), "обложка не сбросилась после правки узора");
    let cover = text.find("cover: true").unwrap();
    erase(&mut h, cover..cover + "cover: true\n".len());
    assert!(h.app.note_ids.as_ref().unwrap().hero.is_none(), "обложка осталась без cover: true");
    h.type_text("cover: true\n");
    assert!(h.app.note_ids.as_ref().unwrap().hero.is_some());
    let _ = h.root.redraw();
}
