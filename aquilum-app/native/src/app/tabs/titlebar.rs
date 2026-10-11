use masonry::app::RenderRoot;
use masonry::kurbo::Size;

use super::super::{App, Outcome, WindowCommand};
use crate::ui::titlebar::{TitlebarAction, WindowButton};

impl App {
    pub(in crate::app) fn on_titlebar(&mut self, root: &mut RenderRoot, action: &TitlebarAction, window_size: Size) -> Outcome {
        if self.renaming_tab.is_some() {
            self.finish_tab_rename(root, None);
        }
        let window = |command| Outcome { window: Some(command), ..Outcome::default() };
        match action {
            TitlebarAction::Menu(id, at) => {
                let Some(path) = self.tabs.tabs().iter().find(|t| t.id == *id).and_then(|t| t.path.clone()) else {
                    return Outcome::default();
                };
                self.open_file_menu(root, vec![path.clone()], path, false, *at, window_size, Some(*id));
                Outcome::default()
            }
            TitlebarAction::Reorder(from, to) => {
                if self.tabs.reorder(*from, *to) {
                    self.refresh_tabs(root);
                }
                Outcome::default()
            }
            TitlebarAction::Select(id) => {
                if self.tabs.select(*id) {
                    self.show_active(root);
                }
                Outcome { title: true, ..Outcome::default() }
            }
            TitlebarAction::Close(id) => {
                self.tabs.close(*id);
                self.show_active(root);
                Outcome { title: true, ..Outcome::default() }
            }
            TitlebarAction::Drag => window(WindowCommand::Drag),
            TitlebarAction::ToggleMaximize => window(WindowCommand::ToggleMaximize),
            TitlebarAction::Minimize => window(WindowCommand::Minimize),
            TitlebarAction::CloseWindow => window(WindowCommand::Close),
        }
    }

    pub fn set_maximized(&mut self, root: &mut RenderRoot, maximized: bool) {
        if self.maximized == maximized {
            return;
        }
        self.maximized = maximized;
        if let Some((_, button)) = self.controls {
            button.edit(root, |mut button| WindowButton::set_maximized(&mut button, maximized));
        }
    }
}
