use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use aquilum_core::settings::models::AppConfig;
use masonry::app::{RenderRoot, RenderRootOptions, RenderRootSignal, WindowSizePolicy};
use masonry::core::keyboard::{Code, Key, KeyState, Location, Modifiers, NamedKey};
use masonry::core::{
    KeyboardEvent, PointerButton, PointerButtonEvent, PointerEvent, PointerId, PointerInfo, PointerState,
    PointerType, PointerUpdate, TextEvent,
};
use masonry::dpi::{PhysicalPosition, PhysicalSize};
use masonry::kurbo::Size;

use super::{App, Outcome, Popup};
use crate::ui::settings::schema::Section;
use crate::ui::theme;
use crate::workspace::Workspace;

const W: u32 = 1000;
const H: u32 = 600;
const MOUSE: PointerInfo = PointerInfo {
    pointer_id: Some(PointerId::PRIMARY),
    persistent_device_id: None,
    pointer_type: PointerType::Mouse,
};

struct Harness {
    app: App,
    root: RenderRoot,
    signals: Rc<RefCell<Vec<RenderRootSignal>>>,
    vault: tempfile::TempDir,
    _data: tempfile::TempDir,
    _theme: std::sync::MutexGuard<'static, ()>,
}

impl Harness {
    fn new() -> Self {
        Self::with(None, 1.0)
    }

    fn with(real_vault: Option<&std::path::Path>, scale: f64) -> Self {
        let theme = theme::test_lock();
        let vault = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        std::fs::write(vault.path().join("a.md"), "текст").unwrap();
        let mut config = AppConfig::default();
        config.ui.theme = if std::env::var_os("AQUILUM_BENCH_LIGHT").is_some() { "light".into() } else { "dark".into() };
        config.ui.language = "ru".into();
        std::fs::write(data.path().join("settings.json"), serde_json::to_string(&config).unwrap()).unwrap();
        let workspace = Workspace::open(Some(data.path()), Some(real_vault.unwrap_or(vault.path())), || {}).unwrap();
        let mut app = App::new(workspace, String::new());
        let signals: Rc<RefCell<Vec<RenderRootSignal>>> = Rc::default();
        let sink = signals.clone();
        let options = RenderRootOptions {
            default_properties: Arc::new(theme::default_properties()),
            use_system_fonts: true,
            size_policy: WindowSizePolicy::User,
            size: bench_size().map_or(PhysicalSize::new((f64::from(W) * scale) as u32, (f64::from(H) * scale) as u32), |(w, h)| PhysicalSize::new(w, h)),
            scale_factor: scale,
            test_font: None,
        };
        let mut root = RenderRoot::new(app.root_widget(false), move |s| sink.borrow_mut().push(s), options);
        theme::register_fonts(&mut root);
        app.attach_window_controls(&mut root, Size::new(f64::from(W), f64::from(H)));
        let _ = root.redraw();
        Harness { app, root, signals, vault, _data: data, _theme: theme }
    }

    fn deliver(&mut self) -> Outcome {
        let _ = self.root.redraw();
        let mut outcome = Outcome::default();
        loop {
            let actions: Vec<_> = std::mem::take(&mut *self.signals.borrow_mut())
                .into_iter()
                .filter_map(|s| match s {
                    RenderRootSignal::Action(action, id) => Some((action, id)),
                    _ => None,
                })
                .collect();
            if actions.is_empty() {
                return outcome;
            }
            for (action, id) in actions {
                let next = self.app.on_action(&mut self.root, id, action, Size::new(f64::from(W), f64::from(H)));
                outcome.title |= next.title;
                outcome.rescale |= next.rescale;
            }
            let _ = self.root.redraw();
        }
    }

    fn click(&mut self, x: f64, y: f64) -> Outcome {
        self.click_with(x, y, Modifiers::empty())
    }

    fn click_with(&mut self, x: f64, y: f64, modifiers: Modifiers) -> Outcome {
        let mut state =
            PointerState { position: PhysicalPosition::new(x, y), count: 1, scale_factor: 1.0, modifiers, ..PointerState::default() };
        self.root.handle_pointer_event(PointerEvent::Move(PointerUpdate {
            pointer: MOUSE,
            current: state.clone(),
            coalesced: vec![],
            predicted: vec![],
        }));
        state.buttons.insert(PointerButton::Primary);
        let down = PointerButtonEvent { pointer: MOUSE, button: Some(PointerButton::Primary), state: state.clone() };
        self.root.handle_pointer_event(PointerEvent::Down(down));
        state.buttons.remove(PointerButton::Primary);
        let up = PointerButtonEvent { pointer: MOUSE, button: Some(PointerButton::Primary), state };
        self.root.handle_pointer_event(PointerEvent::Up(up));
        self.deliver()
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) -> Outcome {
        for state in [KeyState::Down, KeyState::Up] {
            let event = KeyboardEvent {
                state,
                key: key.clone(),
                code: Code::Unidentified,
                location: Location::Standard,
                modifiers,
                repeat: false,
                is_composing: false,
            };
            self.root.handle_text_event(TextEvent::Keyboard(event));
        }
        self.deliver()
    }

    fn escape(&mut self) -> Outcome {
        for state in [KeyState::Down, KeyState::Up] {
            let event = KeyboardEvent {
                state,
                key: Key::Named(NamedKey::Escape),
                code: Code::Escape,
                location: Location::Standard,
                modifiers: Modifiers::empty(),
                repeat: false,
                is_composing: false,
            };
            self.root.handle_text_event(TextEvent::Keyboard(event));
        }
        self.deliver()
    }

    fn config(&self) -> AppConfig {
        self.app.workspace.core.settings.get_config()
    }

    fn section(&self) -> Option<Section> {
        match &self.app.popup {
            Some(Popup::Settings { dialog, .. }) => Some(dialog.section),
            _ => None,
        }
    }

    fn menu_open(&self) -> bool {
        matches!(&self.app.popup, Some(Popup::Settings { menu: Some(_), .. }))
    }
}

#[test]
fn settings_dialog_applies_changes() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    assert_eq!(h.section(), Some(Section::Ui));

    h.click(807.0, 236.0);
    assert_eq!(h.config().ui.theme, "light");
    assert_eq!(h.section(), Some(Section::Ui));

    let outcome = h.click(893.0, 173.0);
    assert!(outcome.rescale);
    assert_eq!(h.app.interface.scale(), 1.05);

    h.click(115.0, 398.0);
    assert_eq!(h.section(), Some(Section::History));
    h.click(816.0, 173.0);
    assert!(h.menu_open());
    h.click(816.0, 240.0);
    assert!(!h.menu_open());
    assert_eq!(h.config().history.retention_days, 30);

    h.click(115.0, 330.0);
    assert_eq!(h.section(), Some(Section::Analysis));
    assert!(h.config().analysis.enable_bm25f);
    h.click(891.0, 173.0);
    assert!(!h.config().analysis.enable_bm25f);

    h.escape();
    assert_eq!(h.section(), None);
    h.click(242.0, 578.0);
    assert_eq!(h.section(), Some(Section::Analysis));
}

#[test]
fn trash_section_restores_deleted_note() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    aquilum_core::files::gate::trash(&h.app.workspace.core, &vault, &vault.join("a.md")).unwrap();
    assert!(!vault.join("a.md").exists());
    h.click(242.0, 578.0);
    h.click(115.0, 432.0);
    assert_eq!(h.section(), Some(Section::Trash));
    let restore = match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog.bindings.iter().find_map(|(id, b)| match b {
            crate::ui::settings::Binding::Button(crate::ui::settings::schema::Action::Restore(_)) => Some(*id),
            _ => None,
        }),
        _ => None,
    }
    .expect("в корзине есть заметка");
    h.app.on_action(&mut h.root, restore, masonry::core::ErasedAction::from(Box::new(crate::ui::widgets::Pressed)), Size::new(1000.0, 600.0));
    assert!(vault.join("a.md").exists());
}

#[test]
fn primary_color_field_updates_settings() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    assert_eq!(h.section(), Some(Section::Ui));
    let field = match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog.bindings.iter().find_map(|(id, b)| match b {
            crate::ui::settings::Binding::Text { value, .. } if value.starts_with('#') => Some(*id),
            _ => None,
        }),
        _ => None,
    }
    .expect("поле основного цвета");
    let change = |text: &str| masonry::core::ErasedAction::from(Box::new(masonry::widgets::TextAction::Changed(text.into())));
    h.app.on_action(&mut h.root, field, change("#E05A"), Size::new(1000.0, 600.0));
    assert_eq!(h.config().ui.primary_color, AppConfig::default().ui.primary_color);
    h.app.on_action(&mut h.root, field, change("#E05A2B"), Size::new(1000.0, 600.0));
    assert_eq!(h.config().ui.primary_color, "#e05a2b");
    assert_eq!(h.section(), Some(Section::Ui));
    assert_eq!(theme::current().bg_accent.to_rgba8().to_u8_array(), [0xe0, 0x5a, 0x2b, 0xff]);
}

#[test]
fn tabs_open_new_select_and_close() {
    let mut h = Harness::new();
    let note = h.vault.path().join("a.md");
    h.app.open(&mut h.root, note.clone());
    assert_eq!(h.app.tabs.tabs().len(), 1);
    assert!(h.app.note.is_some());

    let plus = 1000.0 - 320.0 - 44.0 - 6.0 - 16.0;
    let outcome = h.click(plus, 20.0);
    assert!(outcome.title);
    assert_eq!(h.app.tabs.tabs().len(), 2);
    assert!(h.app.tabs.active().path.is_none());
    assert!(h.app.note.is_none());

    let first = h.app.tabs.tabs()[0].id;
    let select = masonry::core::ErasedAction::from(Box::new(crate::ui::titlebar::TitlebarAction::Select(first)));
    let strip = h.app.chrome.as_ref().unwrap().tab_strip.id();
    h.app.on_action(&mut h.root, strip, select, Size::new(1000.0, 600.0));
    assert_eq!(h.app.tabs.active_id(), first);
    assert!(h.app.note.as_ref().is_some_and(|n| n.is(&note)));

    let second = h.app.tabs.tabs()[1].id;
    let close = masonry::core::ErasedAction::from(Box::new(crate::ui::titlebar::TitlebarAction::Close(second)));
    h.app.on_action(&mut h.root, strip, close, Size::new(1000.0, 600.0));
    assert_eq!(h.app.tabs.tabs().len(), 1);

    let outcome = h.app.on_action(
        &mut h.root,
        strip,
        masonry::core::ErasedAction::from(Box::new(crate::ui::titlebar::TitlebarAction::CloseWindow)),
        Size::new(1000.0, 600.0),
    );
    assert_eq!(outcome.window, Some(super::WindowCommand::Close));
}

#[test]
fn primary_color_typed_into_field() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    let field = match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog.bindings.iter().find_map(|(id, b)| match b {
            crate::ui::settings::Binding::Text { value, .. } if value.starts_with('#') => Some(*id),
            _ => None,
        }),
        _ => None,
    }
    .expect("поле основного цвета");
    h.root.focus_on(Some(field));
    h.key(Key::Character("a".into()), Modifiers::CONTROL);
    for ch in "#e05a2b".chars() {
        h.key(Key::Character(ch.to_string().into()), Modifiers::empty());
    }
    assert_eq!(h.config().ui.primary_color, "#e05a2b");
    assert_eq!(theme::current().bg_accent.to_rgba8().to_u8_array(), [0xe0, 0x5a, 0x2b, 0xff]);
}

impl Harness {
    fn drag(&mut self, from: (f64, f64), to: (f64, f64)) -> Outcome {
        let state = |x: f64, y: f64, pressed: bool| {
            let mut state = PointerState { position: PhysicalPosition::new(x, y), count: 1, scale_factor: 1.0, ..PointerState::default() };
            if pressed {
                state.buttons.insert(PointerButton::Primary);
            }
            state
        };
        let moved = |s: PointerState| PointerEvent::Move(PointerUpdate { pointer: MOUSE, current: s, coalesced: vec![], predicted: vec![] });
        self.root.handle_pointer_event(moved(state(from.0, from.1, false)));
        self.root.handle_pointer_event(PointerEvent::Down(PointerButtonEvent {
            pointer: MOUSE,
            button: Some(PointerButton::Primary),
            state: state(from.0, from.1, true),
        }));
        for step in 1..=10 {
            let k = f64::from(step) / 10.0;
            self.root.handle_pointer_event(moved(state(from.0 + (to.0 - from.0) * k, from.1 + (to.1 - from.1) * k, true)));
        }
        self.root.handle_pointer_event(PointerEvent::Up(PointerButtonEvent {
            pointer: MOUSE,
            button: Some(PointerButton::Primary),
            state: state(to.0, to.1, false),
        }));
        self.deliver()
    }

    fn tab_paths(&self) -> Vec<Option<String>> {
        self.app
            .tabs
            .tabs()
            .iter()
            .map(|t| t.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()))
            .collect()
    }
}

#[test]
fn ctrl_shift_click_opens_note_in_new_tab_and_tabs_reorder_by_drag() {
    let mut h = Harness::new();
    std::fs::write(h.vault.path().join("b.md"), "").unwrap();
    let vault = h.vault.path().to_path_buf();
    h.app.tree.paths_changed(&mut h.root, &[vault.join("b.md")]);
    h.click(120.0, 65.0);
    assert_eq!(h.tab_paths(), [Some("a".to_owned())]);
    h.click_with(120.0, 99.0, Modifiers::CONTROL | Modifiers::SHIFT);
    assert_eq!(h.tab_paths(), [Some("a".to_owned()), Some("b".to_owned())]);
    assert_eq!(h.app.tabs.active().path.as_deref(), Some(vault.join("b.md").as_path()));

    let first = 266.0 + 40.0;
    h.drag((first, 20.0), (first + 200.0, 20.0));
    assert_eq!(h.tab_paths(), [Some("b".to_owned()), Some("a".to_owned())]);
}

#[test]
fn tab_menu_renames_note_in_tab() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    h.app.open(&mut h.root, vault.join("a.md"));
    let id = h.app.tabs.active_id();
    let strip = h.app.chrome.as_ref().unwrap().tab_strip.id();
    let menu = masonry::core::ErasedAction::from(Box::new(crate::ui::titlebar::TitlebarAction::Menu(id, masonry::kurbo::Point::new(300.0, 20.0))));
    h.app.on_action(&mut h.root, strip, menu, Size::new(1000.0, 600.0));
    let layer = match &h.app.popup {
        Some(Popup::FileMenu { layer, tab: Some(_), .. }) => *layer,
        _ => panic!("меню вкладки"),
    };
    let rename = masonry::core::ErasedAction::from(Box::new(crate::ui::menu::MenuChoice::Item(0)));
    h.app.on_action(&mut h.root, layer, rename, Size::new(1000.0, 600.0));
    let field = h.app.rename_field.expect("поле переименования во вкладке");
    assert_eq!(h.root.focused_widget(), Some(field.id()));
    let entered = masonry::core::ErasedAction::from(Box::new(masonry::widgets::TextAction::Entered("Новое имя".into())));
    h.app.on_action(&mut h.root, field.id(), entered, Size::new(1000.0, 600.0));
    assert!(h.app.rename_field.is_none());
    let renamed = vault.join("Новое имя.md");
    for _ in 0..250 {
        if renamed.exists() && !vault.join("a.md").exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(renamed.exists());
    assert!(!vault.join("a.md").exists());
}

impl Harness {
    fn wait_listing(&mut self) -> crate::links::Listing {
        for _ in 0..250 {
            self.app.on_core_events(&mut self.root);
            if let Some(listing) = self.app.listing.as_ref().filter(|l| l.generation == self.app.panel_generation) {
                return listing.clone();
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("панель не получила ответ");
    }
}

#[test]
fn backlinks_panel_lists_and_opens_mentions() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("a.md"), "Ссылка на [[b]]").unwrap();
    std::fs::write(vault.join("b.md"), "Текст").unwrap();
    h.app.tree.paths_changed(&mut h.root, &[vault.join("b.md")]);
    h.app.open(&mut h.root, vault.join("b.md"));
    let mut listing = h.wait_listing();
    for _ in 0..50 {
        if !listing.items.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(40));
        h.app.request_panel(&mut h.root);
        listing = h.wait_listing();
    }
    assert_eq!(listing.items.len(), 1);
    assert_eq!(listing.items[0].title, "a");

    let item = h.app.panel_body.as_ref().unwrap().items[0].0;
    let open = |new_tab| masonry::core::ErasedAction::from(Box::new(crate::ui::widgets::ItemAction::Open { new_tab }));
    h.app.on_action(&mut h.root, item, open(true), Size::new(1000.0, 600.0));
    assert_eq!(h.tab_paths(), [Some("b".to_owned()), Some("a".to_owned())]);

    let outgoing = h.app.chrome.as_ref().unwrap().panel_parts.modes[1].0.id();
    let pressed = || masonry::core::ErasedAction::from(Box::new(crate::ui::widgets::Pressed));
    h.app.on_action(&mut h.root, outgoing, pressed(), Size::new(1000.0, 600.0));
    let listing = h.wait_listing();
    assert_eq!(listing.items.len(), 1);
    assert!(matches!(&listing.items[0].target, crate::links::Target::Note(p) if p.ends_with("b.md")));

    let toggle = h.app.chrome.as_ref().unwrap().panel_toggle;
    h.app.on_action(&mut h.root, toggle, pressed(), Size::new(1000.0, 600.0));
    assert!(!h.app.interface.right_open());
    assert!(h.app.listing.is_none());
}

#[test]
fn history_panel_opens_version_and_reverts_it() {
    use aquilum_core::files::document::read_file_snapshot_impl;
    use aquilum_core::files::gate;
    use aquilum_core::history::Source;

    let mut h = Harness::new();
    let note = h.vault.path().join("a.md");
    h.app.open(&mut h.root, note.clone());
    for (text, source) in [("один\nдва\n", Source::Me), ("один\nдва\nтри\n", Source::Agent)] {
        let hash = read_file_snapshot_impl(&note).unwrap().hash;
        gate::write(&h.app.workspace.core, &note, text, Some(&hash), source, None).unwrap();
    }
    let history_button = h.app.chrome.as_ref().unwrap().panel_parts.modes.last().unwrap().0.id();
    let pressed = || masonry::core::ErasedAction::from(Box::new(crate::ui::widgets::Pressed));
    h.app.on_action(&mut h.root, history_button, pressed(), Size::new(1000.0, 600.0));
    let rows = h.app.history_body.as_ref().expect("список версий").rows.clone();
    assert!(rows.len() >= 2, "строк версий: {}", rows.len());

    let versions = h.app.history.as_ref().unwrap().versions.clone();
    let agent = versions.iter().position(|v| v.source == Source::Agent).expect("версия агента");
    let row = rows.iter().find(|(_, index)| *index == agent).unwrap().0;
    let open = masonry::core::ErasedAction::from(Box::new(crate::ui::widgets::ItemAction::Open { new_tab: false }));
    h.app.on_action(&mut h.root, row, open, Size::new(1000.0, 600.0));
    let view = h.app.version_view.as_ref().expect("версия открыта");
    let revert = view.revert.expect("у версии есть предыдущая");

    h.app.on_action(&mut h.root, revert, pressed(), Size::new(1000.0, 600.0));
    assert_eq!(std::fs::read_to_string(&note).unwrap(), "один\nдва\n");
    assert!(h.app.version_view.is_none());
    assert_eq!(h.app.editor_text, "один\nдва\n");
}

impl Harness {
    fn type_search(&mut self, text: &str) {
        let field = match &self.app.popup {
            Some(Popup::Search(state)) => state.field_id(),
            _ => panic!("поиск не открыт"),
        };
        let changed = masonry::core::ErasedAction::from(Box::new(masonry::widgets::TextAction::Changed(text.into())));
        self.app.on_action(&mut self.root, field, changed, Size::new(1000.0, 600.0));
    }

    fn wait_results(&mut self, at_least: usize) -> usize {
        for _ in 0..250 {
            self.app.on_core_events(&mut self.root);
            if let Some(Popup::Search(state)) = &self.app.popup
                && state.result_count() >= at_least
            {
                return state.result_count();
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("поиск не дал результатов");
    }
}

#[test]
fn search_finds_opens_and_creates_notes() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("Кофе.md"), "Заметка про обжарку кофе").unwrap();
    std::fs::write(vault.join("Чай.md"), "Кофе тоже упомянут").unwrap();
    h.app.start_index();
    for _ in 0..250 {
        let status = h.app.workspace.core.search.status(Some(&vault.to_string_lossy()));
        if status.state == aquilum_core::search::models::SearchIndexState::Ready && !status.updating && status.indexed_documents >= 3 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    h.app.open_search(&mut h.root);
    h.type_search("кофе");
    assert_eq!(h.wait_results(2), 2);
    h.app.search_key(&mut h.root, &Key::Named(NamedKey::ArrowDown), Modifiers::empty());
    let second = match &h.app.popup {
        Some(Popup::Search(state)) => state.selected_path(),
        _ => unreachable!(),
    };
    h.app.search_key(&mut h.root, &Key::Named(NamedKey::Enter), Modifiers::empty());
    assert!(h.app.popup.is_none());
    assert_eq!(h.app.tabs.active().path.as_ref(), Some(&second));

    h.app.open_search(&mut h.root);
    h.type_search("Новая идея");
    h.app.search_key(&mut h.root, &Key::Named(NamedKey::Enter), Modifiers::SHIFT);
    assert!(vault.join("Новая идея.md").exists());
    assert_eq!(h.app.tabs.active().path.as_ref(), Some(&vault.join("Новая идея.md")));
}

impl Harness {
    fn workspace_rows(&self) -> Vec<(masonry::core::WidgetId, std::path::PathBuf)> {
        match &self.app.popup {
            Some(Popup::Workspaces(state)) => state.dialog().rows.iter().map(|(id, path, _)| (*id, path.clone())).collect(),
            _ => Vec::new(),
        }
    }

    fn act(&mut self, id: masonry::core::WidgetId, action: impl std::any::Any + std::fmt::Debug + Send) -> Outcome {
        let outcome = self.app.on_action(&mut self.root, id, masonry::core::ErasedAction::from(Box::new(action)), Size::new(1000.0, 600.0));
        let _ = self.root.redraw();
        outcome
    }
}

#[test]
fn workspace_dialog_switches_and_forgets_vaults() {
    use crate::ui::workspaces::{RowAction, same_path};
    let mut h = Harness::new();
    let first = h.vault.path().to_path_buf();
    h.app.open(&mut h.root, first.join("a.md"));
    let second = tempfile::tempdir().unwrap();
    std::fs::write(second.path().join("b.md"), "второй").unwrap();
    h.app.workspace.core.ui_state.resolve_workspace(&second.path().to_string_lossy(), 1).unwrap();

    let button = h.app.chrome.as_ref().unwrap().workspaces;
    h.act(button, crate::ui::widgets::Pressed);
    let rows = h.workspace_rows();
    assert_eq!(rows.len(), 2);
    let target = rows.iter().find(|(_, p)| same_path(p, second.path())).unwrap().0;
    let outcome = h.act(target, RowAction::Open);
    assert!(outcome.title);
    assert!(h.app.popup.is_none());
    assert!(same_path(h.app.tree.root().unwrap(), second.path()));
    assert!(h.app.note.is_none());
    assert!(h.app.tabs.active().path.is_none());

    h.act(h.app.chrome.as_ref().unwrap().workspaces, crate::ui::widgets::Pressed);
    let back = h.workspace_rows().into_iter().find(|(_, p)| same_path(p, &first)).unwrap().0;
    h.act(back, RowAction::Open);
    assert!(same_path(h.app.tree.root().unwrap(), &first));
    assert!(h.app.note.as_ref().is_some_and(|n| n.is(&first.join("a.md"))));

    h.act(h.app.chrome.as_ref().unwrap().workspaces, crate::ui::widgets::Pressed);
    let other = h.workspace_rows().into_iter().find(|(_, p)| same_path(p, second.path())).unwrap().0;
    h.act(other, RowAction::Forget);
    let confirm = match &h.app.popup {
        Some(Popup::Workspaces(state)) => state.forget_layer().unwrap(),
        _ => panic!("нет окна баз"),
    };
    h.act(confirm, crate::ui::dialog::DialogAnswer::Confirm);
    assert_eq!(h.workspace_rows().len(), 1);
    h.escape();
    assert!(h.app.popup.is_none());

    h.act(h.app.chrome.as_ref().unwrap().workspaces, crate::ui::widgets::Pressed);
    h.escape();
    h.app.switch_workspace(&mut h.root, second.path().join(".").join("..").join(second.path().file_name().unwrap()));
    let reopened = Workspace::open(Some(h._data.path()), None, || {}).unwrap();
    assert!(same_path(reopened.root.as_deref().unwrap(), second.path()));
    let known = h.app.workspace.core.ui_state.list_workspaces().unwrap();
    assert_eq!(known.iter().filter(|w| same_path(std::path::Path::new(&w.path), second.path())).count(), 1);
}

impl Harness {
    fn body_text(&self) -> String {
        let body = self.app.note_ids.as_ref().unwrap().body;
        body.get(&self.root).unwrap().inner().text().to_string()
    }
}

#[test]
fn note_page_hides_frontmatter_renames_and_navigates() {
    use masonry::widgets::TextAction;
    let mut h = Harness::new();
    let note = h.vault.path().join("b.md");
    std::fs::write(&note, "---\ntype: book\n---\nтекст").unwrap();
    h.app.open(&mut h.root, h.vault.path().join("a.md"));
    h.app.open(&mut h.root, note.clone());
    assert_eq!(h.body_text(), "текст");

    let body = h.app.note_ids.as_ref().unwrap().body.id();
    h.act(body, TextAction::Changed("новый текст".into()));
    assert_eq!(h.app.editor_text, "---\ntype: book\n---\nновый текст");

    let toggle = h.app.note_ids.as_ref().unwrap().metadata.id();
    h.act(toggle, crate::ui::note::MetadataToggled);
    assert_eq!(h.body_text(), "---\ntype: book\n---\nновый текст");
    assert!(h.app.interface.metadata_expanded(&note));
    h.act(toggle, crate::ui::note::MetadataToggled);
    assert_eq!(h.body_text(), "новый текст");

    let ids = h.app.note_ids.as_ref().unwrap();
    let (title, back) = (ids.title.id(), ids.back);
    h.act(title, TextAction::Entered("  ".into()));
    assert_eq!(h.app.note_ids.as_ref().unwrap().title.get(&h.root).unwrap().inner().text().to_string(), "b");
    h.act(title, TextAction::Entered("Книга".into()));
    let renamed = h.vault.path().join("Книга.md");
    for _ in 0..1500 {
        h.app.on_core_events(&mut h.root);
        if h.app.note.as_ref().is_some_and(|n| n.is(&renamed)) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(h.app.note.as_ref().is_some_and(|n| n.is(&renamed)));
    assert!(h.app.interface.metadata_expanded(&renamed) == h.app.interface.metadata_expanded(&note));

    let outcome = h.act(back, crate::ui::widgets::Pressed);
    assert!(outcome.title);
    assert!(h.app.note.as_ref().is_some_and(|n| n.is(&h.vault.path().join("a.md"))));
    assert!(h.app.navigation.can_forward());

    h.app.toggle_focus_mode(&mut h.root);
    assert!(h.app.interface.focus_mode());
    assert!(!h.app.panel_visible());
}

#[test]
fn home_button_appears_only_for_resolved_home_page() {
    let mut h = Harness::new();
    assert!(h.app.chrome.as_ref().unwrap().home_button.is_none());
    let root = h.vault.path().to_string_lossy().into_owned();
    h.app.workspace.core.ui_state.set_home_page(&root, "a").unwrap();
    h.app.refresh_home();
    for _ in 0..250 {
        h.app.on_core_events(&mut h.root);
        if h.app.home.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(h.app.home.as_ref().is_some_and(|p| p.ends_with("a.md")));
    let button = h.app.chrome.as_ref().unwrap().home_button.expect("кнопка главной");
    h.act(button, crate::ui::widgets::Pressed);
    assert!(h.app.note.as_ref().is_some_and(|n| n.is(&h.vault.path().join("a.md"))));
}

#[test]
fn cover_actions_rewrite_frontmatter() {
    use crate::ui::widgets::Pressed;
    let mut h = Harness::new();
    let note = h.vault.path().join("Книга.md");
    std::fs::write(&note, "---\ncover: true\ntype: book\nPage_cover: pattern:polka\nBook_cover: Files/x.png\n---\nтекст").unwrap();
    h.app.open(&mut h.root, note.clone());
    let hero = |h: &Harness| h.app.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()).map(|(_, ids)| (ids.random, ids.remove, ids.thumbnail.map(|t| t.id())));
    let (random, _, _) = hero(&h).expect("обложка и книга");
    h.act(random.unwrap(), Pressed);
    let fields = crate::frontmatter::parse(&h.app.editor_text).unwrap();
    let cover = fields.text(crate::frontmatter::PAGE_COVER).unwrap().to_owned();
    assert!(cover.starts_with("pattern:") && cover != "pattern:polka");
    assert_eq!(std::fs::read_to_string(&note).unwrap(), h.app.editor_text);

    let (_, remove, _) = hero(&h).unwrap();
    h.act(remove.unwrap(), Pressed);
    assert!(!h.app.editor_text.contains("cover: true") && !h.app.editor_text.contains("Page_cover"));
    let (_, _, thumbnail) = hero(&h).unwrap();
    h.act(thumbnail.unwrap(), crate::ui::book::ThumbAction::Remove);
    assert_eq!(h.app.editor_text, "---\ntype: book\n---\nтекст");
    assert!(hero(&h).is_some_and(|(random, _, _)| random.is_none()));
}

#[test]
#[ignore]
fn real_vault_book_notes_open() {
    let Some(vault) = std::env::var_os("AQUILUM_TEST_VAULT").map(std::path::PathBuf::from) else { return };
    let mut notes = Vec::new();
    let mut stack = vec![vault.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "md")
                && let Ok(text) = std::fs::read_to_string(&path)
                && crate::frontmatter::parse(&text).is_some_and(|f| f.has_page_cover() || f.is_book())
            {
                notes.push(path);
            }
        }
    }
    eprintln!("заметок с обложкой или книгой: {}", notes.len());
    for scale in [1.0, 1.25, 1.5] {
        let mut h = Harness::with(Some(&vault), scale);
        let mut renderer = crate::render::Renderer::new(4);
        let (w, hgt) = ((f64::from(W) * scale) as u16, (f64::from(H) * scale) as u16);
        let mut frame = |h: &mut Harness| {
            let (layers, _) = h.root.redraw();
            if let Err(error) = renderer.render(&layers, w, hgt, scale, theme::background()) {
                panic!("кадр не нарисован: {error}");
            }
        };
        for note in &notes {
            eprintln!("{scale} {}", note.display());
            h.app.open(&mut h.root, note.clone());
            frame(&mut h);
            h.pointer_move(300.0 * scale, 150.0 * scale);
            frame(&mut h);
            for _ in 0..30 {
                h.app.on_core_events(&mut h.root);
                frame(&mut h);
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    }
}

impl Harness {
    fn pointer_move(&mut self, x: f64, y: f64) {
        let state = PointerState { position: PhysicalPosition::new(x, y), count: 1, scale_factor: 1.0, ..PointerState::default() };
        self.root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: MOUSE, current: state, coalesced: vec![], predicted: vec![] }));
    }
}

#[test]
fn book_page_renders_with_multithreaded_rasterizer() {
    let mut h = Harness::new();
    let note = h.vault.path().join("Книга.md");
    std::fs::write(&note, "---\ncover: true\ntype: book\nPage_cover: pattern:polka\n---\nтекст").unwrap();
    h.app.open(&mut h.root, note);
    let mut renderer = crate::render::Renderer::new(2);
    for _ in 0..3 {
        h.app.on_core_events(&mut h.root);
        let (layers, _) = h.root.redraw();
        renderer.render(&layers, W as u16, H as u16, 1.0, theme::background()).expect("кадр книжной страницы");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
fn editing_cover_fields_in_metadata_keeps_rendering() {
    use masonry::widgets::TextAction;
    let mut h = Harness::new();
    let note = h.vault.path().join("Книга.md");
    let text = "---\ncover: true\ntype: book\nPage_cover: pattern:polka\nBook_cover: Files/x.png\n---\nтекст";
    std::fs::write(&note, text).unwrap();
    h.app.open(&mut h.root, note);
    let toggle = h.app.note_ids.as_ref().unwrap().metadata.id();
    h.act(toggle, crate::ui::note::MetadataToggled);
    let mut renderer = crate::render::Renderer::new(2);
    let mut current = text.to_owned();
    for field in ["Page_cover: pattern:polka\n", "Book_cover: Files/x.png\n", "cover: true\n", "type: book\n"] {
        let start = current.find(field).unwrap();
        for _ in 0..field.chars().count() {
            let ch = current[start..].chars().next().unwrap();
            current.replace_range(start..start + ch.len_utf8(), "");
            let body = h.app.note_ids.as_ref().unwrap().body.id();
            h.act(body, TextAction::Changed(current.clone()));
            h.app.tick(std::time::Instant::now() + std::time::Duration::from_secs(3600));
            for _ in 0..3 {
                std::thread::sleep(std::time::Duration::from_millis(15));
                h.app.on_core_events(&mut h.root);
                let (layers, _) = h.root.redraw();
                renderer.render(&layers, W as u16, H as u16, 1.0, theme::background()).expect("кадр");
            }
        }
    }
    assert_eq!(h.app.editor_text, "---\n---\nтекст");
}

#[test]
#[ignore]
fn scroll_frame_times() {
    use masonry::core::{PointerScrollEvent, ScrollDelta};
    let scale: f64 = std::env::var("AQUILUM_BENCH_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(1.25);
    for (name, text) in [
        ("книга", "---
cover: true
type: book
Page_cover: pattern:ruby
---
"),
        ("обложка", "---
cover: true
Page_cover: pattern:ruby
---
"),
        ("простая", ""),
    ] {
        let mut h = Harness::with(None, scale);
        let note = h.vault.path().join(format!("{name}.md"));
        let body: String = (0..200).map(|i| format!("Строка текста номер {i} для прокрутки страницы.
")).collect();
        std::fs::write(&note, format!("{text}{body}")).unwrap();
        h.app.open(&mut h.root, note);
        let mut renderer = crate::render::Renderer::new(4);
        let (w, hh) = ((f64::from(W) * scale) as u16, (f64::from(H) * scale) as u16);
        let warm = std::time::Instant::now();
        for _ in 0..1000 {
            let finished = crate::cover::take_finished();
            if finished {
                if let Some(cover) = h.app.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()).and_then(|(_, ids)| ids.cover) {
                    cover.edit(&mut h.root, |mut c| crate::ui::book::PageCover::refresh(&mut c));
                }
            }
            let (layers, _) = h.root.redraw();
            renderer.render(&layers, w, hh, scale, theme::background()).unwrap();
            if finished || text.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        eprintln!("{name}: узор готов через {:?}", warm.elapsed());
        let mut total = std::time::Duration::ZERO;
        let mut worst = std::time::Duration::ZERO;
        let frames = 60;
        for i in 0..frames {
            let state = PointerState { position: PhysicalPosition::new(600.0 * scale, 400.0 * scale), count: 1, scale_factor: scale, ..PointerState::default() };
            let dy = if i < frames / 2 { -12.0 } else { 12.0 };
            h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, dy)), state }));
            let started = std::time::Instant::now();
            let (layers, _) = h.root.redraw();
            renderer.draw(&layers, w, hh, scale, theme::background(), i == 0).unwrap();
            let spent = started.elapsed();
            total += spent;
            worst = worst.max(spent);
        }
        eprintln!("{name}: средний кадр {:?}, худший {:?}", total / frames, worst);
    }
}

#[test]
fn cover_disappears_when_its_fields_are_removed() {
    use masonry::widgets::TextAction;
    let mut h = Harness::new();
    let note = h.vault.path().join("Книга.md");
    let text = "---\ncover: true\nPage_cover: pattern:polka\n---\nтекст";
    std::fs::write(&note, text).unwrap();
    h.app.open(&mut h.root, note);
    let toggle = h.app.note_ids.as_ref().unwrap().metadata.id();
    h.act(toggle, crate::ui::note::MetadataToggled);
    assert!(h.app.note_ids.as_ref().unwrap().hero.is_some());
    let body = h.app.note_ids.as_ref().unwrap().body.id();
    h.act(body, TextAction::Changed("---\nPage_cover: pattern:polka\n---\nтекст".into()));
    assert!(h.app.note_ids.as_ref().unwrap().hero.is_none());
    h.act(body, TextAction::Changed("---\ncover: true\n---\nтекст".into()));
    assert!(h.app.note_ids.as_ref().unwrap().hero.is_some());
    let _ = h.root.redraw();
}

fn bench_size() -> Option<(u32, u32)> {
    let size = std::env::var("AQUILUM_BENCH_SIZE").ok()?;
    let (w, h) = size.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

#[test]
#[ignore]
fn frame_parts() {
    use masonry::core::{PointerScrollEvent, ScrollDelta};
    let (w, hh) = bench_size().unwrap_or((W, H));
    let files: usize = std::env::var("AQUILUM_BENCH_FILES").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let vault = tempfile::tempdir().unwrap();
    for i in 0..files.max(1) {
        std::fs::write(vault.path().join(format!("Заметка {i:04}.md")), "текст").unwrap();
    }
    let scale: f64 = std::env::var("AQUILUM_BENCH_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let mut h = Harness::with(Some(vault.path()), scale);
    h.app.window_resized(&mut h.root, Size::new(f64::from(w) / scale, f64::from(hh) / scale));
    let note = vault.path().join("длинная.md");
    let body: String = (0..400).map(|i| format!("Абзац {i}: плотный текст заметки, как в настоящей базе знаний, с длинными предложениями, которые переносятся на несколько строк и заполняют всю ширину колонки редактора от края до края. ")).collect();
    let head = if std::env::var_os("AQUILUM_BENCH_HERO").is_some() { "---
cover: true
type: book
Page_cover: pattern:ruby
---
" } else { "" };
    std::fs::write(&note, format!("{head}{body}")).unwrap();
    h.app.open(&mut h.root, note);
    for _ in 0..500 {
        if crate::cover::take_finished() {
            if let Some(cover) = h.app.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()).and_then(|(_, ids)| ids.cover) {
                cover.edit(&mut h.root, |mut c| crate::ui::book::PageCover::refresh(&mut c));
            }
            break;
        }
        if head.is_empty() {
            break;
        }
        let _ = h.root.redraw();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let threads: u16 = std::env::var("AQUILUM_BENCH_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(std::thread::available_parallelism().map_or(4, |n| n.get() as u16 - 1));
    let mut renderer = crate::render::Renderer::new(threads);
    let (mut scene, mut raster) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
    let frames = 60u32;
    for i in 0..frames {
        let state = PointerState { position: PhysicalPosition::new(f64::from(w) / 2.0, f64::from(hh) / 2.0), count: 1, scale_factor: scale, ..PointerState::default() };
        let dy = if i < frames / 2 { -7.0 } else { 7.0 };
        h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, dy)), state }));
        let t = std::time::Instant::now();
        let (layers, _) = h.root.redraw();
        let t1 = std::time::Instant::now();
        renderer.draw(&layers, w as u16, hh as u16, scale, theme::background(), i == 0).unwrap();
        let t2 = std::time::Instant::now();
        scene += t1 - t;
        raster += t2 - t1;
    }
    eprintln!("{w}x{hh}: сцена {:?}, растр {:?}, всего {:?}", scene / frames, raster / frames, (scene + raster) / frames);
}


#[test]
fn long_tree_builds_only_rows_near_the_view() {
    let vault = tempfile::tempdir().unwrap();
    for i in 0..1000 {
        std::fs::write(vault.path().join(format!("Заметка {i:04}.md")), "текст").unwrap();
    }
    let mut h = Harness::with(Some(vault.path()), 1.0);
    assert!(h.app.tree.built_rows().len() <= 160);
    h.app.open(&mut h.root, vault.path().join("Заметка 0900.md"));
    assert!(h.app.tree.built_rows().contains(&900));
    let scroll = h.app.tree.scroll_id().unwrap();
    h.act(scroll, crate::ui::scroll::Scrolled(0.0));
    assert_eq!(h.app.tree.built_rows().start, 0);
    let _ = h.root.redraw();
}

#[test]
#[ignore]
fn core_events_cost() {
    let Some(source) = std::env::var_os("AQUILUM_TEST_VAULT").map(std::path::PathBuf::from) else { return };
    let mut h = Harness::with(Some(&source), 1.75);
    let started = std::time::Instant::now();
    let mut worst = (std::time::Duration::ZERO, 0.0);
    let mut total = std::time::Duration::ZERO;
    while started.elapsed() < std::time::Duration::from_secs(8) {
        let t = std::time::Instant::now();
        h.app.on_core_events(&mut h.root);
        let (layers, _) = h.root.redraw();
        drop(layers);
        let spent = t.elapsed();
        total += spent;
        if spent > worst.0 {
            worst = (spent, started.elapsed().as_secs_f64());
        }
        if spent > std::time::Duration::from_millis(30) {
            eprintln!("{:.2} с: обработка событий ядра {:?}", started.elapsed().as_secs_f64(), spent);
        }
        std::thread::sleep(std::time::Duration::from_millis(8));
    }
    eprintln!("всего на главном потоке {:?}, худшее {:?} на {:.2} с", total, worst.0, worst.1);
}


#[test]
fn partial_frames_match_full_frames() {
    use masonry::core::{PointerScrollEvent, ScrollDelta};
    let mut h = Harness::new();
    let note = h.vault.path().join("Длинная.md");
    let body: String = (0..120).map(|i| format!("Абзац {i}: текст заметки, который переносится на несколько строк в колонке редактора.\n\n")).collect();
    std::fs::write(&note, format!("---\ncover: true\ntype: book\nPage_cover: pattern:polka\n---\n{body}")).unwrap();
    h.app.open(&mut h.root, note);
    for _ in 0..500 {
        if crate::cover::take_finished() {
            if let Some(cover) = h.app.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()).and_then(|(_, ids)| ids.cover) {
                cover.edit(&mut h.root, |mut c| crate::ui::book::PageCover::refresh(&mut c));
            }
            break;
        }
        let _ = h.root.redraw();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let (w, hh) = (W as u16, H as u16);
    let mut partial = crate::render::Renderer::new(2);
    let mut full = crate::render::Renderer::new(2);
    let mut a = vec![0u32; (W * H) as usize];
    let mut b = vec![0u32; (W * H) as usize];
    for i in 0..40u32 {
        if i % 4 == 3 {
            h.pointer_move(f64::from(40 + i * 7), f64::from(80 + i * 11));
        } else {
            let state = PointerState { position: PhysicalPosition::new(600.0, 300.0), count: 1, scale_factor: 1.0, ..PointerState::default() };
            let dy = if i < 24 { -37.0 } else { 23.0 };
            h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, dy)), state }));
        }
        let (layers, _) = h.root.redraw();
        if let Some((area, pixels)) = partial.draw(&layers, w, hh, 1.0, theme::background(), i == 0).unwrap() {
            for (row, line) in pixels.chunks_exact(area.w as usize).enumerate() {
                let start = (area.y as usize + row) * W as usize + area.x as usize;
                a[start..start + line.len()].copy_from_slice(line);
            }
        }
        full.render(&layers, w, hh, 1.0, theme::background()).unwrap();
        b.copy_from_slice(full.frame_pixels());
        let channel = |p: u32, shift: u32| ((p >> shift) & 0xff) as i32;
        let differ = a.iter().zip(&b).filter(|&(&x, &y)| [0, 8, 16].iter().any(|&s| (channel(x, s) - channel(y, s)).abs() > 2)).count();
        assert_eq!(differ, 0, "шаг {i}: частичная отрисовка расходится с полной в {differ} пикселях");
    }
}

#[test]
#[ignore]
fn typing_cost() {
    let source = std::path::PathBuf::from(std::env::var("AQ_NOTE").unwrap());
    aq_trace::start();
    let mut h = Harness::with(None, 1.75);
    let note = h.vault.path().join("заметка.md");
    std::fs::copy(&source, &note).unwrap();
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    h.root.focus_on(h.app.editor_id());
    let _ = h.root.redraw();
    h.key(Key::Named(NamedKey::End), Modifiers::CONTROL);
    let before = h.app.editor_text.len();
    let mut renderer = crate::render::Renderer::new(4);
    let (w, hh) = ((f64::from(W) * 1.75) as u16, (f64::from(H) * 1.75) as u16);
    let (mut event, mut deliver, mut scene, mut raster) = (std::time::Duration::ZERO, std::time::Duration::ZERO, std::time::Duration::ZERO, std::time::Duration::ZERO);
    let n = 30u32;
    for i in 0..n {
        let t0 = std::time::Instant::now();
        let ev = KeyboardEvent { state: KeyState::Down, key: Key::Named(NamedKey::Backspace), code: Code::Backspace, location: Location::Standard, modifiers: Modifiers::empty(), repeat: i > 0, is_composing: false };
        h.root.handle_text_event(TextEvent::Keyboard(ev));
        let t1 = std::time::Instant::now();
        h.deliver();
        let t2 = std::time::Instant::now();
        let (layers, _) = h.root.redraw();
        let t3 = std::time::Instant::now();
        renderer.draw(&layers, w, hh, 1.75, theme::background(), i == 0).unwrap();
        let t4 = std::time::Instant::now();
        event += t1 - t0;
        deliver += t2 - t1;
        scene += t3 - t2;
        raster += t4 - t3;
    }
    if let Ok(out) = std::env::var("AQ_TRACE") {
        aq_trace::write(std::path::Path::new(&out)).unwrap();
    }
    eprintln!("удалено байт {}; нажатие: событие {:?}, действия {:?}, сцена {:?}, растр {:?}", before - h.app.editor_text.len(), event / n, deliver / n, scene / n, raster / n);
}

#[test]
#[ignore]
fn resize_cost() {
    use masonry::core::WindowEvent as MasonryWindowEvent;
    let source = std::path::PathBuf::from(std::env::var("AQ_NOTE").unwrap());
    let scale = 1.75;
    let mut h = Harness::with(None, scale);
    let note = h.vault.path().join("заметка.md");
    std::fs::copy(&source, &note).unwrap();
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    let mut renderer = crate::render::Renderer::new(4);
    let (mut scene, mut raster) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
    let n = 30u32;
    for i in 0..n {
        let (w, hh) = (2400 - i * 8, 1500 - i * 4);
        let t0 = std::time::Instant::now();
        h.root.handle_window_event(MasonryWindowEvent::Resize(masonry::dpi::PhysicalSize::new(w, hh)));
        h.app.window_resized(&mut h.root, Size::new(f64::from(w) / scale, f64::from(hh) / scale));
        h.deliver();
        let (layers, _) = h.root.redraw();
        let t1 = std::time::Instant::now();
        renderer.draw(&layers, w as u16, hh as u16, scale, theme::background(), true).unwrap();
        scene += t1 - t0;
        raster += t1.elapsed();
    }
    eprintln!("шаг изменения размера: сцена {:?}, растр {:?}", scene / n, raster / n);
}

#[test]
#[ignore]
fn chrome_snapshots() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let scale = 1.75;
    let mut h = Harness::with(None, scale);
    let note = h.vault.path().join("Заметка.md");
    std::fs::write(&note, "Текст заметки\n\nВторой абзац").unwrap();
    h.app.open(&mut h.root, note);
    let (w, hh) = ((f64::from(W) * scale) as u16, (f64::from(H) * scale) as u16);
    let shot = |h: &mut Harness, name: &str| {
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, w, hh, scale, theme::background()).unwrap();
        std::fs::write(out.join(name), r.png()).unwrap();
    };
    h.root.handle_text_event(masonry::core::TextEvent::WindowFocusChange(true));
    h.root.focus_on(h.app.editor_id());
    h.key(Key::Named(NamedKey::End), Modifiers::empty());
    shot(&mut h, "normal.png");
    h.app.toggle_focus_mode(&mut h.root);
    h.deliver();
    shot(&mut h, "focus.png");
    h.app.toggle_focus_mode(&mut h.root);
    h.deliver();
    shot(&mut h, "after.png");
}

#[test]
#[ignore]
fn markdown_snapshot() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let scale = 1.75;
    let mut h = Harness::with(None, scale);
    let note = h.vault.path().join("Разметка.md");
    let body = "# Заголовок первого уровня
## Второй уровень
### Третий
Обычный текст с *курсивом*, **жирным**, ~~зачёркнутым~~ и `кодом в строке`, а ещё [ссылка](https://example.com) и [[Вики|алиас]].

- Пункт списка, достаточно длинный, чтобы перенестись на вторую строку и показать висячий отступ под текстом пункта
	- Вложенный пункт
- [ ] Задача
- [x] Готовая задача
1. Первый
2. Второй

> Цитата в две строки, длинная настолько, чтобы перенестись и показать отступ после полосы
> Вторая строка цитаты

---

```rust
fn main() {
    println!(\"привет\");
}
```
Конец";
    std::fs::write(&note, body).unwrap();
    h.app.open(&mut h.root, note);
    let (w, hh) = ((f64::from(W) * scale) as u16, (f64::from(H) * scale) as u16);
    let shot = |h: &mut Harness, name: &str| {
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, w, hh, scale, theme::background()).unwrap();
        std::fs::write(out.join(name), r.png()).unwrap();
    };
    shot(&mut h, "markdown.png");
    h.root.handle_text_event(masonry::core::TextEvent::WindowFocusChange(true));
    h.root.focus_on(h.app.editor_id());
    h.key(Key::Named(NamedKey::ArrowDown), Modifiers::empty());
    h.key(Key::Named(NamedKey::ArrowDown), Modifiers::empty());
    h.key(Key::Named(NamedKey::ArrowDown), Modifiers::empty());
    h.key(Key::Named(NamedKey::End), Modifiers::empty());
    shot(&mut h, "markdown-caret.png");
    h.key(Key::Named(NamedKey::End), Modifiers::CONTROL);
    shot(&mut h, "markdown-end.png");
}

#[test]
#[ignore]
fn note_snapshot() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let note = std::path::PathBuf::from(std::env::var("AQ_NOTE").unwrap());
    let vault = std::path::PathBuf::from(std::env::var("AQ_VAULT").unwrap());
    let scale = 1.75;
    let mut h = Harness::with(Some(&vault), scale);
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    for _ in 0..200 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
    }
    let size = h.root.size();
    let (w, hh) = (size.width as u16, size.height as u16);
    let editor = h.app.editor_id().unwrap();
    let carets = std::env::var("AQ_CARET").unwrap_or_default();
    for (i, needle) in std::iter::once("").chain(carets.split('|').filter(|s| !s.is_empty())).enumerate() {
        if !needle.is_empty() {
            let text = h.root.get_widget(editor).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).unwrap().inner().text().to_owned();
            let at = text.find(needle).unwrap_or(0);
            h.root.handle_text_event(masonry::core::TextEvent::WindowFocusChange(true));
            h.root.focus_on(Some(editor));
            h.root.edit_widget(editor, |mut w| crate::ui::editor::TextEditor::select_byte_range(&mut w.downcast(), at, at));
        }
        if let Some(dy) = std::env::var("AQ_SCROLL").ok().and_then(|v| v.parse::<f64>().ok()).filter(|_| i > 0) {
            use masonry::core::{PointerScrollEvent, ScrollDelta};
            let state = PointerState { position: PhysicalPosition::new(f64::from(w) / 2.0, f64::from(hh) / 2.0), count: 1, scale_factor: scale, ..PointerState::default() };
            h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -dy)), state }));
            h.deliver();
        }
        if let Some((x, y)) = std::env::var("AQ_HOVER").ok().and_then(|v| v.split_once(',').map(|(x, y)| (x.parse::<f64>().unwrap(), y.parse::<f64>().unwrap()))) {
            let state = PointerState { position: PhysicalPosition::new(x, y), count: 1, scale_factor: scale, ..PointerState::default() };
            h.root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: MOUSE, current: state, coalesced: vec![], predicted: vec![] }));
        }
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, w, hh, scale, theme::background()).unwrap();
        std::fs::write(out.join(format!("note-{i}.png")), r.png()).unwrap();
    }
}

#[test]
fn note_reopens_at_saved_scroll_position() {
    use masonry::core::{PointerScrollEvent, ScrollDelta};
    let mut h = Harness::new();
    let long = h.vault.path().join("Длинная.md");
    let body: String = (0..300).map(|i| format!("Абзац {i}\n\n")).collect();
    std::fs::write(&long, &body).unwrap();
    let other = h.vault.path().join("Другая.md");
    std::fs::write(&other, "текст").unwrap();
    h.app.open(&mut h.root, long.clone());
    let _ = h.root.redraw();
    let scroll = h.app.note_ids.as_ref().unwrap().scroll.id();
    let offset = |h: &Harness| h.root.get_widget(scroll).and_then(|w| w.downcast::<crate::ui::scroll::ScrollArea<masonry::widgets::Flex>>()).map(|w| w.inner().offset());
    let state = PointerState { position: PhysicalPosition::new(600.0, 300.0), count: 1, scale_factor: 1.0, ..PointerState::default() };
    h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -2500.0)), state }));
    h.deliver();
    let saved = offset(&h).unwrap();
    assert!(saved > 2000.0, "прокрутка не сработала: {saved}");
    h.app.open(&mut h.root, other);
    let _ = h.root.redraw();
    h.app.open(&mut h.root, long);
    let _ = h.root.redraw();
    assert!(h.app.restore_view(&mut h.root), "позиция не восстановлена");
    let _ = h.root.redraw();
    let scroll = h.app.note_ids.as_ref().unwrap().scroll.id();
    let restored = h.root.get_widget(scroll).and_then(|w| w.downcast::<crate::ui::scroll::ScrollArea<masonry::widgets::Flex>>()).map(|w| w.inner().offset()).unwrap();
    assert!((restored - saved).abs() < 1.0, "сохранено {saved}, восстановлено {restored}");
}

#[test]
#[ignore]
fn real_tree_after_focus_mode() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let vault = std::path::PathBuf::from(std::env::var("AQ_VAULT").unwrap());
    let mut h = Harness::with(Some(&vault), 1.0);
    for _ in 0..100 {
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let shot = |h: &mut Harness, name: &str| {
        for _ in 0..20 {
            h.app.on_core_events(&mut h.root);
            h.deliver();
        }
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
        std::fs::write(out.join(name), r.png()).unwrap();
    };
    shot(&mut h, "tree-before.png");
    h.app.toggle_focus_mode(&mut h.root);
    shot(&mut h, "tree-focus.png");
    h.app.toggle_focus_mode(&mut h.root);
    shot(&mut h, "tree-after.png");
}

#[test]
fn tree_rows_survive_focus_mode() {
    let vault = tempfile::tempdir().unwrap();
    for i in 0..1000 {
        std::fs::write(vault.path().join(format!("Заметка {i:04}.md")), "текст").unwrap();
    }
    let mut h = Harness::with(Some(vault.path()), 1.0);
    h.app.open(&mut h.root, vault.path().join("Заметка 0900.md"));
    h.deliver();
    let offset = |h: &Harness| {
        let id = h.app.tree.scroll_id().unwrap();
        h.root.get_widget(id).and_then(|w| w.downcast::<crate::ui::scroll::ScrollArea<masonry::widgets::Flex>>()).map(|w| w.inner().offset()).unwrap()
    };
    let before = (offset(&h), h.app.tree.built_rows());
    h.app.toggle_focus_mode(&mut h.root);
    h.deliver();
    h.app.toggle_focus_mode(&mut h.root);
    h.deliver();
    let after = (offset(&h), h.app.tree.built_rows());
    assert_eq!(before, after);
}

#[test]
fn wiki_link_opens_note_from_subfolder_without_creating_duplicate() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::create_dir_all(vault.join("Base")).unwrap();
    std::fs::write(vault.join("Base").join("📖 Книга.md"), "---\nauthor: А\n---\nтекст").unwrap();
    h.app.open(&mut h.root, vault.join("a.md"));
    let mut opened = false;
    for _ in 0..250 {
        h.app.on_core_events(&mut h.root);
        if h.app.open_target(&mut h.root, crate::links::Target::Wiki("📖 Книга".into()), true) {
            opened = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(opened, "ссылка не открылась");
    assert!(!vault.join("📖 Книга.md").exists(), "создан дубль в корне");
    assert_eq!(h.tab_paths(), [Some("a".to_owned()), Some("📖 Книга".to_owned())]);
}

#[test]
#[ignore]
fn books_snapshot() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let scale = 1.75;
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().to_path_buf();
    std::fs::create_dir_all(vault.join("Base")).unwrap();
    std::fs::write(vault.join("Base").join("📖 Победители недр.md"), "---\nauthor: Иван Ефремов\npages: 120/300\nПуть к файлу: book.epub\n---\n").unwrap();
    std::fs::write(vault.join("book.epub"), "").unwrap();
    std::fs::write(vault.join("Base").join("📖 Марсианские хроники.md"), "---\nauthor: Рэй Брэдбери\npages: 300/300\n---\n").unwrap();
    std::fs::write(vault.join("Base").join("📖 Очень длинное название книги, которое точно не поместится в одну строку виджета.md"), "").unwrap();
    let note = vault.join("Книги.md");
    std::fs::write(&note, "Текст\n\n> [!book] [[📖 Победители недр]]\n> [!book] [[📖 Марсианские хроники]]\n\n> [!book] [[📖 Очень длинное название книги, которое точно не поместится в одну строку виджета]]\n> [!book] [[📖 Нет такой книги]]\n\nПосле").unwrap();
    let mut h = Harness::with(Some(&vault), scale);
    std::thread::sleep(std::time::Duration::from_secs(2));
    h.app.open(&mut h.root, note);
    for _ in 0..100 {
        std::thread::sleep(std::time::Duration::from_millis(30));
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
    }
    let size = h.root.size();
    for (x, y) in [(1034.0, 457.0), (1034.0, 625.0)] {
        let state = PointerState { position: PhysicalPosition::new(x, y), count: 1, scale_factor: scale, ..PointerState::default() };
        h.root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: MOUSE, current: state, coalesced: vec![], predicted: vec![] }));
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, size.width as u16, size.height as u16, scale, theme::background()).unwrap();
        std::fs::write(out.join(format!("books-{y}.png")), r.png()).unwrap();
    }
}

#[test]
fn book_widgets_and_dataview_follow_changes() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    let book = vault.join("📖 Книга.md");
    std::fs::write(&book, "---
author: Первый
pages: 10/100
---
").unwrap();
    std::fs::write(vault.join("ссылка.md"), "[[📖 Книга]]").unwrap();
    let query = "LIST FROM [[📖 Книга]]";
    let note = vault.join("Главная.md");
    std::fs::write(&note, format!("> [!book] [[📖 Книга]]

```dataview
{query}
```
")).unwrap();
    h.app.open(&mut h.root, note);
    let wait = |h: &mut Harness, done: &dyn Fn(&Harness) -> bool| {
        for _ in 0..300 {
            h.app.on_core_events(&mut h.root);
            let _ = h.root.redraw();
            if done(h) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        false
    };
    let embeds = |h: &Harness| h.app.embeds.as_ref().unwrap().clone();
    let ok = wait(&mut h, &|h| embeds(h).cached_book("📖 Книга").is_some_and(|b| b.pages == Some((10, 100))) && embeds(h).cached_rows(query) == Some(2));
    assert!(ok, "{:?} {:?}", embeds(&h).cached_book("📖 Книга"), embeds(&h).cached_rows(query));
    let core = h.app.workspace.core.clone();
    let snapshot = aquilum_core::files::document::read_file_snapshot_impl(&book).unwrap();
    aquilum_core::files::gate::write(&core, &book, "---
author: Второй
pages: 50/100
---
", Some(&snapshot.hash), aquilum_core::history::Source::Me, None).unwrap();
    assert!(wait(&mut h, &|h| embeds(h).cached_book("📖 Книга").is_some_and(|b| b.pages == Some((50, 100)) && b.author == "Второй")), "виджет книги не обновился");
    aquilum_core::files::gate::create(&core, &vault.join("новая.md"), "[[📖 Книга]]", aquilum_core::history::Source::Me, None).unwrap();
    assert!(wait(&mut h, &|h| embeds(h).cached_rows(query) == Some(3)), "новая ссылка не попала в таблицу");
}

fn sample_fb2() -> String {
    let paragraphs: String = (1..=400).map(|i| format!("<p>Абзац номер {i}. Съешь же ещё этих мягких французских булок да выпей чаю, повторяя это снова и снова.</p>")).collect();
    format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><FictionBook xmlns=\"http://www.gribuser.ru/xml/fictionbook/2.0\"><body><section><title><p>Глава первая</p></title>{paragraphs}</section><section><title><p>Глава вторая</p></title>{paragraphs}</section></body></FictionBook>")
}

impl Harness {
    fn reader(&self) -> bool {
        matches!(&self.app.popup, Some(Popup::Reader(_)))
    }
}

#[test]
fn reader_turns_pages_and_saves_position() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::create_dir_all(vault.join("Files")).unwrap();
    let file = vault.join("Files").join("книга.fb2");
    std::fs::write(&file, sample_fb2()).unwrap();
    let page = vault.join("📖 Книга.md");
    std::fs::write(&page, "---
Путь к файлу: Files/книга.fb2
pages: 
---
").unwrap();
    h.app.open_reader(&mut h.root, super::reader::Opening { file: file.clone(), page: Some(page.clone()), title: "Книга".into(), initial: None, peek: false });
    h.deliver();
    let _ = h.root.redraw();
    h.deliver();
    assert!(h.reader(), "читалка не открылась");
    let fields = |p: &std::path::Path| std::fs::read_to_string(p).unwrap();
    assert!(fields(&page).contains("pages: 0/"), "{}", fields(&page));
    for _ in 0..5 {
        h.key(Key::Named(NamedKey::ArrowRight), Modifiers::empty());
        let _ = h.root.redraw();
        h.deliver();
    }
    let text = fields(&page);
    assert!(text.contains("reader_position: epubcfi(/6/2!"), "{text}");
    let first = text.lines().find(|l| l.starts_with("reader_position")).unwrap().to_owned();
    for _ in 0..200 {
        h.key(Key::Named(NamedKey::ArrowRight), Modifiers::empty());
        let _ = h.root.redraw();
        h.deliver();
    }
    let text = fields(&page);
    assert!(text.contains("reader_position: epubcfi(/6/4!"), "вторая глава не открылась: {text}");
    assert_ne!(first, text.lines().find(|l| l.starts_with("reader_position")).unwrap());
    let percent: u32 = text.lines().find_map(|l| l.strip_prefix("read_percent: ")).unwrap().trim().parse().unwrap();
    assert!(percent > 50, "{text}");
    h.escape();
    assert!(!h.reader(), "Escape не закрыл читалку");
    let saved = text.lines().find_map(|l| l.strip_prefix("reader_position: ")).unwrap().to_owned();
    h.app.open_reader(&mut h.root, super::reader::Opening { file: file.clone(), page: Some(page.clone()), title: "Книга".into(), initial: None, peek: false });
    h.deliver();
    let _ = h.root.redraw();
    h.deliver();
    let text = fields(&page);
    let reopened = text.lines().find_map(|l| l.strip_prefix("reader_position: ")).unwrap();
    assert_eq!(reopened, saved, "позиция не восстановилась");
}

#[test]
fn reader_settings_inputs_keep_popover_open() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    let file = vault.join("книга.fb2");
    std::fs::write(&file, sample_fb2()).unwrap();
    h.app.open_reader(&mut h.root, super::reader::Opening { file, page: None, title: "Книга".into(), initial: None, peek: false });
    h.deliver();
    let _ = h.root.redraw();
    h.click(936.0, 30.0);
    let _ = h.root.redraw();
    let size_field = match &h.app.popup {
        Some(Popup::Reader(state)) => state
            .bindings
            .iter()
            .find_map(|(id, b)| matches!(b, crate::ui::settings::Binding::Number { number, .. } if number.max == 32.0).then_some(*id))
            .expect("поле размера шрифта"),
        _ => panic!("читалка закрылась"),
    };
    let open = |h: &Harness| match &h.app.popup {
        Some(Popup::Reader(state)) => h.root.get_widget(state.layer).and_then(|w| w.downcast::<crate::ui::reader::ReaderView>().map(|v| v.inner().has_popover())).unwrap_or(false),
        _ => false,
    };
    assert!(open(&h), "настройки не открылись");
    let field = h.root.get_widget(size_field).unwrap().ctx().bounding_box();
    h.click(field.center().x, field.center().y);
    let _ = h.root.redraw();
    assert!(open(&h), "щелчок в поле закрыл настройки");
    h.key(Key::Character("a".into()), Modifiers::CONTROL);
    for ch in ["2", "0"] {
        h.key(Key::Character(ch.into()), Modifiers::empty());
    }
    let _ = h.root.redraw();
    assert!(open(&h));
    assert_eq!(h.config().reader.font.font_size_base, 20);
    h.click(500.0, 300.0);
    let _ = h.root.redraw();
    assert!(!open(&h), "щелчок по книге не закрыл настройки");
    assert!(h.reader());
}

#[test]
fn font_size_field_keeps_focus_and_blank_means_default() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    let editor_nav = match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog.nav.iter().find(|(_, s)| *s == Section::Editor).map(|(id, _)| *id).unwrap(),
        _ => panic!("настройки не открылись"),
    };
    let nav = h.root.get_widget(editor_nav).unwrap().ctx().bounding_box();
    h.click(nav.center().x, nav.center().y);
    let field = |h: &Harness| match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog
            .bindings
            .iter()
            .find_map(|(id, b)| matches!(b, crate::ui::settings::Binding::Number { number, .. } if number.blank).then_some(*id))
            .unwrap(),
        _ => panic!("настройки закрылись"),
    };
    let rect = h.root.get_widget(field(&h)).unwrap().ctx().bounding_box();
    h.click(rect.center().x, rect.center().y);
    h.key(Key::Named(NamedKey::End), Modifiers::empty());
    h.key(Key::Named(NamedKey::Backspace), Modifiers::empty());
    let _ = h.root.redraw();
    assert_eq!(h.config().editor.font.font_size_base, 1);
    assert_eq!(h.root.focused_widget(), Some(field(&h)), "фокус слетел");
    h.key(Key::Named(NamedKey::Backspace), Modifiers::empty());
    let _ = h.root.redraw();
    assert_eq!(h.config().editor.font.font_size_base, 16, "пустое поле — размер по умолчанию");
    assert_eq!(h.root.focused_widget(), Some(field(&h)), "фокус слетел");
    let text = |h: &Harness| h.root.get_widget(field(h)).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).unwrap().inner().text().to_owned();
    assert_eq!(text(&h), "");
    for ch in ["2", "0"] {
        h.key(Key::Character(ch.into()), Modifiers::empty());
    }
    let _ = h.root.redraw();
    assert_eq!(h.config().editor.font.font_size_base, 20);
    assert_eq!(text(&h), "20");
    assert_eq!(h.root.focused_widget(), Some(field(&h)), "фокус слетел");
}

#[test]
fn graph_tab_loads_opens_notes_and_unloads() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("b.md"), "[[a]] [[c]]").unwrap();
    std::fs::write(vault.join("c.md"), "[[a]]").unwrap();
    h.app.tree.paths_changed(&mut h.root, &[vault.join("b.md"), vault.join("c.md")]);
    let button = h.app.chrome.as_ref().unwrap().graph_button.expect("кнопка графа");
    let rect = h.root.get_widget(button).unwrap().ctx().bounding_box();
    let wait = |h: &mut Harness, done: &dyn Fn(&Harness) -> bool| {
        for _ in 0..300 {
            h.app.on_core_events(&mut h.root);
            let _ = h.root.redraw();
            if done(h) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        false
    };
    h.click(rect.center().x, rect.center().y);
    assert!(h.app.tabs.active().graph, "вкладка графа не открылась");
    assert!(wait(&mut h, &|h| h.app.graph_loaded() == Some(3)), "граф не загрузился: {:?}", h.app.graph_loaded());
    h.click(rect.center().x, rect.center().y);
    assert_eq!(h.app.tabs.tabs().iter().filter(|t| t.graph).count(), 1, "второй граф");
    let view = h.app.graph_view().unwrap();
    let area = h.root.get_widget(view).unwrap().ctx().bounding_box();
    let mut opened = false;
    'scan: for y in (area.y0 as i32..area.y1 as i32).step_by(6) {
        for x in (area.x0 as i32..(area.x1 - 260.0) as i32).step_by(6) {
            h.root.handle_pointer_event(PointerEvent::Move(PointerUpdate {
                pointer: MOUSE,
                current: PointerState { position: PhysicalPosition::new(f64::from(x), f64::from(y)), count: 1, scale_factor: 1.0, ..PointerState::default() },
                coalesced: vec![],
                predicted: vec![],
            }));
            if h.root.cursor_icon() == masonry::core::CursorIcon::Pointer {
                h.click(f64::from(x), f64::from(y));
                opened = true;
                break 'scan;
            }
        }
    }
    assert!(opened, "под курсором не нашлось ни одного узла");
    assert!(!h.app.tabs.active().graph && h.app.tabs.active().path.is_some(), "щелчок по узлу не открыл заметку");
    assert!(h.app.tabs.has_graph() && h.app.graph_loaded() == Some(3), "граф закрылся вместе с переходом к заметке");
    let graph = h.app.tabs.tabs().iter().find(|t| t.graph).unwrap().id;
    h.app.tabs.close(graph);
    h.app.show_active(&mut h.root);
    assert!(h.app.graph_released(), "граф остался в памяти после закрытия вкладки");
}

#[test]
#[ignore]
fn graph_snapshot() {
    use masonry::core::{PointerScrollEvent, ScrollDelta};
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let vault = std::path::PathBuf::from(std::env::var("AQ_VAULT").unwrap());
    let scale = 1.5;
    let mut h = Harness::with(Some(&vault), scale);
    h.app.open_graph(&mut h.root);
    for _ in 0..600 {
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
        if h.app.graph_loaded().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    eprintln!("узлов: {:?}", h.app.graph_loaded());
    let size = h.root.size();
    let shot = |h: &mut Harness, name: &str| {
        for _ in 0..40 {
            h.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_millis(16)));
        }
        let started = std::time::Instant::now();
        let (layers, _) = h.root.redraw();
        let built = started.elapsed();
        let mut r = crate::render::Renderer::new(4);
        r.render(&layers, size.width as u16, size.height as u16, scale, theme::background()).unwrap();
        eprintln!("{name}: сцена {built:?}, всего {:?}", started.elapsed());
        std::fs::write(out.join(format!("{name}.png")), r.png()).unwrap();
    };
    shot(&mut h, "graph-fit");
    let center = PhysicalPosition::new(f64::from(size.width) * 0.4, f64::from(size.height) * 0.5);
    for _ in 0..12 {
        let state = PointerState { position: center, count: 1, scale_factor: scale, ..PointerState::default() };
        h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::LineDelta(0.0, 1.0), state }));
    }
    shot(&mut h, "graph-zoom");
    let state = PointerState { position: center, count: 1, scale_factor: scale, ..PointerState::default() };
    h.root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: MOUSE, current: state, coalesced: vec![], predicted: vec![] }));
    shot(&mut h, "graph-hover");
}

#[test]
fn typing_suggests_notes_and_enter_links_them() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("Проектирование.md"), "x").unwrap();
    let note = vault.join("Черновик.md");
    std::fs::write(&note, "").unwrap();
    h.app.tree.paths_changed(&mut h.root, &[vault.join("Проектирование.md"), note.clone()]);
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    let editor = h.app.editor_id().unwrap();
    h.root.handle_text_event(TextEvent::WindowFocusChange(true));
    h.root.focus_on(Some(editor));
    let text = |h: &Harness| h.root.get_widget(editor).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).unwrap().inner().text().to_owned();
    for round in ["", "[["] {
        for ch in round.chars().chain("Прое".chars()) {
            h.key(Key::Character(ch.to_string().into()), Modifiers::empty());
        }
        let mut shown = false;
        for _ in 0..200 {
            h.app.on_core_events(&mut h.root);
            let _ = h.root.redraw();
            if h.app.suggest_layer.is_some() {
                shown = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(shown, "список подсказок не появился ({round:?}): {:?}", text(&h));
        if let Ok(out) = std::env::var("AQ_OUT") {
            let (layers, _) = h.root.redraw();
            let mut r = crate::render::Renderer::new(2);
            r.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
            std::fs::write(std::path::Path::new(&out).join(format!("suggest-{}.png", round.len())), r.png()).unwrap();
        }
        h.key(Key::Named(NamedKey::Enter), Modifiers::empty());
        let _ = h.root.redraw();
        h.app.on_core_events(&mut h.root);
        assert!(text(&h).ends_with("[[Проектирование]]"), "{:?}", text(&h));
        assert!(h.app.suggest_layer.is_none(), "список не закрылся");
        h.key(Key::Character(" ".into()), Modifiers::empty());
    }
    assert_eq!(text(&h), "[[Проектирование]] [[Проектирование]] ");
}

#[test]
fn reader_quotes_round_trip() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::create_dir_all(vault.join("Files")).unwrap();
    let file = vault.join("Files").join("книга.fb2");
    std::fs::write(&file, sample_fb2()).unwrap();
    let page = vault.join("📖 Книга.md");
    std::fs::write(&page, "---
Путь к файлу: Files/книга.fb2
pages: 
---
Заметки о книге").unwrap();
    h.app.open(&mut h.root, page.clone());
    let _ = h.root.redraw();
    h.app.open_reader(&mut h.root, super::reader::Opening { file: file.clone(), page: Some(page.clone()), title: "Книга".into(), initial: None, peek: false });
    h.deliver();
    let _ = h.root.redraw();
    h.deliver();
    h.drag((120.0, 160.0), (420.0, 220.0));
    let _ = h.root.redraw();
    h.deliver();
    let quote = match &h.app.popup {
        Some(Popup::Reader(state)) => state.ids.quote.unwrap(),
        _ => panic!("читалка закрылась"),
    };
    let button = h.root.get_widget(quote).unwrap().ctx().bounding_box();
    assert!(button.width() > 0.0, "нет кнопки цитаты");
    h.click(button.center().x, button.center().y);
    let _ = h.root.redraw();
    h.deliver();
    let text = h.app.editor_text.clone();
    let found = crate::reader::quotes::find(&text);
    assert_eq!(found.len(), 1, "{text}");
    assert_eq!(found[0].label, "1");
    assert_eq!(found[0].book.as_deref(), Some("Files/книга.fb2"));
    assert!(found[0].cfi.starts_with("epubcfi(/6/2!"), "{}", found[0].cfi);
    assert!(text[found[0].text.clone()].contains("Абзац номер"), "{text}");
    assert!(std::fs::read_to_string(&page).unwrap().contains("[!quote]"), "цитата не сохранена в файл");
    let _ = h.root.redraw();
    let label = match &h.app.popup {
        Some(Popup::Reader(state)) => h.root.get_widget(state.layer).and_then(|w| w.downcast::<crate::ui::reader::ReaderView>().map(|v| v.inner().label_rects())).unwrap(),
        _ => panic!("читалка закрылась"),
    };
    assert_eq!(label.len(), 1, "метка цитаты не нарисована");
    h.click(label[0].center().x, label[0].center().y);
    assert!(!h.reader(), "щелчок по метке не закрыл читалку");
    let selected = h.root.get_widget(h.app.editor_id().unwrap()).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).unwrap().inner().selected();
    assert_eq!(selected, text[found[0].text.clone()]);
    let url = format!("aquilum-reader:cfi={}", found[0].cfi.replace('(', "%28").replace(')', "%29"));
    h.app.open_target(&mut h.root, crate::links::Target::Url(url), false);
    h.deliver();
    let _ = h.root.redraw();
    assert!(h.reader(), "ссылка на цитату не открыла книгу");
}

#[test]
#[ignore]
fn reader_snapshot() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let book = std::path::PathBuf::from(std::env::var("AQ_BOOK").unwrap());
    let mut h = Harness::new();
    let note = h.vault.path().join("a.md");
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    h.app.open_reader(&mut h.root, super::reader::Opening { file: book.clone(), page: None, title: book.file_stem().unwrap().to_string_lossy().into_owned(), initial: None, peek: false });
    h.deliver();
    let _ = h.root.redraw();
    h.deliver();
    let size = h.root.size();
    for i in 0..4 {
        let started = std::time::Instant::now();
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, size.width as u16, size.height as u16, 1.0, theme::background()).unwrap();
        std::fs::write(out.join(format!("reader-{i}.png")), r.png()).unwrap();
        eprintln!("страница {i}: {:?}", started.elapsed());
        for _ in 0..3 {
            h.key(Key::Named(NamedKey::ArrowRight), Modifiers::empty());
            let _ = h.root.redraw();
            h.deliver();
        }
    }
    h.drag((120.0, 160.0), (420.0, 220.0));
    let _ = h.root.redraw();
    h.deliver();
    let (layers, _) = h.root.redraw();
    let mut r = crate::render::Renderer::new(2);
    r.render(&layers, size.width as u16, size.height as u16, 1.0, theme::background()).unwrap();
    std::fs::write(out.join("reader-selection.png"), r.png()).unwrap();
    h.click(936.0, 30.0);
    let _ = h.root.redraw();
    h.deliver();
    let (layers, _) = h.root.redraw();
    let mut r = crate::render::Renderer::new(2);
    r.render(&layers, size.width as u16, size.height as u16, 1.0, theme::background()).unwrap();
    std::fs::write(out.join("reader-settings.png"), r.png()).unwrap();
}

#[test]
fn removed_images_leave_caches() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    image::RgbaImage::new(40, 30).save(vault.join("pic.png")).unwrap();
    let note = vault.join("Картинка.md");
    std::fs::write(&note, "текст\n\n![](pic.png)\n\nконец").unwrap();
    h.app.open(&mut h.root, note);
    let cached = |h: &Harness| h.app.embeds.as_ref().unwrap().cached_media();
    for _ in 0..100 {
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
        if cached(&h) == 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(cached(&h), 2, "картинка не попала в кэш заметки и общий кэш");
    h.app.set_editor_text(&mut h.root, "текст\n\nконец".into());
    let _ = h.root.redraw();
    assert_eq!(cached(&h), 0, "удалённая с холста картинка осталась в кэше");
    h.app.set_editor_text(&mut h.root, "текст\n\n![](pic.png)\n\nконец".into());
    for _ in 0..100 {
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
        if cached(&h) == 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    h.app.embeds.as_ref().unwrap().forget_paths(&[vault.join("pic.png")]);
    assert_eq!(cached(&h), 0, "удалённый файл остался в кэше");
}

#[test]
fn settings_dropdown_toggles_like_web() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    h.click(115.0, 398.0);
    assert_eq!(h.section(), Some(Section::History));
    h.click(816.0, 173.0);
    assert!(h.menu_open());
    if let Ok(out) = std::env::var("AQ_OUT") {
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
        std::fs::write(std::path::Path::new(&out).join("dropdown.png"), r.png()).unwrap();
    }
    h.click(816.0, 173.0);
    assert!(!h.menu_open(), "повторный щелчок по полю не закрыл список");
    h.click(816.0, 173.0);
    assert!(h.menu_open());
    h.escape();
    assert!(!h.menu_open());
    assert_eq!(h.section(), Some(Section::History));
}

#[test]
#[ignore]
fn mcp_snapshot() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    h.click(115.0, 364.0);
    assert_eq!(h.section(), Some(Section::Mcp));
    assert!(!h.config().mcp.enabled);
    assert_eq!(h.config().mcp.token.len(), 32);
    {
        use masonry::core::{PointerScrollEvent, ScrollDelta};
        let state = PointerState { position: PhysicalPosition::new(600.0, 400.0), count: 1, scale_factor: 1.0, ..PointerState::default() };
        h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -420.0)), state }));
        h.deliver();
    }
    let (layers, _) = h.root.redraw();
    let mut r = crate::render::Renderer::new(2);
    r.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
    std::fs::write(out.join("mcp.png"), r.png()).unwrap();
}

#[test]
#[ignore]
fn book_page_snapshot() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let scale = 1.0;
    let mut h = Harness::with(None, scale);
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("book.epub"), "").unwrap();
    std::fs::write(vault.join("С обложкой.md"), "---\ncover: true\ntype: book\nPage_cover: pattern:tunnel\nПуть к файлу: book.epub\n---\n\nТекст").unwrap();
    std::fs::write(vault.join("Без обложки.md"), "---\ntype: book\n---\n\nТекст").unwrap();
    for name in ["С обложкой", "Без обложки"] {
        h.app.open(&mut h.root, vault.join(format!("{name}.md")));
        let _ = h.root.redraw();
        let size = h.root.size();
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, size.width as u16, size.height as u16, scale, theme::background()).unwrap();
        std::fs::write(out.join(format!("{name}.png")), r.png()).unwrap();
    }
}

#[test]
fn mcp_navigation_opens_notes_and_tracks_active_note() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("b.md"), "текст").unwrap();
    h.app.open(&mut h.root, vault.join("a.md"));
    assert_eq!(h.app.workspace.core.active_note.get(), Some(vault.join("a.md")));
    aquilum_core::mcp::bridge::open_note(&h.app.workspace.core, &vault.join("b.md"), true);
    for _ in 0..50 {
        h.app.on_core_events(&mut h.root);
        if h.tab_paths().len() == 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(h.tab_paths(), [Some("a".to_owned()), Some("b".to_owned())]);
    assert_eq!(h.app.workspace.core.active_note.get(), Some(vault.join("b.md")));
}

#[test]
fn editor_font_and_column_follow_settings() {
    let mut h = Harness::new();
    h.app.open(&mut h.root, h.vault.path().join("a.md"));
    let _ = h.root.redraw();
    let mut config = h.config();
    config.editor.font.font_size_base = 22;
    config.editor.max_width_ch = 50;
    config.editor.line_height = 1.8;
    h.app.workspace.core.settings.update_config(config).unwrap();
    let section = h.app.close_settings(&mut h.root);
    assert!(section.is_none());
    h.app.rebuild(&mut h.root);
    let _ = h.root.redraw();
    let font = theme::editor_font_settings();
    assert_eq!((font.size, font.max_width_ch, font.line_height), (22.0, 50, 1.8));
}
