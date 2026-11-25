use proc_macro2::{Delimiter, TokenStream, TokenTree};

pub fn each(tokens: TokenStream, visit: &mut impl FnMut(&TokenTree)) {
    for tree in tokens {
        if let TokenTree::Group(group) = &tree {
            each(group.stream(), visit);
        } else {
            visit(&tree);
        }
    }
}

pub fn words(tokens: TokenStream, mut word: impl FnMut(String)) {
    each(tokens, &mut |tree| match tree {
        TokenTree::Ident(ident) => word(ident.to_string().trim_start_matches("r#").to_owned()),
        TokenTree::Literal(literal) => {
            for part in literal.to_string().split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|part| !part.is_empty()) {
                word(part.to_owned());
            }
        }
        _ => {}
    });
}

pub fn flat(tokens: TokenStream, out: &mut Vec<(String, usize)>) {
    for tree in tokens {
        let line = tree.span().start().line;
        match &tree {
            TokenTree::Group(group) => {
                let (open, close) = match group.delimiter() {
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::None => ("", ""),
                };
                out.push((open.to_owned(), line));
                flat(group.stream(), out);
                out.push((close.to_owned(), group.span_close().end().line));
            }
            other => out.push((other.to_string(), line)),
        }
    }
}

pub fn lines(tokens: TokenStream) -> (usize, usize) {
    let mut range: Option<(usize, usize)> = None;
    each(tokens, &mut |tree| {
        let span = tree.span();
        let (start, end) = (span.start().line, span.end().line);
        range = Some(range.map_or((start, end), |(first, last)| (first.min(start), last.max(end))));
    });
    range.unwrap_or_default()
}
