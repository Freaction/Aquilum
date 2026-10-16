use super::moves::{notes_under, relocated};
use super::trash::removed;
use super::NoteMove;
use std::fs;

#[test]
fn a_note_moves_as_itself() {
    let directory = tempfile::tempdir().unwrap();
    let note = directory.path().join("Идея.md");
    fs::write(&note, "текст").unwrap();
    let target = directory.path().join("Проекты").join("Идея.md");

    let relocation = relocated(&notes_under(&note), &note, &target);

    assert_eq!(
        relocation.moves,
        vec![NoteMove {
            from: note.to_string_lossy().into_owned(),
            to: target.to_string_lossy().into_owned(),
        }]
    );
}

#[test]
fn a_text_file_moves_and_disappears_like_a_note() {
    let directory = tempfile::tempdir().unwrap();
    let text = directory.path().join("Черновик.txt");
    fs::write(&text, "текст").unwrap();
    let target = directory.path().join("Архив").join("Черновик.txt");

    let relocation = relocated(&notes_under(&text), &text, &target);
    assert_eq!(relocation.moves.len(), 1);
    assert_eq!(relocation.moves[0].to, target.to_string_lossy());
    assert_eq!(removed(notes_under(&text).iter()).removed, vec![text.to_string_lossy().into_owned()]);
}

#[test]
fn a_folder_moves_as_every_note_inside_it() {
    let directory = tempfile::tempdir().unwrap();
    let folder = directory.path().join("Проекты");
    fs::create_dir_all(folder.join("Глубже")).unwrap();
    fs::write(folder.join("А.md"), "").unwrap();
    fs::write(folder.join("Глубже").join("Б.md"), "").unwrap();
    fs::write(folder.join("картинка.png"), "").unwrap();
    let target = directory.path().join("Архив");

    let mut relocation = relocated(&notes_under(&folder), &folder, &target);
    relocation.moves.sort_by(|left, right| left.from.cmp(&right.from));

    let targets = relocation
        .moves
        .iter()
        .map(|entry| entry.to.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        targets,
        vec![
            target.join("А.md").to_string_lossy().into_owned(),
            target.join("Глубже").join("Б.md").to_string_lossy().into_owned(),
        ],
        "только заметки, вложенность сохраняется"
    );
}

#[test]
fn a_deleted_folder_reports_the_notes_it_held() {
    let directory = tempfile::tempdir().unwrap();
    let folder = directory.path().join("Старое");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("А.md"), "").unwrap();

    let relocation = removed(notes_under(&folder).iter());

    assert_eq!(relocation.removed, vec![folder.join("А.md").to_string_lossy().into_owned()]);
    assert!(relocation.moves.is_empty());
}
