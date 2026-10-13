use masonry::core::{NewWidget, PropertySet, Widget, WidgetId};
use masonry::layout::Length;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{Gap, Padding};
use masonry::widgets::{Flex, SizedBox};

use super::column::{Caption, EditorColumn};
use super::metadata::metadata_widget;
use super::title::TitleField;
use crate::i18n::t;
use crate::ui::editor::TextEditor;
use crate::ui::handle::Handle;
use crate::ui::icons;
use crate::ui::scroll::ScrollArea;
use crate::ui::style;
use crate::ui::tokens::size;
use crate::ui::widgets::{ButtonSize, IconButton, Slot, TextButton, Variant, Visible};

pub struct NoteView<'a> {
    pub file_name: &'a str,
    pub title: &'a str,
    pub body: &'a str,
    pub metadata: Option<bool>,
    pub can_back: bool,
    pub can_forward: bool,
    pub focus_mode: bool,
    pub wide: bool,
    pub hero: Option<crate::ui::book::HeroView>,
    pub resolver: Option<std::sync::Arc<dyn crate::ui::editor::resolve::Resolver>>,
    pub smart: crate::ui::editor::Smart,
}

pub struct NoteIds {
    pub back: WidgetId,
    pub forward: WidgetId,
    pub focus: WidgetId,
    pub focus_button: Handle<TextButton>,
    pub focus_hidden: Vec<Handle<Visible>>,
    pub more: WidgetId,
    pub error_slot: Handle<Slot>,
    pub name: Handle<Caption>,
    pub title_box: Handle<TitleField>,
    pub title: Handle<TextEditor>,
    pub metadata: Handle<Slot>,
    pub body: Handle<crate::ui::editor::TextEditor>,
    pub hero: Option<(Handle<crate::ui::book::Hero>, crate::ui::book::HeroIds)>,
    pub hero_slot: Handle<Slot>,
    pub spacer: Handle<SizedBox>,
    pub column: Handle<EditorColumn>,
    pub scroll: Handle<ScrollArea<Flex>>,
    pub find: Handle<Slot>,
}

pub struct HeroParts {
    pub widget: NewWidget<dyn Widget>,
    pub hero: Option<(Handle<crate::ui::book::Hero>, crate::ui::book::HeroIds)>,
    pub top: f64,
    pub inset: f64,
}

pub fn hero_parts(view: Option<crate::ui::book::HeroView>) -> HeroParts {
    match view {
        Some(view) => {
            let (hero, ids) = crate::ui::book::hero(view);
            let handle = Handle::of(&hero);
            HeroParts { widget: hero.erased(), hero: Some((handle, ids)), top: size::SPACE_24, inset: size::EDITOR_PADDING_X * 2.0 }
        }
        None => HeroParts {
            widget: NewWidget::new(SizedBox::empty().height(Length::ZERO)).erased(),
            hero: None,
            top: size::EDITOR_PADDING_Y,
            inset: 0.0,
        },
    }
}

pub fn page(view: NoteView<'_>) -> (NewWidget<Flex>, NoteIds) {
    let back = NewWidget::new(IconButton::new(icons::ARROW_LEFT, t("editor.back"))).disabled(!view.can_back);
    let forward = NewWidget::new(IconButton::new(icons::ARROW_RIGHT, t("editor.forward"))).disabled(!view.can_forward);
    let (back_id, forward_id) = (back.id(), forward.id());
    let focus = NewWidget::new(TextButton::new(t("editor.focusMode"), Variant::Ghost).size(ButtonSize::Xs).pressed(view.focus_mode));
    let focus_id = focus.id();
    let focus_button = Handle::of(&focus);
    let more = NewWidget::new(IconButton::new(icons::MORE_HORIZONTAL, t("editor.noteActions")));
    let more_id = more.id();
    let name = NewWidget::new(Caption::new(view.file_name));
    let name_handle = Handle::of(&name);
    let name = Visible::new(name, !view.focus_mode);
    let left = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(back).with_fixed(forward);
    let left = Visible::new(NewWidget::new(left).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_4)))), !view.focus_mode);
    let focus_hidden = vec![Handle::of(&left), Handle::of(&name)];
    let right = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(focus).with_fixed(more);
    let right = NewWidget::new(right).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_4))));
    let toolbar = Flex::row().main_axis_alignment(MainAxisAlignment::SpaceBetween).cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(left);
    let toolbar = toolbar.with(name, 1.0).with_fixed(right);
    let toolbar = NewWidget::new(SizedBox::new(NewWidget::new(toolbar)).height(Length::px(size::SIZE_40)))
        .with_props(PropertySet::new().with(Padding::all(Length::px(size::SPACE_4))).with(Gap::new(Length::px(size::SPACE_8))))
        .with_class(style::EDITOR);

    let error_slot = Slot::fit(NewWidget::new(SizedBox::empty()).erased());
    let error_slot_handle = Handle::of(&error_slot);

    let title_area = NewWidget::new(TextEditor::single_line(view.title).styled(style::editor_title)).with_class(style::EDITOR_TEXT);
    let title = Handle::of(&title_area);
    let title_field = TitleField::new(title_area, view.title.is_empty());
    let title_box = Handle::of(&title_field);

    let metadata = Slot::fit(metadata_widget(view.metadata));
    let metadata_handle = Handle::of(&metadata);

    let body_area = crate::ui::editor::TextEditor::new(view.body)
        .styled(style::editor_text)
        .markdown()
        .report_edits()
        .with_resolver(view.resolver.clone())
        .with_smart(view.smart);
    let body_area = NewWidget::new(body_area).with_class(style::EDITOR_TEXT);
    let body = Handle::of(&body_area);

    let content = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(error_slot)
        .with_fixed(title_field)
        .with_fixed(metadata)
        .with_fixed(body_area);
    let parts = hero_parts(view.hero);
    let hero = parts.hero;
    let hero_slot = Slot::fit(parts.widget);
    let hero_slot_handle = Handle::of(&hero_slot);
    let spacer = NewWidget::new(SizedBox::empty().height(Length::px(parts.top)));
    let spacer_handle = Handle::of(&spacer);
    let column = EditorColumn::new(NewWidget::new(content), parts.inset, view.wide);
    let column_handle = Handle::of(&column);
    let wrapper = NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(spacer).with_fixed(column)).with_props(
        PropertySet::new().with(Padding {
            top: Length::ZERO,
            bottom: Length::px(size::EDITOR_PADDING_Y + size::EDITOR_SCROLL_PAST_END),
            left: Length::px(size::EDITOR_PADDING_X),
            right: Length::px(size::EDITOR_PADDING_X),
        }),
    );
    let scroller = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(hero_slot).with_fixed(wrapper);
    let scroller = NewWidget::new(scroller).with_class(style::EDITOR);
    let scroll = NewWidget::new(ScrollArea::new(scroller));
    let scroll_handle = Handle::of(&scroll);
    let find = Slot::fit(NewWidget::new(SizedBox::empty()).erased());
    let find_handle = Handle::of(&find);
    let page = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(toolbar)
        .with_fixed(find)
        .with(scroll, 1.0);
    let page = NewWidget::new(page).with_class(style::EDITOR);
    let ids = NoteIds {
        back: back_id,
        forward: forward_id,
        focus: focus_id,
        focus_button,
        focus_hidden,
        more: more_id,
        error_slot: error_slot_handle,
        name: name_handle,
        title_box,
        title,
        metadata: metadata_handle,
        body,
        hero,
        hero_slot: hero_slot_handle,
        spacer: spacer_handle,
        column: column_handle,
        scroll: scroll_handle,
        find: find_handle,
    };
    (page, ids)
}
