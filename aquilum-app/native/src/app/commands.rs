use masonry::app::RenderRoot;

use super::{App, Outcome};
use crate::ui::keymap::Command;

impl App {
    pub fn command(&mut self, root: &mut RenderRoot, command: Command) -> Option<Outcome> {
        let zoom = matches!(command, Command::ZoomIn | Command::ZoomOut | Command::ZoomReset);
        if self.popup.is_some() && !zoom {
            return None;
        }
        let done = Outcome::default();
        Some(match command {
            Command::NewNote => Outcome { title: self.new_note(root), ..done },
            Command::NewTab => self.new_tab(root),
            Command::CloseTab => self.close_active_tab(root),
            Command::NextTab => self.cycle_tab(root, true),
            Command::PreviousTab => self.cycle_tab(root, false),
            Command::Back => self.navigate(root, -1),
            Command::Forward => self.navigate(root, 1),
            Command::QuickOpen => {
                self.open_search(root);
                done
            }
            Command::Templates => {
                self.open_templates(root);
                done
            }
            Command::FindInNote => {
                self.open_find(root);
                done
            }
            Command::FocusMode => {
                self.toggle_focus_mode(root);
                done
            }
            Command::WideMode => {
                self.toggle_wide_mode(root);
                done
            }
            Command::ZoomIn => Outcome { rescale: self.zoom(true), ..done },
            Command::ZoomOut => Outcome { rescale: self.zoom(false), ..done },
            Command::ZoomReset => Outcome { rescale: self.interface.set_scale(1.0), ..done },
            _ => return None,
        })
    }

    fn cycle_tab(&mut self, root: &mut RenderRoot, forward: bool) -> Outcome {
        let ids: Vec<_> = self.tabs.tabs().iter().map(|tab| tab.id).collect();
        if ids.is_empty() {
            return Outcome::default();
        }
        let current = ids.iter().position(|&id| id == self.tabs.active_id()).unwrap_or(0);
        let next = ids[crate::ui::keymap::cycle(current, ids.len(), forward)];
        if self.tabs.select(next) {
            self.show_active(root);
        }
        Outcome { title: true, ..Outcome::default() }
    }
}
