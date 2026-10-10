//! Проба Masonry (шаг 2 этапа 2): ядро виджетов `masonry_core` на собственном хосте —
//! winit 0.31 + vello_cpu + softbuffer, без `masonry_winit` (у него только GPU-бэкенды и winit 0.30).
//!
//! Макет тот же, что в `aquilum-native`: боковая панель и колонка текста, только текст здесь —
//! редактируемый `TextArea` как заглушка редактора. Проверяется: встаёт ли Masonry на наш хост без
//! форка, работают ли фокус, ввод, IME, буфер обмена, и сколько это стоит по памяти.
//!
//!   masonry-probe.exe [--size ШxВ] [--file заметка.md] [--metrics файл.json] [--trace] [--focus]

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod host;

use std::path::PathBuf;

use masonry::core::{NewWidget, PropertySet, Widget, WidgetTag};
use masonry::layout::Length;
use masonry::parley::style::StyleProperty;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Background, Padding};
use masonry::peniko::color::AlphaColor;
use masonry::widgets::{Flex, Label, Portal, SizedBox, TextArea};
use winit::dpi::PhysicalSize;

const TREE: &[&str] = &[
    "Входящие",
    "Проекты",
    "    Aquilum.md",
    "    Уход от Tauri.md",
    "    Документ в Rust.md",
    "Книги",
    "Ежедневник",
    "    2026-10-07.md",
    "    2026-10-08.md",
    "Шаблоны",
    "README.md",
];

pub const EDITOR_TAG: WidgetTag<TextArea<true>> = WidgetTag::named("editor");

const SAMPLE_TEXT: &str = include_str!("../../../src/sample.md");

pub struct Args {
    pub size: PhysicalSize<u32>,
    pub text: String,
    pub metrics: Option<PathBuf>,
    pub trace: bool,
    /// Сразу поставить фокус в текст: замер с мигающей кареткой без участия мыши.
    pub focus: bool,
}

fn parse_args() -> Args {
    let mut args = Args {
        size: PhysicalSize::new(2881, 1717),
        text: SAMPLE_TEXT.to_owned(),
        metrics: None,
        trace: false,
        focus: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().unwrap_or_else(|| panic!("{flag}: нет значения"));
        match flag.as_str() {
            "--size" => {
                let v = value();
                let (w, h) = v.split_once('x').expect("--size: нужно ШxВ");
                args.size = PhysicalSize::new(w.parse().expect("ширина"), h.parse().expect("высота"));
            }
            "--file" => args.text = std::fs::read_to_string(value()).expect("--file: не читается"),
            "--metrics" => args.metrics = Some(value().into()),
            "--trace" => args.trace = true,
            "--focus" => args.focus = true,
            // Аргументы скрипта замера, которые относятся только к aquilum-native.
            "--threads" | "--bench-frames" => {
                value();
            }
            other => panic!("неизвестный аргумент {other}"),
        }
    }
    args
}

pub fn widget_tree(text: &str) -> NewWidget<impl Widget> {
    let sidebar_bg = AlphaColor::from_rgb8(0x18, 0x18, 0x1b);
    let mut tree = Flex::column().cross_axis_alignment(CrossAxisAlignment::Start);
    for name in TREE {
        tree = tree.with_fixed(Label::new(*name).with_style(StyleProperty::FontSize(14.0)).prepare());
    }
    let sidebar = NewWidget::new(SizedBox::new(NewWidget::new(tree)).width(Length::px(260.0)))
        .with_props(PropertySet::new().with(Background::Color(sidebar_bg)).with(Padding::all(Length::px(12.0))));

    let editor = TextArea::new_editable(text)
        .with_style(StyleProperty::FontSize(16.0))
        .with_style(StyleProperty::LineHeight(masonry::parley::LineHeight::FontSizeRelative(1.6)));
    let column = SizedBox::new(NewWidget::new(editor).with_tag(EDITOR_TAG)).width(Length::px(760.0));
    let content = Portal::new(
        NewWidget::new(Flex::column().with_fixed(NewWidget::new(column)))
            .with_props(PropertySet::new().with(Padding::all(Length::px(32.0)))),
    )
    .prepare();

    NewWidget::new(Flex::row().with_fixed(sidebar).with(content, 1.0))
}

fn main() {
    let args = parse_args();
    host::run(args);
}
