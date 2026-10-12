use std::path::PathBuf;

use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};

use super::{App, depth_hint};
use crate::interface::GraphPrefs;
use crate::ui::graph::GraphAction;
use crate::ui::settings::controls::{Changed, Slider};
use crate::ui::widgets::Pressed;

impl App {
    pub(in crate::app) fn is_graph_widget(&self, id: WidgetId) -> bool {
        let Some(c) = &self.graph.controls else { return false };
        self.graph.view.is_some_and(|v| v.id() == id) || [c.refresh, c.node_size, c.spread, c.depth, c.created, c.labels, c.heat].contains(&id)
    }

    pub(in crate::app) fn on_graph_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> bool {
        if let Some(action) = action.downcast_ref::<GraphAction>() {
            match action {
                GraphAction::Open(path) => return self.open_in(root, PathBuf::from(path), true),
                GraphAction::Camera(x, y, scale) => {
                    if let Some(session) = &mut self.session {
                        session.save_camera(&self.workspace.core, (*x, *y, *scale));
                    }
                }
                GraphAction::ShowAll => {
                    self.graph.share = 0.0;
                    self.refresh_created_hint(root);
                    self.push_display(root);
                }
                GraphAction::Retry => {
                    self.load_graph();
                }
            }
            return false;
        }
        let Some(c) = &self.graph.controls else { return false };
        let (refresh, node_size, spread, depth, created, labels, heat) = (c.refresh, c.node_size, c.spread, c.depth, c.created, c.labels, c.heat);
        if id == refresh && action.downcast_ref::<Pressed>().is_some() {
            self.load_graph();
            return false;
        }
        let Some(changed) = action.downcast_ref::<Changed>() else { return false };
        let mut prefs: GraphPrefs = self.interface.graph();
        let mut hint = None;
        match changed {
            Changed::Value(v) if id == node_size => {
                prefs.node_size = *v;
                hint = Some(format!("{v:.2}×"));
            }
            Changed::Value(v) if id == spread => {
                prefs.spread = *v;
                hint = Some(format!("{v:.2}×"));
            }
            Changed::Value(v) if id == depth => {
                prefs.highlight_depth = v.round() as usize;
                hint = Some(depth_hint(prefs.highlight_depth));
            }
            Changed::Value(v) if id == created => self.graph.share = *v,
            Changed::Bool(on) if id == labels => prefs.labels = *on,
            Changed::Choice(index) if id == heat => prefs.heatmap_axis = (*index).min(2) as u8,
            _ => return false,
        }
        self.interface.set_graph(prefs);
        if let (Some(hint), Changed::Value(v)) = (hint, changed) {
            root.edit_widget(id, |mut w| Slider::set(&mut w.downcast(), *v, &hint));
        }
        if id == created {
            self.refresh_created_hint(root);
        }
        self.push_display(root);
        false
    }
}
