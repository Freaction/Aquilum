pub mod backlinks;
pub mod book;
pub mod dialog;
pub mod file_tree;
pub mod handle;
pub mod menu;
pub mod note;
pub mod editor;
pub mod scroll;
pub mod update;
pub mod search;
pub mod find;
pub mod settings;
pub mod style;
pub mod icons;
pub mod text;
pub mod theme;
pub mod titlebar;
pub mod timer;
pub mod graph;
pub mod motion;
pub mod reader;
pub mod suggest;
pub mod viewer;
pub mod tokens;
pub mod widgets;
pub mod workspaces;

mod frame;
mod header;
mod new_tab;
mod rail;
mod shell;
mod sidebar;
#[cfg(test)]
mod snapshot;

use masonry::core::WidgetTag;

pub use header::panel_label;
pub use rail::{home_widget, sidebar_label};
pub use shell::{Chrome, RootParts, root};

pub const ROOT_TAG: WidgetTag<widgets::Slot> = WidgetTag::named("root");

pub fn rescale(root: &mut masonry::app::RenderRoot, scale: f64) {
    let size = root.size();
    root.handle_window_event(masonry::core::WindowEvent::Rescale(scale));
    root.handle_window_event(masonry::core::WindowEvent::Resize(size));
}
