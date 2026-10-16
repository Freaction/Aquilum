use super::*;

fn follow(h: &mut Harness, raw: &str, new_tab: bool) -> bool {
    let opened = h.app.open_target(&mut h.root, crate::links::Target::Wiki(raw.into()), new_tab);
    h.deliver();
    opened
}

fn notes(h: &Harness) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(h.vault.path())
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".md"))
        .collect();
    names.sort();
    names
}

fn tabs(names: &[&str]) -> Vec<Option<String>> {
    names.iter().map(|name| Some((*name).to_owned())).collect()
}

#[test]
fn heading_link_opens_the_note_at_the_heading_without_creating_files() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("a.md"), "[[Заметка#Раздел]]").unwrap();
    std::fs::write(vault.join("Заметка.md"), "# Первый\n\nтекст\n\n## Раздел\n\nещё").unwrap();
    h.app.open(&mut h.root, vault.join("a.md"));
    assert!(follow(&mut h, "Заметка#Раздел|подпись", true));
    assert_eq!(h.tab_paths(), tabs(&["a", "Заметка"]));
    assert_eq!(caret(&h), "# Первый\n\nтекст\n\n".len());
    assert_eq!(notes(&h), ["a.md", "Заметка.md"]);
}

#[test]
fn link_to_a_missing_note_with_an_anchor_creates_a_clean_file_name() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("a.md"), "текст").unwrap();
    h.app.open(&mut h.root, vault.join("a.md"));
    assert!(follow(&mut h, "Новая#Раздел|подпись", false));
    assert_eq!(notes(&h), ["a.md", "Новая.md"]);
    assert_eq!(h.tab_paths(), tabs(&["Новая"]));
}

#[test]
fn link_to_a_heading_of_the_current_note_stays_in_it() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("b.md"), "# А\n\n## Раздел\n\nтекст").unwrap();
    h.app.open(&mut h.root, vault.join("b.md"));
    let before = notes(&h);
    assert!(follow(&mut h, "#Раздел", false));
    assert_eq!(notes(&h), before);
    assert_eq!(h.tab_paths(), tabs(&["b"]));
    assert_eq!(caret(&h), "# А\n\n".len());
    assert!(follow(&mut h, "#Нет такого", true));
    assert_eq!(notes(&h), before);
    assert_eq!(h.tab_paths(), tabs(&["b"]));
}
