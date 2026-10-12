use super::{Lang, Token, tokens};

fn kinds(line: &str, lang: Lang) -> Vec<(&str, Token)> {
    tokens(line, lang).into_iter().map(|(r, t)| (&line[r], t)).collect()
}

#[test]
fn highlights_common_tokens() {
    let got = kinds("fn main() { let x = 42; println!(\"hi\"); } // done", Lang::Rust);
    assert!(got.contains(&("fn", Token::Keyword)));
    assert!(got.contains(&("main", Token::Function)));
    assert!(got.contains(&("42", Token::Number)));
    assert!(got.contains(&("\"hi\"", Token::String)));
    assert!(got.contains(&("// done", Token::Comment)));
    let js = kinds("document.getElementById(\"demo\").innerHTML = txt", Lang::Script);
    assert!(js.contains(&("getElementById", Token::Function)));
    assert!(js.contains(&("innerHTML", Token::Property)));
    let html = kinds("<input type=\"button\">", Lang::Markup);
    assert!(html.contains(&("input", Token::Keyword)));
    assert!(html.contains(&("type", Token::Property)));
}
