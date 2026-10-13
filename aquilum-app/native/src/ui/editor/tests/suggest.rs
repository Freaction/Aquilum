use std::sync::{Arc, Mutex};

use masonry::core::keyboard::NamedKey;

use super::support::{editor_id, focused, named, text, typing};
use crate::ui::editor::TextEditor;
use crate::ui::editor::resolve::{Book, Fetch, Grid, Media, Request, Resolver};

struct Titles(Mutex<Vec<bool>>);

impl Resolver for Titles {
    fn dataview(&self, _query: &str) -> (Fetch<Grid>, u64) {
        (Fetch::Pending, 0)
    }
    fn media(&self, _source: &str, _wiki: bool) -> (Fetch<Media>, u64) {
        (Fetch::Pending, 0)
    }
    fn book(&self, _target: &str) -> (Fetch<Book>, u64) {
        (Fetch::Pending, 0)
    }
    fn request(&self, request: Request) {
        if let Request::Suggest(view) = request {
            self.0.lock().unwrap().push(view.is_some());
        }
    }
    fn suggest(&self, query: &str) -> Option<Vec<String>> {
        let query = query.to_lowercase();
        Some(["Обратная ссылка", "Проект", "Заметки"].into_iter().filter(|t| t.to_lowercase().starts_with(&query)).map(str::to_owned).collect())
    }
    fn suggest_min(&self) -> Option<usize> {
        Some(1)
    }
}

fn editor(source: &str) -> (masonry::app::RenderRoot, Arc<Titles>) {
    let titles = Arc::new(Titles(Mutex::new(Vec::new())));
    let resolver: Arc<dyn Resolver> = titles.clone();
    (focused(TextEditor::new(source).markdown().with_resolver(Some(resolver)), 600, 400), titles)
}

fn caret_after(root: &mut masonry::app::RenderRoot, source: &str, marker: &str) {
    let at = source.find(marker).expect("метка") + marker.len();
    let id = editor_id(root);
    root.edit_widget(id, |mut w| TextEditor::select_byte_range(&mut w.downcast(), at, at));
    let _ = root.redraw();
}

#[test]
fn choosing_a_title_keeps_the_alias_after_it() {
    let source = "[[обратная ссылк|текст]] хвост";
    let (mut root, _) = editor(source);
    caret_after(&mut root, source, "[[обратная ссылк");
    typing(&mut root, "а");
    named(&mut root, NamedKey::Enter);
    assert_eq!(text(&root), "[[Обратная ссылка|текст]] хвост");
}

#[test]
fn choosing_a_title_keeps_the_heading_anchor_after_it() {
    let source = "[[Замет#Раздел|подпись]]";
    let (mut root, _) = editor(source);
    caret_after(&mut root, source, "[[Замет");
    typing(&mut root, "к");
    named(&mut root, NamedKey::Enter);
    assert_eq!(text(&root), "[[Заметки#Раздел|подпись]]");
}

#[test]
fn choosing_a_title_in_a_plain_link_still_replaces_the_rest_of_the_title() {
    let source = "[[Про отрезок]] хвост";
    let (mut root, _) = editor(source);
    caret_after(&mut root, source, "[[Про");
    typing(&mut root, "е");
    named(&mut root, NamedKey::Enter);
    assert_eq!(text(&root), "[[Проект]] хвост");
}

#[test]
fn nothing_is_suggested_while_the_caret_is_in_the_alias() {
    let source = "[[Заметки|Про]]";
    let (mut root, titles) = editor(source);
    caret_after(&mut root, source, "[[Заметки|Про");
    typing(&mut root, "е");
    assert!(titles.0.lock().unwrap().iter().all(|shown| !shown), "подсказки появились внутри подписи ссылки");
    assert_eq!(text(&root), "[[Заметки|Прое]]");
}
