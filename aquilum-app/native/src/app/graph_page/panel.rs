use masonry::core::{NewWidget, PropertySet, Widget};
use masonry::layout::Length;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{BorderWidth, CornerRadius, Gap, Padding};
use masonry::widgets::{Flex, SizedBox};

use super::{App, Controls, depth_hint};
use crate::graph;
use crate::i18n::t;
use crate::ui::graph::{GraphView, PANEL_PADDING};
use crate::ui::handle::Handle;
use crate::ui::icons::{self, Icon};
use crate::ui::settings::controls::{Segmented, Slider, Switch};
use crate::ui::text::label;
use crate::ui::tokens::{number, size};
use crate::ui::widgets::{IconButton, Leading};

pub(super) fn panel_toggle(open: bool) -> (Icon, &'static str) {
    if open { (icons::X, "common.close") } else { (icons::SETTINGS, "graph.settings") }
}

impl App {
    pub(in crate::app) fn graph_widget(&mut self) -> Option<NewWidget<dyn Widget>> {
        if !self.graph_active() {
            self.graph.view = None;
            self.graph.controls = None;
            return None;
        }
        let prefs = self.interface.graph();
        let title = label(&t("graph.title"), |t| t.text_primary, Some(number::FONT_WEIGHT_UI_STRONG as f32), Leading::Times(number::FONT_LINE_HEIGHT_NORMAL as f32));
        let reset = NewWidget::new(IconButton::new(icons::ROTATE_CCW, t("graph.reset")));
        let (icon, hint) = panel_toggle(prefs.panel);
        let button = NewWidget::new(IconButton::new(icon, t(hint)));
        let (reset_id, toggle_id) = (reset.id(), button.id());
        let head = Flex::row().main_axis_alignment(MainAxisAlignment::SpaceBetween).cross_axis_alignment(CrossAxisAlignment::Center);
        let head = head.with(title, 1.0).with_fixed(reset).with_fixed(NewWidget::new(SizedBox::empty().width(Length::px(size::SIZE_32))));
        let head = NewWidget::new(head).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_4))));
        let node_size = Slider::new(&t("graph.nodeSize"), prefs.node_size, 0.4, 2.5, 0.05, &format!("{:.2}×", prefs.node_size));
        let spread = Slider::new(&t("graph.spread"), prefs.spread, graph::MIN_SPREAD, 2.5, 0.05, &format!("{:.2}×", prefs.spread));
        let depth = Slider::new(&t("graph.depth"), prefs.highlight_depth as f64, 1.0, 3.0, 1.0, &depth_hint(prefs.highlight_depth));
        let created = Slider::new(&t("graph.createdAfter"), self.graph.share, 0.0, 1.0, 0.005, &self.created_hint());
        let labels = Switch::new(prefs.labels, &t("graph.labels"));
        let toggle = Flex::row()
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .with(label(&t("graph.labels"), |t| t.text_secondary, None, Leading::Times(number::FONT_LINE_HEIGHT_NORMAL as f32)), 1.0);
        let labels_id = labels.id();
        let toggle = toggle.with_fixed(labels);
        let options = vec![t("graph.heatNone"), t("graph.heatModified"), t("graph.heatCreated")];
        let heat = Segmented::new(options, usize::from(prefs.heatmap_axis).min(2));
        let heat_id = heat.id();
        let choice = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(label(&t("graph.heatmap"), |t| t.text_secondary, None, Leading::Times(number::FONT_LINE_HEIGHT_NORMAL as f32)))
            .with_fixed(heat);
        let choice = NewWidget::new(choice).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_8))));
        let controls = Controls {
            reset: reset_id,
            toggle: toggle_id,
            node_size: node_size.id(),
            spread: spread.id(),
            depth: depth.id(),
            created: created.id(),
            labels: labels_id,
            heat: heat_id,
        };
        let panel = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(head)
            .with_fixed(node_size)
            .with_fixed(spread)
            .with_fixed(depth)
            .with_fixed(created)
            .with_fixed(NewWidget::new(toggle))
            .with_fixed(choice);
        let panel = NewWidget::new(panel).with_class(crate::ui::style::GRAPH_PANEL).with_props(
            PropertySet::new()
                .with(BorderWidth { width: Length::px(size::BORDER_1) })
                .with(CornerRadius { radius: Length::px(size::RADIUS_2XL) })
                .with(Padding::all(Length::px(PANEL_PADDING)))
                .with(Gap::new(Length::px(size::SPACE_16))),
        );
        let camera = self.session.as_ref().and_then(|s| s.camera);
        let view = GraphView::new(self.graph.data.clone(), self.graph_status(), self.graph_display(), camera, Some([panel.erased(), button.erased()]));
        self.graph.view = Some(Handle::of(&view));
        self.graph.controls = Some(controls);
        Some(view.erased())
    }
}
