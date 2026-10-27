use std::path::PathBuf;

use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Point;

use super::{App, depth_hint};
use crate::app::Popup;
use crate::i18n::t;
use crate::interface::GraphPrefs;
use crate::ui::dialog::ConfirmDialog;
use crate::ui::graph::GraphAction;
use crate::ui::settings::controls::{Changed, Segmented, Slider, Switch};
use crate::ui::widgets::{IconButton, Pressed};

impl App {
    pub(in crate::app) fn is_graph_widget(&self, id: WidgetId) -> bool {
        let Some(c) = &self.graph.controls else { return false };
        self.graph.view.is_some_and(|v| v.id() == id) || [c.reset, c.toggle, c.node_size, c.spread, c.depth, c.created, c.labels, c.heat].contains(&id)
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
        let (reset, toggle, node_size, spread, depth, created, labels, heat) = (c.reset, c.toggle, c.node_size, c.spread, c.depth, c.created, c.labels, c.heat);
        if id == reset && action.downcast_ref::<Pressed>().is_some() {
            self.confirm_reset_graph(root);
            return false;
        }
        if id == toggle && action.downcast_ref::<Pressed>().is_some() {
            let prefs = self.interface.graph();
            self.interface.set_graph(GraphPrefs { panel: !prefs.panel, ..prefs });
            let (icon, label) = super::panel::panel_toggle(!prefs.panel);
            root.edit_widget(toggle, |mut w| {
                let mut button = w.downcast::<IconButton>();
                IconButton::set_icon(&mut button, icon);
                IconButton::set_label(&mut button, &t(label));
            });
            self.push_display(root);
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

    pub(in crate::app) fn confirm_reset_graph(&mut self, root: &mut RenderRoot) {
        let (layer, dialog) = ConfirmDialog::new(
            &t("graph.resetTitle"),
            &t("graph.resetDescription"),
            &t("common.cancel"),
            &t("graph.resetConfirm"),
        );
        root.add_layer(layer, Point::ORIGIN);
        root.focus_on(Some(dialog.layer));
        self.popup = Some(Popup::ResetGraph(dialog));
    }

    pub(in crate::app) fn reset_graph_settings(&mut self, root: &mut RenderRoot) {
        let prefs = GraphPrefs { panel: self.interface.graph().panel, ..GraphPrefs::default() };
        self.interface.set_graph(prefs);
        self.graph.share = 0.0;
        if let Some(c) = &self.graph.controls {
            root.edit_widget(c.node_size, |mut w| Slider::set(&mut w.downcast(), prefs.node_size, &format!("{:.2}×", prefs.node_size)));
            root.edit_widget(c.spread, |mut w| Slider::set(&mut w.downcast(), prefs.spread, &format!("{:.2}×", prefs.spread)));
            root.edit_widget(c.depth, |mut w| Slider::set(&mut w.downcast(), prefs.highlight_depth as f64, &depth_hint(prefs.highlight_depth)));
            root.edit_widget(c.created, |mut w| Slider::set(&mut w.downcast(), 0.0, &t("graph.all")));
            root.edit_widget(c.labels, |mut w| Switch::set(&mut w.downcast(), prefs.labels));
            root.edit_widget(c.heat, |mut w| Segmented::set(&mut w.downcast(), usize::from(prefs.heatmap_axis).min(2)));
        }
        self.push_display(root);
    }
}
