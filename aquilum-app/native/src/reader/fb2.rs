use super::dom::{Dom, NodeId};

#[derive(Clone, Copy, PartialEq)]
enum Ctx {
    Root,
    Body,
    Title,
    Section,
    Style,
    Table,
    Row,
    Poem,
    Stanza,
    StanzaTitle,
    Nothing,
}

enum Rule {
    Element(&'static str, Ctx, &'static [&'static str]),
    Image,
    Anchor,
    Stanza,
}

const CELL: &[&str] = &["colspan", "rowspan", "align", "valign"];

fn rule(ctx: Ctx, name: &str) -> Option<Rule> {
    use Rule::Element as E;
    Some(match (ctx, name) {
        (Ctx::Root, "body") => E("body", Ctx::Body, &[]),
        (Ctx::Body | Ctx::Section | Ctx::Style, "image") => Rule::Image,
        (Ctx::Body, "title") => E("section", Ctx::Title, &[]),
        (Ctx::Body, "epigraph" | "section") => E("section", Ctx::Section, &[]),
        (Ctx::Title, "p") => E("h1", Ctx::Style, &[]),
        (Ctx::Title | Ctx::Section | Ctx::StanzaTitle, "empty-line") => E("br", Ctx::Nothing, &[]),
        (Ctx::Section, "title") => E("header", Ctx::Title, &[]),
        (Ctx::Section, "epigraph" | "cite") => E("blockquote", Ctx::Section, &[]),
        (Ctx::Section, "annotation") => E("aside", Ctx::Nothing, &[]),
        (Ctx::Section, "section") => E("section", Ctx::Section, &[]),
        (Ctx::Section, "p" | "text-author") => E("p", Ctx::Style, &[]),
        (Ctx::Section, "poem") => E("blockquote", Ctx::Poem, &[]),
        (Ctx::Section | Ctx::Poem, "subtitle") => E("h2", Ctx::Style, &[]),
        (Ctx::Section, "table") => E("table", Ctx::Table, &[]),
        (Ctx::Style, "strong") => E("strong", Ctx::Style, &[]),
        (Ctx::Style, "emphasis") => E("em", Ctx::Style, &[]),
        (Ctx::Style, "style") => E("span", Ctx::Style, &[]),
        (Ctx::Style, "strikethrough") => E("s", Ctx::Style, &[]),
        (Ctx::Style, "sub") => E("sub", Ctx::Style, &[]),
        (Ctx::Style, "sup") => E("sup", Ctx::Style, &[]),
        (Ctx::Style, "code") => E("code", Ctx::Style, &[]),
        (Ctx::Style, "a") => Rule::Anchor,
        (Ctx::Table, "tr") => E("tr", Ctx::Row, &["align"]),
        (Ctx::Row, "th") => E("th", Ctx::Style, CELL),
        (Ctx::Row, "td") => E("td", Ctx::Style, CELL),
        (Ctx::Poem, "epigraph") => E("blockquote", Ctx::Section, &[]),
        (Ctx::Poem, "text-author" | "date") => E("p", Ctx::Style, &[]),
        (Ctx::Poem, "stanza") => Rule::Stanza,
        (Ctx::Stanza, "title") => E("header", Ctx::StanzaTitle, &[]),
        (Ctx::Stanza, "subtitle") => E("p", Ctx::Style, &[]),
        (Ctx::StanzaTitle, "p") => E("strong", Ctx::Style, &[]),
        _ => return None,
    })
}

type Source<'a, 'b> = roxmltree::Node<'a, 'b>;

fn attr<'a>(node: Source<'a, '_>, name: &str) -> Option<&'a str> {
    node.attributes().find(|a| a.name() == name).map(|a| a.value())
}

fn text_content(node: Source<'_, '_>) -> String {
    node.descendants().filter(|n| n.is_text()).filter_map(|n| n.text()).collect()
}

fn element(out: &mut Dom, parent: NodeId, node: Source<'_, '_>, name: &str, keep: &[&str]) -> NodeId {
    let mut attrs: Vec<(Box<str>, Box<str>)> = Vec::new();
    if let Some(id) = attr(node, "id").filter(|v| !v.is_empty()) {
        attrs.push(("id".into(), id.into()));
    }
    attrs.push(("class".into(), node.tag_name().name().into()));
    for key in keep {
        if let Some(value) = attr(node, key).filter(|v| !v.is_empty()) {
            attrs.push(((*key).into(), value.into()));
        }
    }
    out.element(parent, name, attrs)
}

fn children(out: &mut Dom, parent: NodeId, node: Source<'_, '_>, ctx: Ctx) {
    for child in node.children() {
        convert(out, parent, child, ctx);
    }
}

fn convert(out: &mut Dom, parent: NodeId, node: Source<'_, '_>, ctx: Ctx) -> Option<NodeId> {
    if node.is_text() {
        return Some(out.text(parent, node.text().unwrap_or_default()));
    }
    if !node.is_element() {
        return None;
    }
    let name = node.tag_name().name();
    match rule(ctx, name)? {
        Rule::Element(tag, child, keep) => {
            let id = element(out, parent, node, tag, keep);
            children(out, id, node, child);
            Some(id)
        }
        Rule::Image => {
            let href = attr(node, "href").unwrap_or("data:,");
            let attrs = vec![("alt".into(), attr(node, "alt").unwrap_or("null").into()), ("title".into(), attr(node, "title").unwrap_or("null").into()), ("src".into(), href.into())];
            Some(out.element(parent, "img", attrs))
        }
        Rule::Anchor => {
            let id = element(out, parent, node, "a", &[]);
            children(out, id, node, Ctx::Style);
            Some(id)
        }
        Rule::Stanza => {
            let id = element(out, parent, node, "p", &[]);
            children(out, id, node, Ctx::Stanza);
            for verse in node.children().filter(|c| c.is_element() && c.tag_name().name() == "v") {
                out.text(id, &text_content(verse));
                out.element(id, "br", Vec::new());
            }
            Some(id)
        }
    }
}

fn page() -> (Dom, NodeId) {
    let mut dom = Dom::new("html", vec![("xmlns".into(), "http://www.w3.org/1999/xhtml".into())]);
    dom.text(Dom::ROOT, "\n    ");
    let head = dom.element(Dom::ROOT, "head", Vec::new());
    dom.element(head, "link", vec![("rel".into(), "stylesheet".into()), ("type".into(), "text/css".into())]);
    dom.text(Dom::ROOT, "\n    ");
    let body = dom.element(Dom::ROOT, "body", Vec::new());
    dom.text(Dom::ROOT, "\n");
    (dom, body)
}

pub fn sections(doc: &roxmltree::Document<'_>) -> Vec<Dom> {
    let bodies: Vec<Source<'_, '_>> = doc.root_element().children().filter(|c| c.is_element() && c.tag_name().name() == "body").collect();
    let mut out = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        if index == 0 {
            for child in body.children().filter(|c| c.is_element()) {
                if rule(Ctx::Body, child.tag_name().name()).is_none() {
                    continue;
                }
                let (mut dom, root) = page();
                convert(&mut dom, root, child, Ctx::Body);
                out.push(dom);
            }
        } else {
            let (mut dom, root) = page();
            if let Some(converted) = convert(&mut dom, root, *body, Ctx::Root) {
                notes(&mut dom, converted);
            }
            out.push(dom);
        }
    }
    out
}

fn notes(dom: &mut Dom, body: NodeId) {
    dom.add_class(body, "notesBodyType");
}
