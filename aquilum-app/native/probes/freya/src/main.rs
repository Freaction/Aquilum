//! Прототип на Freya 0.5 для сравнения с окном на vello_cpu (`aquilum-native`): тот же макет —
//! боковая панель, вкладки, колонка текста с прокруткой.
//!
//! Рендерер выбирается переменной `FREYA_RENDERER`: `software`, `opengl` (по умолчанию на Windows)
//! или `vulkan`.
//!
//!   freya-probe.exe [--size ШxВ] [--file заметка.md] [--static]
//!
//! По умолчанию текст — редактируемый многострочный `Input`; `--static` — статичный абзац (шаг 1).
//!
//! `--size` — логические пиксели (Freya сама умножает на масштаб): 1646x981 при 175% = 2881x1717.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use freya::prelude::*;

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

const SAMPLE_TEXT: &str = include_str!("../../../src/sample.md");

const BACKGROUND: (u8, u8, u8) = (0x1f, 0x1f, 0x22);
const SIDEBAR: (u8, u8, u8) = (0x18, 0x18, 0x1b);
const SELECTED_ROW: (u8, u8, u8) = (0x2c, 0x2c, 0x33);
const BORDER: (u8, u8, u8) = (0x2e, 0x2e, 0x33);
const TEXT: (u8, u8, u8) = (0xdc, 0xdc, 0xe0);
const TEXT_MUTED: (u8, u8, u8) = (0x9a, 0x9a, 0xa3);
const HEADING: (u8, u8, u8) = (0xf2, 0xf2, 0xf5);

struct Probe {
    text: String,
    /// Редактируемый многострочный `Input` вместо статичного абзаца (шаг 2: заглушка редактора).
    editable: bool,
}

impl App for Probe {
    fn render(&self) -> impl IntoElement {
        let tree = TREE.iter().enumerate().map(|(i, name)| {
            let color = if name.starts_with(' ') { TEXT } else { TEXT_MUTED };
            rect()
                .width(Size::fill())
                .height(Size::px(28.))
                .padding((0., 12.))
                .main_align(Alignment::Center)
                .corner_radius(4.)
                .background(if i == 3 { SELECTED_ROW } else { SIDEBAR })
                .child(label().text(*name).font_size(14.).color(color))
                .into()
        });
        let sidebar = rect()
            .width(Size::px(260.))
            .height(Size::fill())
            .background(SIDEBAR)
            .border(Border::new().fill(BORDER).width(BorderWidth { right: 1., ..Default::default() }))
            .padding(8.)
            .children(tree.collect::<Vec<Element>>());

        let tabs = rect()
            .horizontal()
            .width(Size::fill())
            .height(Size::px(40.))
            .background(SIDEBAR)
            .border(Border::new().fill(BORDER).width(BorderWidth { bottom: 1., ..Default::default() }))
            .child(
                rect()
                    .width(Size::px(220.))
                    .height(Size::fill())
                    .padding((0., 16.))
                    .main_align(Alignment::Center)
                    .background(BACKGROUND)
                    .child(label().text("Уход от Tauri.md").font_size(13.).color(TEXT)),
            );

        let spans = self.text.split_inclusive('\n').map(|line| {
            let span = Span::new(line.to_owned());
            if line.starts_with("# ") {
                span.font_size(30.).font_weight(FontWeight::BOLD).color(HEADING)
            } else if line.starts_with("## ") {
                span.font_size(22.).font_weight(FontWeight::SEMI_BOLD).color(HEADING)
            } else {
                span.font_size(16.).color(TEXT)
            }
        });
        let text = use_state(|| self.text.clone());
        let body: Element = if self.editable {
            Input::new(text).multiline(true).width(Size::px(760.)).into()
        } else {
            paragraph()
                .width(Size::px(760.))
                .line_height(1.6)
                .spans_iter(spans.collect::<Vec<_>>().into_iter())
                .into()
        };
        let content = ScrollView::new().child(
            rect().width(Size::fill()).cross_align(Alignment::Center).padding((32., 32.)).child(body),
        );

        let editor = rect().width(Size::flex(1.)).height(Size::fill()).child(tabs).child(content);

        rect()
            .horizontal()
            .content(Content::Flex)
            .width(Size::fill())
            .height(Size::fill())
            .background(BACKGROUND)
            .child(sidebar)
            .child(editor)
    }
}

fn main() {
    let mut size = (1646.0, 981.0);
    let mut text = SAMPLE_TEXT.to_owned();
    let mut editable = true;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        if flag == "--static" {
            editable = false;
            continue;
        }
        let value = args.next().unwrap_or_else(|| panic!("{flag}: нет значения"));
        match flag.as_str() {
            "--size" => {
                let (w, h) = value.split_once('x').expect("--size: нужно ШxВ");
                size = (w.parse().expect("--size: ширина"), h.parse().expect("--size: высота"));
            }
            "--file" => text = std::fs::read_to_string(&value).expect("--file: не читается"),
            // Аргументы скрипта замера, которые относятся только к aquilum-native.
            "--threads" | "--bench-frames" | "--metrics" => {}
            other => panic!("неизвестный аргумент {other}"),
        }
    }
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new_app(Probe { text, editable })
                .with_title("Aquilum (Freya probe)")
                .with_size(size.0, size.1)
                .with_background(BACKGROUND),
        ),
    )
}
