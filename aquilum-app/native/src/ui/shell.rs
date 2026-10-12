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

pub struct Chrome {
    pub toggle_sidebar: WidgetId,
    pub settings: WidgetId,
    pub new_tab: WidgetId,
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
    pub note_page: Handle<Slot>,
    pub rail_settings: WidgetId,
    pub home_slot: Handle<Slot>,
    pub home_button: Option<WidgetId>,
    pub graph_page: Handle<Slot>,
    pub graph_button: Option<WidgetId>,
    pub focus_hidden: Vec<Handle<Visible>>,
    pub focus_shown: Handle<Visible>,
}

pub struct RootParts<'a> {
    pub workspace_name: &'a str,
    pub note: NewWidget<dyn Widget>,
    pub sidebar_open: bool,
    pub sidebar_from: bool,
    pub tabs: Vec<NewWidget<dyn Widget>>,
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
}

pub fn root(tree: NewWidget<impl Widget>, parts: RootParts<'_>) -> (NewWidget<impl Widget>, Chrome) {
    let RootParts { workspace_name, note, sidebar_open, sidebar_from, tabs, empty_tab, vault_open, vault_error, panel_open, panel_from, panel_mode, has_analysis, focus_mode, home, graph, graph_active } = parts;

    let rail = rail(sidebar_open, home, focus_mode);
    let sidebar = sidebar(tree, workspace_name, sidebar_open && !focus_mode, sidebar_from);

    let note_page = Slot::new(note);
    let note_handle = Handle::of(&note_page);
    let titlebar = titlebar(tabs, panel_open, panel_from, focus_mode);
    let mut focus_hidden = rail.focus_hidden;
    focus_hidden.push(titlebar.toggle_shown);
    let (panel, panel_handle, panel_parts) = backlinks::panel(panel_mode, has_analysis, panel_open, panel_from);
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
    let main = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(titlebar.widget)
        .with(pages, 1.0);

    let window = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(rail.widget)
        .with_fixed(sidebar.widget)
        .with(NewWidget::new(main).with_class(style::CANVAS), 1.0)
        .with_fixed(panel);
    let chrome = Chrome {
        toggle_sidebar: rail.toggle_sidebar,
        settings: sidebar.settings,
        new_tab: titlebar.new_tab,
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
        note_page: note_handle,
        rail_settings: rail.settings,
        home_slot: rail.home_slot,
        home_button: rail.home_button,
        graph_page: graph_handle,
        graph_button: rail.graph_button,
        focus_hidden,
        focus_shown: rail.focus_shown,
    };
    (NewWidget::new(window), chrome)
}
