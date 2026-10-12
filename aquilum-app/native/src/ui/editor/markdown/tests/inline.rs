use super::super::inline::{Kind, hashtags, inline};

#[test]
fn hashtags_need_space_around() {
    let text = "#tag #тег a#no #ok_1 #";
    let tags: Vec<&str> = hashtags(text).into_iter().map(|r| &text[r]).collect();
    assert_eq!(tags, ["#tag", "#тег", "#ok_1"]);
}

#[test]
fn inline_spans_have_inner_ranges() {
    let text = "a *b* **c** ~~d~~ `e` [f](g) [[h|i]] [3](aquilum-reader:x)";
    let spans = inline(text);
    let pick = |k: Kind| spans.iter().find(|s| s.kind == k).map(|s| &text[s.inner.clone()]).unwrap();
    assert_eq!(pick(Kind::Emphasis), "b");
    assert_eq!(pick(Kind::Strong), "c");
    assert_eq!(pick(Kind::Strike), "d");
    assert_eq!(pick(Kind::Code), "e");
    assert_eq!(pick(Kind::Reader), "3");
    let links: Vec<&str> = spans.iter().filter(|s| s.kind == Kind::Link).map(|s| &text[s.inner.clone()]).collect();
    assert_eq!(links, ["f", "i"]);
}

#[test]
fn bare_urls_autolinks_and_legacy_quotes_are_links_like_tauri() {
    let text = "см. https://example.com/a_(b), www.site.ru. и <https://x.org> или [[→]](quantum-reader:cfi=c) а не `https://code.io`";
    let spans = inline(text);
    let links: Vec<(&str, &str, Kind)> = spans.iter().filter(|s| matches!(s.kind, Kind::Link | Kind::Reader)).map(|s| (&text[s.inner.clone()], s.target.as_ref().unwrap().0.as_str(), s.kind)).collect();
    assert!(links.contains(&("https://example.com/a_(b)", "https://example.com/a_(b)", Kind::Link)), "{links:?}");
    assert!(links.contains(&("www.site.ru", "https://www.site.ru", Kind::Link)), "{links:?}");
    assert!(links.contains(&("https://x.org", "https://x.org", Kind::Link)), "{links:?}");
    assert!(links.contains(&("→", "quantum-reader:cfi=c", Kind::Reader)), "{links:?}");
    assert!(!links.iter().any(|(_, target, _)| target.contains("code.io")), "ссылка внутри кода: {links:?}");
}

#[test]
fn scripts_math_and_the_obsidian_marks_are_spans() {
    let text = "H~2~O и E = mc^2^, $x^2$, ==важно==, %%заметка%%, сноска[^1], ^id";
    let spans = inline(text);
    let inner = |k: Kind| spans.iter().filter(|s| s.kind == k).map(|s| &text[s.inner.clone()]).collect::<Vec<_>>();
    assert_eq!(inner(Kind::Sub), ["2"]);
    assert_eq!(inner(Kind::Sup), ["2"]);
    assert_eq!(inner(Kind::Math), ["x^2"]);
    assert_eq!(inner(Kind::Highlight), ["важно"]);
    assert_eq!(inner(Kind::Comment), ["%%заметка%%"]);
    assert_eq!(inner(Kind::Footnote), ["[^1]"]);
    assert_eq!(inner(Kind::BlockId), ["^id"]);
}
