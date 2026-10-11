use masonry::core::{DefaultProperties, PropertySet, PropertyStack, Selector, StyleProperty};
use masonry::parley::{FontWeight, LineHeight};
use masonry::peniko::Color;
use masonry::properties::{Background, BorderColor, CaretColor, ContentColor, SelectionColor};
use masonry::widgets::{Flex, SizedBox};

use super::editor::TextEditor;
use super::theme;
use super::tokens::{Theme, number, size};

pub fn ui_text(line: f32) -> Vec<StyleProperty> {
    let font = theme::ui_font_settings();
    vec![StyleProperty::FontFamily(theme::ui_font(&font.family)), StyleProperty::FontSize(font.size), StyleProperty::LineHeight(LineHeight::Absolute(line))]
}

pub fn editor_text() -> Vec<StyleProperty> {
    let font = theme::editor_font_settings();
    vec![
        StyleProperty::FontFamily(theme::editor_font(&font.family)),
        StyleProperty::FontSize(font.size),
        StyleProperty::FontWeight(FontWeight::new(font.weight)),
        StyleProperty::LineHeight(LineHeight::FontSizeRelative(font.line_height)),
    ]
}

pub fn editor_title() -> Vec<StyleProperty> {
    let font = theme::editor_font_settings();
    vec![
        StyleProperty::FontFamily(theme::editor_font(&font.family)),
        StyleProperty::FontSize(size::FONT_SIZE_24 as f32),
        StyleProperty::FontWeight(FontWeight::new(number::EDITOR_HEADING_WEIGHT as f32)),
        StyleProperty::LineHeight(LineHeight::FontSizeRelative(font.line_height)),
    ]
}

pub fn tab_text() -> Vec<StyleProperty> {
    let font = theme::ui_font_settings();
    vec![
        StyleProperty::FontFamily(theme::ui_font(&font.family)),
        StyleProperty::FontSize(size::TABS_FONT_SIZE as f32),
        StyleProperty::LineHeight(LineHeight::Absolute(size::TAB_CONTENT_HEIGHT as f32)),
    ]
}

pub const EDITOR: &str = "editor";
pub const SIDEBAR: &str = "sidebar";
pub const TABS: &str = "tabs";
pub const CANVAS: &str = "canvas";
pub const ACTIVE_ITEM: &str = "active-item";
pub const DANGER: &str = "danger";
pub const CARD: &str = "card";
pub const BLOCK: &str = "block";
pub const GRAPH_PANEL: &str = "graph-panel";

pub const FIELD: &str = "field";
pub const SIDEBAR_FIELD: &str = "sidebar-field";
pub const TAB_FIELD: &str = "tab-field";
pub const EDITOR_TEXT: &str = "editor-text";

fn surfaces(t: &Theme) -> [(&'static str, Color, Option<Color>); 9] {
    [
        (EDITOR, t.editor_bg, None),
        (SIDEBAR, t.sidebar_bg, None),
        (TABS, t.tabs_bg, None),
        (CANVAS, t.bg_canvas, None),
        (ACTIVE_ITEM, t.sidebar_item_active_bg, None),
        (DANGER, t.bg_danger_subtle, None),
        (CARD, t.bg_surface_subtle, None),
        (BLOCK, t.block_bg, Some(t.block_border)),
        (GRAPH_PANEL, t.graph_settings_bg, Some(t.sidebar_border)),
    ]
}

fn fields(t: &Theme) -> [(&'static str, Color, Color); 4] {
    [
        (FIELD, t.text_primary, t.text_primary),
        (SIDEBAR_FIELD, t.sidebar_text, t.text_primary),
        (TAB_FIELD, t.tab_text_active, t.text_primary),
        (EDITOR_TEXT, t.editor_text, t.editor_text),
    ]
}

pub fn install(properties: &mut DefaultProperties, t: &Theme) {
    let mut boxes = PropertyStack::new();
    for (class, background, border) in surfaces(t) {
        let mut set = PropertySet::new().with(Background::Color(background));
        if let Some(border) = border {
            set = set.with(BorderColor { color: border });
        }
        boxes.push_layer(Selector::classes(&[class]), set);
    }
    properties.insert_stack::<Flex>(boxes.clone());
    properties.insert_stack::<SizedBox>(boxes);
    let mut editors = PropertyStack::new();
    for (class, text, caret) in fields(t) {
        let set = PropertySet::new().with(ContentColor::new(text)).with(CaretColor { color: caret }).with(SelectionColor { color: t.selection_bg });
        editors.push_layer(Selector::classes(&[class]), set);
    }
    properties.insert_stack::<TextEditor>(editors);
}
