use super::*;
use crate::app::tests::adaptive::resize;

#[test]
fn focus_mode_sits_out_a_narrow_screen_and_comes_back() {
    let mut h = Harness::new();
    h.open(h.vault.path().join("a.md"));
    h.settle();
    h.app.toggle_focus_mode(&mut h.root);
    h.settle();
    let button_hidden = |h: &Harness| h.root.get_widget(h.app.note_ids.as_ref().unwrap().focus).unwrap().ctx().is_stashed();
    assert!(h.app.interface.focus_mode() && !button_hidden(&h));

    resize(&mut h, 360.0);
    assert!(!h.app.interface.focus_mode(), "на узком экране режим фокуса действует");
    assert!(button_hidden(&h), "кнопка фокуса видна на узком экране");
    h.app.toggle_focus_mode(&mut h.root);

    resize(&mut h, 1000.0);
    assert!(h.app.interface.focus_mode(), "режим фокуса не вернулся на широком экране");
    assert!(!button_hidden(&h), "кнопка фокуса не вернулась на широком экране");
}
