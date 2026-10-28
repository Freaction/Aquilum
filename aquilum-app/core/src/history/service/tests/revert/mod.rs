use super::{text_of, Fixture};
use crate::files::content::FileContent;
use crate::files::document::hash_bytes;
use crate::history::milestone::Source;
use crate::history::revert::tests::Edited;
use crate::history::service::{Change, HistoryService, NoteWrite};
use crate::history::store::{versions, VersionFile};
use crate::text::content::collect;
use std::fs;
use std::io;
use std::path::PathBuf;

mod chain;
mod life;

struct Story {
    history: HistoryService,
    note: PathBuf,
    fixture: Fixture,
}

impl Story {
    fn new(name: &str, text: &str) -> Self {
        let fixture = Fixture::new();
        let note = fixture.vault.join(name);
        fs::write(&note, text).unwrap();
        Self { history: fixture.service(), note, fixture }
    }

    fn agent(&self, text: &str) {
        self.save(text, Source::Agent);
    }

    fn save(&self, text: &str, source: Source) {
        self.fixture.save(&self.history, &self.note, text, source);
    }

    fn edit(&self, edits: &[(&str, &str)], source: Source) {
        let before = self.text();
        let mut edited = Edited::new(&before);
        for (old, new) in edits {
            edited.replace(old, new);
        }
        self.record(&before, &edited.text, source, edited.regions);
    }

    fn record(&self, before: &str, text: &str, source: Source, regions: Vec<crate::history::delta::Region>) {
        let roots = || self.fixture.roots();
        let original = self.history.original(&roots, &self.note);
        fs::write(&self.note, text).unwrap();
        let (hash, base) = (hash_bytes(text.as_bytes()), hash_bytes(before.as_bytes()));
        let write = NoteWrite { text: &text, hash: &hash, source, name: None, change: Some(Change { regions, base: &base }), original };
        self.history.saved(&roots, &self.note, write);
    }

    fn versions(&self) -> Vec<VersionFile> {
        versions(&self.fixture.history_of(&self.relative()))
    }

    fn relative(&self) -> String {
        self.note.strip_prefix(&self.fixture.vault).unwrap().to_string_lossy().into_owned()
    }

    fn version_text(&self, id: &str) -> String {
        text_of(&self.fixture.history_of(&self.relative()), id)
    }

    fn text(&self) -> String {
        fs::read_to_string(&self.note).unwrap()
    }

    fn revert(&self, version: &VersionFile) -> io::Result<Vec<String>> {
        let before = self.text();
        let reverted = self.history.revert(&|| self.fixture.roots(), &self.note, &version.id, Box::new(FileContent::open(&self.note)?))?;
        let text = collect(&reverted.text);
        drop(reverted.text);
        if !reverted.regions.is_empty() {
            self.record(&before, &text, Source::Revert(version.at_ms), reverted.regions);
        }
        Ok(reverted.displaced)
    }

    fn revert_nth(&self, index: usize) -> Vec<String> {
        let version = self.versions().remove(index);
        self.revert(&version).unwrap_or_else(|error| panic!("версия {index} не отменилась: {error}"))
    }
}

#[test]
fn reverting_the_latest_version_matches_restoring_the_one_before() {
    let shapes: [&[(&str, &str)]; 6] = [
        &[("два\n", "")],
        &[("Привет, ", "")],
        &[("раз\n", "")],
        &[("три", "три\nчетыре")],
        &[("раз", "РАЗ"), ("мир", "мир!")],
        &[("раз\nдва\nПривет, мир\nтри", "")],
    ];
    for (index, edits) in shapes.iter().enumerate() {
        let story = Story::new("Заметка.md", "раз\nдва\nПривет, мир\nтри");
        story.agent("раз\nдва\nПривет, мир\nтри");
        story.edit(edits, Source::Agent);
        let all = story.versions();
        let before = story.version_text(&all[all.len() - 2].id);
        story.revert_nth(all.len() - 1);
        assert_eq!(story.text(), before, "форма {index}: отмена последней версии равна восстановлению предыдущей");
    }
}

#[test]
fn a_version_from_the_middle_is_undone_and_later_edits_stay() {
    let story = Story::new("Список.md", "раз\nдва\nтри\nчетыре\n");
    story.agent("раз\nдва\nтри\nчетыре\n");
    story.edit(&[("два\n", "")], Source::Agent);
    story.edit(&[("четыре", "ЧЕТЫРЕ")], Source::Agent);
    story.edit(&[("ЧЕТЫРЕ\n", "ЧЕТЫРЕ\nпять\n")], Source::Agent);
    let displaced = story.revert_nth(1);
    assert_eq!(story.text(), "раз\nдва\nтри\nЧЕТЫРЕ\nпять\n", "удалённая в середине истории строка вернулась, поздние правки остались");
    assert!(displaced.is_empty());
}

#[test]
fn a_typing_version_with_several_regions_is_undone_whole() {
    let story = Story::new("Черновик.md", "заголовок\nабзац один\nабзац два\nабзац три\n");
    story.agent("заголовок\nабзац один\nабзац два\nабзац три\n");
    story.edit(&[("заголовок", "Заголовок")], Source::Me);
    story.edit(&[("абзац два\n", "")], Source::Me);
    story.edit(&[("три", "три!")], Source::Me);
    assert_eq!(story.versions().len(), 2, "правки подряд копятся в одну веху");
    story.edit(&[("абзац один", "абзац один и ещё")], Source::Agent);
    story.revert_nth(1);
    assert_eq!(story.text(), "заголовок\nабзац один и ещё\nабзац два\nабзац три\n", "все участки накопленной вехи отменены, правка агента осталась");
}

#[test]
fn a_later_edit_of_the_same_line_goes_aside_and_the_revert_wins() {
    let story = Story::new("Спор.md", "раз\nдва\nтри\n");
    story.agent("раз\nдва\nтри\n");
    story.edit(&[("два", "ДВА")], Source::Agent);
    story.edit(&[("ДВА", "ДВА и ещё")], Source::Agent);
    let displaced = story.revert_nth(1);
    assert_eq!(story.text(), "раз\nдва\nтри\n", "отмена побеждает");
    assert_eq!(displaced, ["ДВА и ещё"], "поздний текст той же строки уходит в копию-конфликт");
}
