use masonry::core::StyleSet;
use masonry::parley::{FontContext, LayoutContext};

use super::Document;
use crate::ui::editor::highlight::Lang;
use crate::ui::editor::markdown::{Block, Callout, Fonts};

const CORPUS: &str = include_str!("../../../../testdata/markdown-test.md");

fn corpus() -> Document {
    let mut doc = Document::new(CORPUS);
    doc.set_markdown(Some(Fonts::new(16.0, true, true)));
    doc
}

fn at(doc: &Document, line: &str) -> usize {
    doc.paragraphs().position(|p| doc.slice(p.range()) == line).unwrap_or_else(|| panic!("нет строки «{line}»"))
}

fn block<'a>(doc: &'a Document, line: &str) -> &'a Block {
    doc.block(at(doc, line))
}

fn framed<'a>(doc: &'a Document, line: &str) -> (&'a Block, u8, Option<usize>, usize) {
    let frame = doc.frame(at(doc, line));
    (block(doc, line), frame.quote, frame.list.map(|nest| nest.level), frame.lead)
}

fn item_level(doc: &Document, line: &str) -> Option<usize> {
    match block(doc, line) {
        Block::Item(item) => Some(item.level()),
        _ => None,
    }
}

#[test]
fn frontmatter_is_metadata_and_the_body_starts_after_it() {
    let doc = corpus();
    let front = crate::frontmatter::end(CORPUS).expect("в тестовой заметке есть frontmatter");
    let meta = doc.paragraphs().take_while(|p| p.start < front).count();
    assert_eq!(meta, 13);
    let kinds: Vec<&Block> = (0..meta).map(|i| doc.block(i)).collect();
    assert_eq!((kinds[0], kinds[meta - 1]), (&Block::Rule, &Block::Rule), "обе черты блока свойств — линии");
    assert!(kinds[1..meta - 1].iter().all(|b| **b == Block::Meta));
    assert_eq!(block(&doc, "# Полный тест Markdown-редактора"), &Block::Heading { level: 1, marker: 2 });
}

#[test]
fn headings_atx_setext_and_non_headings() {
    let doc = corpus();
    assert_eq!(block(&doc, "###### Заголовок H6"), &Block::Heading { level: 6, marker: 7 });
    assert_eq!(block(&doc, "Заголовок H1 (Setext)"), &Block::Heading { level: 1, marker: 0 });
    assert_eq!(doc.block(at(&doc, "Заголовок H1 (Setext)") + 1), &Block::Underline);
    assert_eq!(block(&doc, "Заголовок H2 (Setext)"), &Block::Heading { level: 2, marker: 0 });
    assert_eq!(block(&doc, "####### Семь решёток — это НЕ заголовок"), &Block::Text);
    assert_eq!(block(&doc, "#Без пробела — это НЕ заголовок"), &Block::Text);
    assert_eq!(block(&doc, "# Заголовок с закрывающими решётками #"), &Block::Heading { level: 1, marker: 2 });
    assert_eq!(block(&doc, "# Заголовок без пустой строки перед ним"), &Block::Heading { level: 1, marker: 2 });
}

#[test]
fn property_fences_are_rules_and_the_lines_between_are_meta() {
    let mut doc = Document::new("---\nтег: один\nдата: 2026\n---\nтекст");
    doc.set_markdown(Some(Fonts::new(16.0, true, true)));
    let blocks: Vec<&Block> = (0..doc.line_count()).map(|i| doc.block(i)).collect();
    assert!(matches!(blocks[..], [Block::Rule, Block::Meta, Block::Meta, Block::Rule, _]), "{blocks:?}");
    assert!(!matches!(blocks[4], Block::Rule | Block::Meta));
}

#[test]
fn quotes_nest_and_callouts_keep_their_kind() {
    let doc = corpus();
    assert_eq!(framed(&doc, "> Простая цитата в одну строку."), (&Block::Text, 1, None, 2));
    assert_eq!(framed(&doc, "> > > Третий уровень"), (&Block::Text, 3, None, 6));
    assert!(matches!(block(&doc, "> [!quote] Цитата"), Block::Callout { kind: Callout::Quote, .. }));
    assert!(matches!(block(&doc, "> [!NOTE]"), Block::Callout { kind: Callout::Typed(_), .. }));
    assert!(matches!(block(&doc, "> [!note]+ Раскрытая по умолчанию"), Block::Callout { range, kind: Callout::Typed(_) } if range.end == 11));
    assert!(matches!(block(&doc, "> [!custom-type] Пользовательский тип"), Block::Callout { kind: Callout::Typed(_), .. }));
    assert_eq!(framed(&doc, "> Цитата"), (&Block::Text, 1, None, 2));
}

#[test]
fn lists_of_every_marker_nest_by_columns() {
    let doc = corpus();
    for line in ["- Пункт с дефисом", "* Пункт со звёздочкой", "+ Пункт с плюсом", "1. Первый", "1) Скобка вместо точки", "5. Начало с пяти"] {
        assert_eq!(item_level(&doc, line), Some(0), "{line}");
    }
    let ladder = ["- 1", "  - 2", "    - 3", "      - 4", "        - 5", "          - 6", "            - 7", "              - 8"];
    let levels: Vec<Option<usize>> = ladder.iter().map(|line| item_level(&doc, line)).collect();
    assert_eq!(levels, (0..8).map(Some).collect::<Vec<_>>());
    assert_eq!(item_level(&doc, "\t1. Вложенный нумерованный"), Some(1));
    assert_eq!(item_level(&doc, "\t\t- Маркированный внутри нумерованного"), Some(2));
    assert_eq!(item_level(&doc, "- Свободный список (с пустыми строками между пунктами)"), Some(0));
    assert_eq!(block(&doc, "1986\\. Год, а не список."), &Block::Text);
    assert_eq!(block(&doc, "\\- Не маркер списка."), &Block::Text);
}

#[test]
fn tasks_have_boxes_at_every_level() {
    let doc = corpus();
    let task = |line: &str| match block(&doc, line) {
        Block::Item(item) => item.task().map(|(_, done)| done),
        _ => None,
    };
    assert_eq!(task("- [ ] Невыполненная задача"), Some(false));
    assert_eq!(task("- [x] Выполненная задача"), Some(true));
    assert_eq!(task("- [X] Выполненная (заглавная X)"), Some(true));
    assert_eq!(task("\t- [ ] Вложенная задача"), Some(false));
    assert_eq!(task("\t- [x] Вложенная выполненная"), Some(true));
    assert_eq!(task("\t\t- [ ] Ещё глубже"), Some(false));
    assert_eq!(task("1. [ ] Нумерованная задача"), Some(false));
    assert_eq!(task("2. [x] Нумерованная выполненная"), Some(true));
    assert_eq!(item_level(&doc, "\t\t- [ ] Ещё глубже"), Some(2));
}

#[test]
fn list_items_keep_paragraphs_code_and_definitions() {
    let doc = corpus();
    assert_eq!(framed(&doc, "   Второй абзац внутри пункта (отступ 3 пробела)."), (&Block::Text, 0, Some(0), 3));
    assert!(matches!(block(&doc, "   ```bash"), Block::Code { first: true, fence: true, lang: Some(_), .. }));
    assert!(matches!(block(&doc, "   echo \"Код внутри пункта списка\""), Block::Code { fence: false, .. }));
    assert_eq!(block(&doc, "Термин 1"), &Block::Text);
    assert_eq!(block(&doc, ": Определение первого термина."), &Block::Definition { marker: 2 });
    assert_eq!(block(&doc, ": Второе определение."), &Block::Definition { marker: 2 });
}

#[test]
fn code_blocks_fences_and_indentation() {
    let doc = corpus();
    assert!(matches!(block(&doc, "    function indented() {"), Block::Code { first: true, fence: false, lang: None, .. }));
    assert!(matches!(block(&doc, "Простой текст"), Block::Code { fence: false, .. }));
    assert!(matches!(block(&doc, "~~~"), Block::Code { first: true, fence: true, .. }));
    let outer = at(&doc, "````markdown");
    assert!(matches!(doc.paragraph(outer).block, Block::Code { first: true, fence: true, .. }));
    assert!(matches!(doc.paragraph(outer + 1).block, Block::Code { first: false, fence: false, .. }), "```js внутри четырёх обратных кавычек не должно открывать блок");
    assert!(matches!(doc.paragraph(outer + 2).block, Block::Code { fence: false, .. }));
    assert!(matches!(doc.paragraph(outer + 3).block, Block::Code { last: false, fence: false, .. }), "``` внутри четырёх обратных кавычек не закрывает блок");
    assert!(matches!(doc.paragraph(outer + 4).block, Block::Code { last: true, fence: true, .. }));
    assert!(matches!(block(&doc, "graph TD"), Block::Code { fence: false, .. }));
    assert_eq!(block(&doc, "Текст сразу после кода."), &Block::Text);
    assert!(matches!(block(&doc, "console.log(\"код внутри details\");"), Block::Code { fence: false, lang: Some(_), .. }));
}

#[test]
fn code_sits_inside_its_quote_or_list_item() {
    let doc = corpus();
    let code = |line: &str| match framed(&doc, line) {
        (Block::Code { .. }, quote, list, lead) => Some((quote, list, lead)),
        _ => None,
    };
    assert_eq!(code("> print(\"Код в цитате\")"), Some((1, None, 2)));
    assert_eq!(code("> > Код во вложенной цитате"), Some((2, None, 4)));
    assert_eq!(code("> console.log(\"код в выноске\");"), Some((1, None, 2)));
    assert!(matches!(block(&doc, "> Незакрытое ограждение кончается вместе с цитатой"), Block::Code { last: true, .. }));
    assert_eq!(block(&doc, "Строка без «>» — уже обычный абзац, не код."), &Block::Text);
    assert_eq!(code("  echo \"код в пункте\""), Some((0, Some(0), 2)));
    assert_eq!(code("    print(\"код во вложенном пункте\")"), Some((0, Some(1), 4)));
}

#[test]
fn tables_in_every_shape_are_runs() {
    let doc = corpus();
    for header in ["| Левое выравнивание | По центру | Правое выравнивание |", "| Имя | Значение |", "Заголовок 1 | Заголовок 2 | Заголовок 3", "| A | B | C |"] {
        assert!(matches!(block(&doc, header), Block::Table { first: true, .. }), "{header}");
    }
    assert!(matches!(block(&doc, "| только одна |"), Block::Table { last: true, .. }));
    assert!(matches!(block(&doc, "<!--q-table:{\"merges\":[],\"cols\":[231,334]}-->"), Block::Table { last: true, .. }));
}

#[test]
fn horizontal_rules_in_all_spellings() {
    let doc = corpus();
    let from = at(&doc, "## 10. Горизонтальные линии");
    let until = at(&doc, "## 11. Сноски, аббревиатуры, комментарии");
    let rules = (from..until).filter(|&i| *doc.block(i) == Block::Rule).count();
    assert_eq!(rules, 7);
}

#[test]
fn comments_and_math_blocks_are_not_markup() {
    let doc = corpus();
    assert_eq!(block(&doc, "<!-- Это HTML-комментарий, он не должен отображаться -->"), &Block::Comment);
    assert_eq!(block(&doc, "Многострочный"), &Block::Comment);
    assert_eq!(block(&doc, "-->"), &Block::Comment);
    let math = at(&doc, "\\int_{-\\infty}^{\\infty} e^{-x^2}\\,dx = \\sqrt{\\pi}");
    assert!(matches!(doc.paragraph(math - 1).block, Block::Code { first: true, fence: true, lang: Some(Lang::Math), .. }));
    assert!(matches!(doc.paragraph(math).block, Block::Code { fence: false, .. }));
    assert!(matches!(doc.paragraph(math + 1).block, Block::Code { last: true, fence: true, .. }));
    assert_eq!(block(&doc, "Формула в одну строку: $$ x^2 + y^2 = z^2 $$"), &Block::Text);
}

#[test]
fn the_whole_note_lays_out_without_panicking() {
    let mut doc = corpus();
    doc.keep = (0.0, 1_000_000.0);
    let mut fcx = FontContext::default();
    let mut lcx = LayoutContext::new();
    doc.layout(&mut fcx, &mut lcx, &StyleSet::new(16.0), Some(700.0), None, None);
    assert!(doc.height() > 0.0);
}
