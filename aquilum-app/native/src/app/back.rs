use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};

use super::{App, Outcome, Popup};
use crate::ui::dialog::DialogAnswer;
use crate::ui::menu::MenuChoice;
use crate::ui::reader::ReaderAction;
use crate::ui::settings::Pane;
use crate::ui::viewer::ViewerClosed;
use crate::ui::widgets::Pressed;

impl Popup {
    fn cancel(&self) -> (WidgetId, ErasedAction) {
        let dialog = |layer: WidgetId| (layer, Box::new(DialogAnswer::Cancel) as ErasedAction);
        let menu = |layer: WidgetId| (layer, Box::new(MenuChoice::Dismissed) as ErasedAction);
        match self {
            Popup::FileMenu { layer, .. }
            | Popup::VersionMenu { layer, .. }
            | Popup::TableMenu { layer, .. }
            | Popup::EditorMenu { layer, .. }
            | Popup::NoteActionsMenu { layer }
            | Popup::SortMenu { layer, .. }
            | Popup::TabsMenu { layer, .. } => menu(*layer),
            Popup::Settings { menu: Some(open), .. } => menu(open.layer),
            Popup::Settings { dialog: settings, .. } => match &settings.confirm {
                Some((confirm, _)) => dialog(confirm.layer),
                None if settings.pane() == Pane::Detail => (settings.back, Box::new(Pressed)),
                None => dialog(settings.layer),
            },
            Popup::Delete { dialog: ids, .. } => dialog(ids.layer),
            Popup::Workspaces(state) => dialog(state.forget.as_ref().map_or(state.dialog.layer, |(confirm, _)| confirm.layer)),
            Popup::Search(state) => dialog(state.layer),
            Popup::Templates(state) => dialog(state.layer),
            Popup::Reader(state) => (state.layer, Box::new(ReaderAction::Close)),
            Popup::Viewer { layer } => (*layer, Box::new(ViewerClosed)),
            Popup::ResetGraph(ids) => dialog(ids.layer),
        }
    }
}

impl App {
    pub(in crate::app) fn back(&mut self, root: &mut RenderRoot) -> Outcome {
        if let Some((layer, action)) = self.popup.as_ref().map(Popup::cancel) {
            if action.downcast_ref::<MenuChoice>().is_some() {
                root.remove_layer(layer);
            }
            return self.on_action(root, layer, action, self.window);
        }
        if self.interface.screen().drawer.is_some() {
            self.set_drawer(root, None);
        } else if self.renaming_tab.is_some() {
            self.cancel_tab_rename(root);
        } else if self.find.is_some() {
            self.close_find(root);
        } else {
            let outcome = self.navigate(root, -1);
            if !outcome.title {
                crate::system::leave();
            }
            return outcome;
        }
        Outcome::default()
    }
}
