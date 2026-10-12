use std::path::Path;

use super::Tabs;

fn paths(tabs: &Tabs) -> Vec<Option<&str>> {
    tabs.tabs().iter().map(|t| t.path.as_deref().and_then(Path::to_str)).collect()
}

#[test]
fn open_replaces_active_tab_and_selects_existing() {
    let mut tabs = Tabs::default();
    let first = tabs.active_id();
    tabs.open("a.md".into());
    assert_eq!(tabs.active_id(), first);
    assert_eq!(paths(&tabs), [Some("a.md")]);
    tabs.new_tab();
    tabs.open("b.md".into());
    assert_eq!(paths(&tabs), [Some("a.md"), Some("b.md")]);
    tabs.open("a.md".into());
    assert_eq!(tabs.active_id(), first);
    assert_eq!(tabs.tabs().len(), 2);
}

#[test]
fn close_selects_left_neighbour_and_keeps_one_tab() {
    let mut tabs = Tabs::default();
    tabs.open("a.md".into());
    tabs.new_tab();
    tabs.open("b.md".into());
    tabs.new_tab();
    tabs.open("c.md".into());
    let b = tabs.tabs()[1].id;
    tabs.close(tabs.active_id());
    assert_eq!(tabs.active_id(), b);
    let a = tabs.tabs()[0].id;
    tabs.select(a);
    tabs.close(a);
    assert_eq!(tabs.active_id(), b);
    tabs.close(b);
    assert_eq!(tabs.tabs().len(), 1);
    assert!(tabs.active().path.is_none());
}

#[test]
fn open_new_adds_tab_or_selects_existing() {
    let mut tabs = Tabs::default();
    tabs.open("a.md".into());
    tabs.open_new("b.md".into());
    assert_eq!(paths(&tabs), [Some("a.md"), Some("b.md")]);
    let b = tabs.active_id();
    tabs.open("a.md".into());
    tabs.open_new("b.md".into());
    assert_eq!(tabs.active_id(), b);
    assert_eq!(tabs.tabs().len(), 2);
}

#[test]
fn reorder_moves_tab() {
    let mut tabs = Tabs::default();
    tabs.open("a.md".into());
    tabs.new_tab();
    tabs.open("b.md".into());
    tabs.new_tab();
    tabs.open("c.md".into());
    assert!(tabs.reorder(0, 2));
    assert_eq!(paths(&tabs), [Some("b.md"), Some("c.md"), Some("a.md")]);
    assert!(!tabs.reorder(1, 1));
    assert!(!tabs.reorder(0, 9));
}

#[test]
fn rename_follows_moved_note() {
    let mut tabs = Tabs::default();
    tabs.open("a.md".into());
    tabs.new_tab();
    tabs.open("b.md".into());
    assert!(tabs.rename(Path::new("a.md"), Path::new("c.md")));
    assert_eq!(paths(&tabs), [Some("c.md"), Some("b.md")]);
}
