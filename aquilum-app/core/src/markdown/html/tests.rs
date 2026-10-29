use super::to_markdown;

fn md(html: &str) -> String {
    to_markdown(html).unwrap_or_else(|| "<plain>".to_owned())
}

#[test]
fn a_chrome_wrapper_with_a_link_becomes_a_markdown_link() {
    assert_eq!(md(r#"<meta charset='utf-8'><span style="font-weight: 400; font-style: normal">см. <a href="https://e.com/a">сайт</a></span>"#), "см. [сайт](https://e.com/a)");
}

#[test]
fn the_style_of_an_element_wins_over_its_tag() {
    let doc = r#"<b style="font-weight:normal" id="docs-internal-guid-1"><p dir="ltr"><span style="font-weight:700">А</span><span style="font-weight:400"> б </span><span style="font-style:italic">в</span><span style="text-decoration:line-through"> г</span></p></b>"#;
    assert_eq!(md(doc), "**А** б *в* ~~г~~");
}

#[test]
fn plain_text_in_wrappers_stays_plain() {
    assert_eq!(md(r#"<meta charset="utf-8"><span style="font-weight: 400; font-style: normal">просто текст</span>"#), "<plain>");
    assert_eq!(md("<p>один</p><p>два</p>"), "<plain>");
    assert_eq!(md("текст без тегов"), "<plain>");
    assert_eq!(md("<br> <br>"), "<plain>");
}

#[test]
fn nested_lists_keep_their_depth_even_when_the_inner_one_is_a_sibling() {
    let doc = r#"<ul><li aria-level="1"><p><span>один</span></p></li><ul><li aria-level="2"><p><span>два</span></p></li></ul><li aria-level="1"><p><span>три</span></p></li></ul>"#;
    assert_eq!(md(doc), "- один\n\t- два\n- три");
    assert_eq!(md("<ol start=\"3\"><li>три<ul><li>а</li></ul></li><li>четыре</li></ol>"), "3. три\n\t- а\n4. четыре");
}

#[test]
fn bare_list_items_from_a_fragment_make_a_list() {
    assert_eq!(md("<li>Item 3</li><li>Item 4</li>"), "- Item 3\n- Item 4");
}

#[test]
fn spaces_move_outside_the_marks() {
    assert_eq!(md("слово<b> жирное </b>конец"), "слово **жирное** конец");
    assert_eq!(md("<b>a</b><b>b</b>"), "**ab**");
    assert_eq!(md("<p>до <i>курсив <b>оба</b></i> после</p>"), "до *курсив **оба*** после");
}

#[test]
fn code_that_looks_like_an_editor_copy_stays_plain() {
    let doc = r#"<meta charset='utf-8'><div style="color:#d4d4d4;font-family:Consolas,monospace;white-space: pre;"><div><span style="color:#569cd6">let</span><span style="font-style:italic"> x</span></div></div>"#;
    assert_eq!(md(doc), "<plain>");
}

#[test]
fn a_design_tool_copy_without_structure_stays_plain() {
    let doc = r#"<meta charset="utf-8"><div><span data-metadata="<!--(figmeta)eyJ9(/figmeta)-->"></span><span data-buffer="<!--(figma)ZmlnLWt(/figma)-->"></span><span style="white-space:pre-wrap;">Кнопка</span></div>"#;
    assert_eq!(md(doc), "<plain>");
}

#[test]
fn word_markup_is_read_by_the_exact_property_name() {
    let doc = "<p class=MsoNormal>строка\r\nещё<o:p></o:p></p><p class=MsoNormal><b style='mso-bidi-font-weight:normal'>жирный</b></p>";
    assert_eq!(md(doc), "строка ещё\n\n**жирный**");
}

#[test]
fn headings_quotes_and_rules_are_block_syntax() {
    assert_eq!(md("<h2><b>Заголовок</b></h2><p>текст</p>"), "## Заголовок\n\nтекст");
    assert_eq!(md("<blockquote><p>раз</p><p>два</p></blockquote><hr><p>после</p>"), "> раз\n>\n> два\n\n---\n\nпосле");
}

#[test]
fn code_blocks_keep_their_text_and_language() {
    assert_eq!(md("<pre><code class=\"language-rust\">fn f() {\n    a &lt; b\n}\n</code></pre>"), "```rust\nfn f() {\n    a < b\n}\n```");
    assert_eq!(md("<p><b>x</b> <code>a</code></p>"), "**x** `a`");
    assert_eq!(md("<pre>````\n</pre>"), "`````\n````\n`````");
}

#[test]
fn text_is_escaped_so_it_stays_text() {
    let doc = "<p><b>Цена</b>: 5*3, snake_case, [1]&nbsp;&amp;&nbsp;A&copy;</p><p>1. не список</p><p>- не пункт</p>";
    assert_eq!(md(doc), "**Цена**: 5\\*3, snake_case, \\[1\\] & A©\n\n1\\. не список\n\n\\- не пункт");
}

#[test]
fn only_web_links_survive_and_empty_ones_vanish() {
    assert_eq!(md("<p><a href=https://e.com title=\"a>b\">x</a> <a href=\"#note\">сноска</a> <a href=\"https://e.com/i\"></a><a href=\"https://e.com/y\">https://e.com/y</a></p>"), "[x](https://e.com) сноска https://e.com/y");
    assert_eq!(md("<a href=\"https://www.google.com/url?q=https://e.com/a%3Fb&amp;sa=D\">ссылка</a>"), "[ссылка](https://e.com/a?b)");
    assert_eq!(md("<a href=\"https://e.com/a b(1)\">ссылка</a>"), "[ссылка](https://e.com/a%20b%281%29)");
}

#[test]
fn hidden_and_script_content_is_dropped() {
    let doc = "<style>p { color: red }</style><p>видно <span style=\"display:none\">скрыто</span><script>if (a<b) { x() }</script><b>жирно</b></p><!--[if gte mso 9]><xml>x</xml><![endif]-->";
    assert_eq!(md(doc), "видно **жирно**");
}

#[test]
fn a_table_is_left_to_the_plain_text() {
    assert_eq!(md("<table><tr><th>A</th><th>B</th></tr></table>"), "<plain>");
}

#[test]
fn line_breaks_close_and_reopen_marks() {
    assert_eq!(md("<p><b>раз<br>два</b></p><p>a<br><br>b <b>c</b></p>"), "**раз**\n**два**\n\na\n\nb **c**");
}

#[test]
fn broken_markup_never_panics() {
    let pieces = ["<", ">", "</", "<a", "<a href=", "<a href='x>т", "<!--", "<![if", "<?", "<p><b>", "</b></p></ul>", "&", "&#x;", "<<>>", "<li><li>", "<pre><code>", "<script>", "ж", "<style", "\"'", "<br/>", "<h7>", "<ol start=x>"];
    for first in pieces {
        for second in pieces {
            for third in pieces {
                let _ = to_markdown(&format!("{first}{second}{third}"));
            }
        }
    }
}

#[test]
fn task_list_checkboxes_become_markdown_tasks() {
    assert_eq!(md("<ul><li><input type=checkbox checked> готово</li><li><input type=checkbox> нет</li></ul>"), "- [x] готово\n- [ ] нет");
}
