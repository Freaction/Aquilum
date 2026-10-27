mod scrim;
mod stage;

use masonry::app::RenderRoot;
use masonry::core::{NewWidget, Widget, WidgetId};
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::{Flex, IndexedStack, SizedBox};

use super::handle::Handle;
use super::header::titlebar;
use super::new_tab::new_tab_view;
use super::rail::rail;
use super::sidebar::sidebar;
use super::titlebar::TabStrip;
use super::widgets::{Collapse, Slot, Visible};
use super::{backlinks, style, workspaces};
use crate::interface::Screen;
use stage::Stage;

pub struct Chrome {
    pub toggle_sidebar: WidgetId,
    pub settings: WidgetId,
    pub new_tab: WidgetId,
    pub tab_list: WidgetId,
    pub tab_strip: Handle<TabStrip>,
    pub panel_toggle: WidgetId,
    pub controls_inset: Handle<Collapse>,
    pub panel: Handle<Collapse>,
    pub panel_parts: backlinks::Panel,
    pub pages: Handle<IndexedStack>,
    pub version_page: Handle<Slot>,
    pub create_note: WidgetId,
    pub open_file: WidgetId,
    pub close_tab: WidgetId,
    pub workspaces: WidgetId,
    pub open_vault: WidgetId,
    pub sidebar: Handle<Collapse>,
    pub sidebar_sash: WidgetId,
    pub note_page: Handle<Slot>,
    pub rail: Handle<Collapse>,
    pub rail_settings: WidgetId,
    pub search_button: WidgetId,
    pub home_slot: Handle<Slot>,
    pub home_button: Option<WidgetId>,
    pub graph_page: Handle<Slot>,
    pub notice: Handle<Slot>,
    pub graph_button: Option<WidgetId>,
    pub focus_hidden: Vec<Handle<Visible>>,
    pub focus_shown: Handle<Visible>,
    pub drawer: WidgetId,
    pub scrim: WidgetId,
    drawer_shown: Handle<Visible>,
    stage: Handle<Stage>,
}

impl Chrome {
    pub fn set_screen(&self, root: &mut RenderRoot, screen: Screen) {
        self.stage.edit(root, |mut stage| Stage::set_screen(&mut stage, screen));
        self.drawer_shown.edit(root, |mut shown| Visible::set_shown(&mut shown, screen.compact));
        self.tab_strip.edit(root, |mut strip| TabStrip::set_compact(&mut strip, screen.compact));
    }
}

pub struct RootParts<'a> {
    pub workspace_name: &'a str,
    pub note: NewWidget<dyn Widget>,
    pub sidebar_open: bool,
    pub sidebar_from: bool,
    pub sidebar_width: f64,
    pub panel_width: f64,
    pub tabs: Vec<NewWidget<dyn Widget>>,
    pub active_tab: usize,
    pub empty_tab: bool,
    pub vault_open: bool,
    pub vault_error: Option<&'a str>,
    pub panel_open: bool,
    pub panel_from: bool,
    pub panel_mode: crate::links::Mode,
    pub has_analysis: bool,
    pub focus_mode: bool,
    pub home: bool,
    pub graph: Option<NewWidget<dyn Widget>>,
    pub graph_active: bool,
    pub screen: Screen,
}

pub fn root(tree: NewWidget<impl Widget>, parts: RootParts<'_>) -> (NewWidget<impl Widget>, Chrome) {
    let RootParts { workspace_name, note, sidebar_open, sidebar_from, sidebar_width, panel_width, tabs, active_tab, empty_tab, vault_open, vault_error, panel_open, panel_from, panel_mode, has_analysis, focus_mode, home, graph, graph_active, screen } = parts;

    let rail = rail(sidebar_open, home, focus_mode);
    let sidebar = sidebar(tree, workspace_name, sidebar_width, sidebar_open && !focus_mode, sidebar_from);

    let note_page = Slot::new(note);
    let note_handle = Handle::of(&note_page);
    let titlebar = titlebar(tabs, active_tab, panel_open, panel_from, focus_mode, screen.compact);
    let focus_hidden = vec![titlebar.toggle_shown];
    let (panel, panel_handle, panel_parts) = backlinks::panel(panel_mode, has_analysis, panel_width, panel_open, panel_from);
    let (empty, create_note, open_file, close_tab) = new_tab_view();
    let version_page = Slot::new(NewWidget::new(Flex::column()).erased());
    let version_handle = Handle::of(&version_page);
    let (no_vault, open_vault) = workspaces::no_vault(vault_error);
    let active = if !vault_open {
        3
    } else if graph_active {
        4
    } else {
        usize::from(empty_tab)
    };
    let graph_page = Slot::new(graph.unwrap_or_else(|| NewWidget::new(SizedBox::empty()).erased()));
    let graph_handle = Handle::of(&graph_page);
    let pages = IndexedStack::new().with(note_page).with(empty).with(version_page).with(no_vault).with(graph_page).with_active_child(active);
    let pages = NewWidget::new(pages);
    let pages_handle = Handle::of(&pages);
    let notice = Slot::fit(NewWidget::new(SizedBox::empty()).erased());
    let notice_handle = Handle::of(&notice);
    let main = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(titlebar.widget)
        .with(pages, 1.0)
        .with_fixed(notice);

    let main = NewWidget::new(main).with_class(style::CANVAS).erased();
    let stage = Stage::new(main, [rail.widget.erased(), sidebar.widget.erased(), panel.erased()], screen);
    let scrim = stage.widget.scrim();
    let stage_handle = Handle::of(&stage);
    let chrome = Chrome {
        toggle_sidebar: rail.toggle_sidebar,
        settings: sidebar.settings,
        new_tab: titlebar.new_tab,
        tab_list: titlebar.tab_list,
        tab_strip: titlebar.strip,
        panel_toggle: titlebar.toggle,
        controls_inset: titlebar.inset,
        panel: panel_handle,
        panel_parts,
        pages: pages_handle,
        version_page: version_handle,
        create_note,
        open_file,
        close_tab,
        workspaces: sidebar.workspaces,
        open_vault,
        sidebar: sidebar.handle,
        sidebar_sash: sidebar.sash,
        note_page: note_handle,
        rail: rail.handle,
        rail_settings: titlebar.settings,
        search_button: rail.search_button,
        home_slot: rail.home_slot,
        home_button: rail.home_button,
        graph_page: graph_handle,
        notice: notice_handle,
        graph_button: rail.graph_button,
        focus_hidden,
        focus_shown: titlebar.settings_shown,
        drawer: titlebar.drawer,
        scrim,
        drawer_shown: titlebar.drawer_shown,
        stage: stage_handle,
    };
    (NewWidget::new(SizedBox::new(stage)).with_class(style::TABS), chrome)
}
