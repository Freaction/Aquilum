use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Point;
use crate::ui::editor::TextAction;

use super::super::{App, Outcome};
use crate::ui::editor::TextEditor;
use crate::ui::scroll::ScrollArea;
use crate::ui::note::{MetadataToggled, TitleAction, TitleField};
use crate::ui::widgets::Pressed;

impl App {
    pub(in crate::app) fn on_note_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Option<Outcome> {
        if let Some(err_ids) = &self.open_error_ids {
            if action.downcast_ref::<Pressed>().is_some() {
                if id == err_ids.retry {
                    self.show_active(root);
                    return Some(Outcome::default());
                }
                if id == err_ids.reveal {
                    if let Some((path, _)) = &self.open_error {
                        crate::system::reveal(path);
                    }
                    return Some(Outcome::default());
                }
                if id == err_ids.close {
                    let active = self.tabs.active_id();
                    self.tabs.close(active);
                    self.show_active(root);
                    return Some(Outcome { title: true, ..Outcome::default() });
                }
            }
        }

        if let Some(save_ids) = &self.save_error_ids {
            if action.downcast_ref::<Pressed>().is_some() && id == save_ids.retry {
                self.retry_save(root);
                return Some(Outcome::default());
            }
        }

        let ids = self.note_ids.as_ref()?;
        if id == ids.scroll.id() {
            if action.downcast_ref::<crate::ui::scroll::Scrolled>().is_some() {
                self.capture_view(root);
            }
            return Some(Outcome::default());
        }
        if id == ids.body.id() {
            let (scroll, body) = (ids.scroll, ids.body);
            match action.downcast_ref::<TextAction>() {
                Some(TextAction::Page { down, extend }) => {
                    let (down, extend) = (*down, *extend);
                    let dy = scroll
                        .edit(root, |mut area| {
                            let step = area.widget.page();
                            ScrollArea::scroll_by(&mut area, if down { step } else { -step })
                        })
                        .unwrap_or(0.0);
                    body.edit(root, |mut editor| TextEditor::page(&mut editor, down, dy, extend));
                }
                Some(TextAction::Line { down }) => {
                    let line = if *down { crate::ui::keymap::gesture::WHEEL_LINE } else { -crate::ui::keymap::gesture::WHEEL_LINE };
                    scroll.edit(root, |mut area| ScrollArea::scroll_by(&mut area, line));
                }
                Some(TextAction::Edited(edits)) => self.body_edited(root, edits),
                _ => {}
            }
            return Some(Outcome::default());
        }
        if id == ids.title.id() {
            match action.downcast_ref::<TextAction>() {
                Some(TextAction::Changed(text)) => {
                    let empty = text.is_empty();
                    ids.title_box.edit(root, |mut field| TitleField::set_empty(&mut field, empty));
                }
                Some(TextAction::Entered(text)) => {
                    let text = text.clone();
                    self.commit_title(root, &text);
                    if let Some(ids) = &self.note_ids {
                        let body = ids.body;
                        root.focus_on(Some(body.id()));
                        body.edit(root, |mut body| TextEditor::select_byte_range(&mut body, 0, 0));
                    }
                }
                _ => {}
            }
            return Some(Outcome::default());
        }
        if id == ids.title_box.id() && action.downcast_ref::<TitleAction>().is_some() {
            let title = ids.title;
            let text = title.get(root).map(|t| t.inner().text()).unwrap_or_default();
            self.commit_title(root, &text);
            return Some(Outcome::default());
        }
        if action.downcast_ref::<Pressed>().is_some() {
            if id == ids.back {
                return Some(self.navigate(root, -1));
            }
            if id == ids.forward {
                return Some(self.navigate(root, 1));
            }
            if id == ids.focus {
                self.toggle_focus_mode(root);
                return Some(Outcome::default());
            }
            if id == ids.search {
                self.open_find(root);
                return Some(Outcome::default());
            }
            if id == ids.more {
                let transform = root.get_widget(id).map_or(Point::ORIGIN, |w| w.ctx().window_transform() * Point::ZERO);
                self.open_note_actions_menu(root, Point::new(transform.x, transform.y + crate::ui::tokens::size::SIZE_32));
                return Some(Outcome::default());
            }
        }
        if action.downcast_ref::<MetadataToggled>().is_some() {
            self.toggle_metadata(root);
            return Some(Outcome::default());
        }
        None
    }

    pub fn navigate(&mut self, root: &mut RenderRoot, delta: isize) -> Outcome {
        let Some(path) = self.navigation.step(delta) else { return Outcome::default() };
        if !path.is_file() {
            return Outcome::default();
        }
        self.tabs.open(path);
        self.show_active(root);
        Outcome { title: true, ..Outcome::default() }
    }
}
