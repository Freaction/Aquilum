use crate::ui::editor::markdown::{self, inline_spans};

fn kinds(line: &str) -> Vec<(markdown::SpanKind, &str)> {
    inline_spans(line).into_iter().map(|s| (s.kind, &line[s.inner])).collect()
}

fn count(line: &str, kind: markdown::SpanKind) -> usize {
    kinds(line).iter().filter(|(k, _)| *k == kind).count()
}

#[test]
fn text_formatting_in_every_flavour() {
    use markdown::SpanKind::*;
    assert_eq!(count("*Курсив со звёздочками* и _курсив с подчёркиваниями_.", Emphasis), 2);
    assert_eq!(count("**Жирный со звёздочками** и __жирный с подчёркиваниями__.", Strong), 2);
    assert_eq!(count("~~Зачёркнутый текст~~ (GFM).", Strike), 1);
    assert_eq!(kinds("==Выделенный текст== (расширение highlight)."), [(Highlight, "Выделенный текст")]);
    let nested = kinds("==Выделение== в стиле Obsidian и ==выделение с **жирным** внутри==.");
    assert_eq!(nested.iter().filter(|(k, _)| *k == Highlight).count(), 2);
    assert_eq!(nested.iter().filter(|(k, _)| *k == Strong).count(), 1);
    let scripts = kinds("H~2~O — нижний индекс. E = mc^2^ — верхний индекс.");
    assert_eq!(scripts, [(Sub, "2"), (Sup, "2")]);
    let html = kinds("<sub>HTML sub</sub> и <sup>HTML sup</sup>, <u>подчёркнутый</u>, <mark>mark</mark>, <kbd>Ctrl</kbd> + <kbd>S</kbd>, <small>мелкий</small>, <ins>вставка</ins>, <del>удаление</del>.");
    assert_eq!(html.iter().filter(|(k, _)| *k == Sub).count(), 1);
    assert_eq!(html.iter().filter(|(k, _)| *k == Sup).count(), 1);
    assert_eq!(html.iter().filter(|(k, _)| *k == Underline).count(), 2);
    assert_eq!(html.iter().filter(|(k, _)| *k == Highlight).count(), 1);
    assert_eq!(html.iter().filter(|(k, _)| *k == Code).count(), 2);
    assert_eq!(html.iter().filter(|(k, _)| *k == Strike).count(), 1);
    assert_eq!(count("Слова_с_подчёркиваниями_внутри не должны становиться курсивом. 2*3*4 — тоже спорный случай. snake_case_name и __init__.py.", Emphasis), 1);
}

#[test]
fn escapes_hide_their_backslash_and_make_no_markup() {
    use markdown::SpanKind::*;
    let line = "Экранирование: \\*не курсив\\*, \\_не курсив\\_, \\# не заголовок, \\[не ссылка\\], \\`не код\\`, \\\\ обратный слэш, \\> не цитата, \\- не список, \\| не таблица.";
    assert_eq!(count(line, Escape), 13);
    assert_eq!(count(line, Emphasis) + count(line, Code) + count(line, Link), 0);
    assert_eq!(count("Код со спецсимволами: `<div class=\"a\">&amp;</div>`, `**не жирный**`, `[не ссылка](url)`.", Code), 3);
    assert_eq!(count("Код со спецсимволами: `<div class=\"a\">&amp;</div>`, `**не жирный**`, `[не ссылка](url)`.", Strong), 0);
}

#[test]
fn obsidian_marks_footnotes_ids_and_tags() {
    use markdown::SpanKind::*;
    assert_eq!(count("Текст со сноской[^1] и ещё одной[^note] и встроенной^[Встроенная сноска прямо в тексте.].", Footnote), 2);
    assert_eq!(count("Текст со сноской[^1] и ещё одной[^note] и встроенной^[Встроенная сноска прямо в тексте.].", Sup), 0);
    assert_eq!(kinds("Абзац с идентификатором блока. ^block-id"), [(BlockId, "^block-id")]);
    assert_eq!(count("%% Это комментарий Obsidian, видимый только в редакторе %%", Comment), 1);
    let tags = "#тег #tag #вложенный/тег #tag-with-dash #tag_with_underscore #2026";
    let found: Vec<&str> = markdown::hashtags(tags).into_iter().map(|r| &tags[r]).collect();
    assert_eq!(found, ["#тег", "#tag", "#вложенный/тег", "#tag-with-dash", "#tag_with_underscore"]);
    assert!(markdown::hashtags("Не тег: # пробел после решётки, C# или http://example.com/#anchor.").is_empty());
}

#[test]
fn wiki_links_embeds_and_math() {
    use markdown::SpanKind::*;
    let alias = inline_spans("[[Заметка|Отображаемый текст]]");
    assert_eq!(alias.len(), 1);
    assert_eq!(alias[0].target.as_ref().map(|(t, wiki)| (t.as_str(), *wiki)), Some(("Заметка", true)));
    assert_eq!(inline_spans("![[Встроенная заметка]]").iter().filter(|s| s.kind == Link).count(), 1);
    assert_eq!(count("Встроенная формула: $E = mc^2$, дробь $\\frac{a}{b}$, сумма $\\sum_{i=1}^{n} i = \\frac{n(n+1)}{2}$, греческие $\\alpha, \\beta, \\gamma, \\Omega$.", Math), 4);
    assert_eq!(count("Цена в долларах: $5 и $10 — не формула (одиночные доллары с пробелами).", Math), 0);
    assert_eq!(count("Формула в одну строку: $$ x^2 + y^2 = z^2 $$", Math), 1);
}
