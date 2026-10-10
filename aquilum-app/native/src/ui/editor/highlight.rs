use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token {
    Keyword,
    String,
    Number,
    Comment,
    Function,
    Type,
    Property,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Rust,
    Script,
    Python,
    Go,
    CLike,
    Shell,
    Sql,
    Css,
    Json,
    Yaml,
    Markup,
    Lua,
    Dataview,
}

const RUST: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
    "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
];
const SCRIPT: &[&str] = &[
    "async", "await", "break", "case", "catch", "class", "const", "continue", "default", "delete", "do", "else", "export", "extends", "false", "finally", "for", "from",
    "function", "if", "import", "in", "instanceof", "interface", "let", "new", "null", "of", "return", "static", "super", "switch", "this", "throw", "true", "try", "type",
    "typeof", "undefined", "var", "void", "while", "yield",
];
const PYTHON: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif", "else", "except", "False", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "None", "nonlocal", "not", "or", "pass", "raise", "return", "self", "True", "try", "while", "with", "yield",
];
const GO: &[&str] = &[
    "break", "case", "chan", "const", "continue", "default", "defer", "else", "false", "for", "func", "go", "goto", "if", "import", "interface", "map", "nil", "package",
    "range", "return", "select", "struct", "switch", "true", "type", "var",
];
const CLIKE: &[&str] = &[
    "abstract", "auto", "bool", "break", "case", "catch", "char", "class", "const", "continue", "default", "delete", "do", "double", "else", "enum", "extends", "false", "final",
    "float", "for", "fun", "if", "implements", "import", "int", "interface", "long", "namespace", "new", "null", "override", "package", "private", "protected", "public",
    "return", "short", "static", "struct", "switch", "this", "throw", "true", "try", "typedef", "using", "val", "var", "virtual", "void", "while",
];
const SHELL: &[&str] = &["case", "do", "done", "echo", "elif", "else", "esac", "export", "fi", "for", "function", "if", "in", "local", "return", "then", "while"];
const SQL: &[&str] = &[
    "and", "as", "asc", "by", "create", "delete", "desc", "distinct", "drop", "from", "group", "having", "in", "insert", "into", "is", "join", "left", "limit", "not", "null",
    "on", "or", "order", "select", "set", "table", "update", "values", "where", "with",
];
const DATAVIEW: &[&str] = &[
    "and", "as", "asc", "by", "calendar", "desc", "flatten", "from", "group", "id", "limit", "list", "not", "or", "sort", "table", "task", "where", "without", "once",
];
const LUA: &[&str] = &["and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in", "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while"];

impl Lang {
    pub fn from_info(info: &str) -> Option<Lang> {
        let name = info.split_whitespace().next().unwrap_or_default().to_ascii_lowercase();
        Some(match name.as_str() {
            "rust" | "rs" => Lang::Rust,
            "js" | "javascript" | "jsx" | "ts" | "typescript" | "tsx" | "mjs" => Lang::Script,
            "py" | "python" => Lang::Python,
            "go" | "golang" => Lang::Go,
            "c" | "cpp" | "c++" | "h" | "hpp" | "java" | "kotlin" | "kt" | "cs" | "csharp" | "c#" | "swift" | "dart" | "php" => Lang::CLike,
            "sh" | "bash" | "zsh" | "shell" | "ps1" | "powershell" | "bat" => Lang::Shell,
            "sql" => Lang::Sql,
            "dataview" | "dataviewjs" => Lang::Dataview,
            "css" | "scss" | "less" => Lang::Css,
            "json" | "jsonc" => Lang::Json,
            "yaml" | "yml" | "toml" | "ini" => Lang::Yaml,
            "html" | "xml" | "svg" | "vue" => Lang::Markup,
            "lua" => Lang::Lua,
            _ => return None,
        })
    }

    fn keywords(self) -> &'static [&'static str] {
        match self {
            Lang::Rust => RUST,
            Lang::Script => SCRIPT,
            Lang::Python => PYTHON,
            Lang::Go => GO,
            Lang::CLike => CLIKE,
            Lang::Shell => SHELL,
            Lang::Sql => SQL,
            Lang::Lua => LUA,
            Lang::Dataview => DATAVIEW,
            Lang::Css | Lang::Json | Lang::Yaml | Lang::Markup => &[],
        }
    }

    fn comment(self) -> &'static [&'static str] {
        match self {
            Lang::Python | Lang::Shell | Lang::Yaml => &["#"],
            Lang::Sql | Lang::Lua => &["--"],
            Lang::Dataview => &["//"],
            Lang::Markup => &["<!--"],
            Lang::Json => &[],
            _ => &["//", "/*"],
        }
    }
}

fn word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

pub fn tokens(line: &str, lang: Lang) -> Vec<(Range<usize>, Token)> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut prev = ' ';
    let mut in_tag = false;
    while i < line.len() {
        let rest = &line[i..];
        let c = rest.chars().next().unwrap_or(' ');
        if let Some(open) = lang.comment().iter().find(|m| rest.starts_with(**m)) {
            let close = match *open {
                "/*" => rest[2..].find("*/").map(|j| i + j + 4),
                "<!--" => rest[4..].find("-->").map(|j| i + j + 7),
                _ => None,
            };
            let end = close.unwrap_or(line.len());
            out.push((i..end, Token::Comment));
            i = end;
            continue;
        }
        if lang == Lang::Markup && c == '<' {
            let name = rest[1..].trim_start_matches(['/', '!']);
            let start = line.len() - name.len();
            let len = name.find(|c: char| !(c.is_alphanumeric() || c == '-')).unwrap_or(name.len());
            if len > 0 {
                out.push((start..start + len, Token::Keyword));
                in_tag = true;
                i = start + len;
                prev = 'a';
                continue;
            }
        }
        if lang == Lang::Markup && c == '>' {
            in_tag = false;
        }
        if matches!(c, '"' | '\'' | '`') && !(c == '\'' && lang == Lang::Rust && prev.is_alphanumeric()) {
            let mut j = i + 1;
            while j < line.len() {
                if bytes[j] == b'\\' {
                    j += 2;
                    continue;
                }
                if bytes[j] == c as u8 {
                    j += 1;
                    break;
                }
                j += 1;
            }
            let end = j.min(line.len());
            let key = matches!(lang, Lang::Json) && line[end..].trim_start().starts_with(':');
            out.push((i..end, if key { Token::Property } else { Token::String }));
            i = end;
            prev = c;
            continue;
        }
        if c.is_ascii_digit() && !word_char(prev) {
            let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '_')).map_or(line.len(), |j| i + j);
            out.push((i..end, Token::Number));
            i = end;
            prev = '0';
            continue;
        }
        if word_char(c) && !c.is_ascii_digit() {
            let end = rest.find(|c: char| !word_char(c) && c != '-' || (c == '-' && !matches!(lang, Lang::Css | Lang::Markup))).map_or(line.len(), |j| i + j);
            let word = &line[i..end];
            let after = line[end..].trim_start();
            let keywords = lang.keywords();
            let token = if (matches!(lang, Lang::Sql | Lang::Dataview) && keywords.contains(&word.to_ascii_lowercase().as_str())) || keywords.contains(&word) {
                Some(Token::Keyword)
            } else if in_tag || (lang == Lang::Css && after.starts_with(':')) || (lang == Lang::Yaml && after.starts_with(':') && line[..i].trim().is_empty()) {
                Some(Token::Property)
            } else if after.starts_with('(') || (lang == Lang::Rust && after.starts_with('!')) {
                Some(Token::Function)
            } else if prev == '.' {
                Some(Token::Property)
            } else if word.chars().next().is_some_and(|c| c.is_ascii_uppercase()) && !matches!(lang, Lang::Yaml | Lang::Shell | Lang::Sql | Lang::Dataview) {
                Some(Token::Type)
            } else {
                None
            };
            if let Some(token) = token {
                out.push((i..end, token));
            }
            i = end;
            prev = 'a';
            continue;
        }
        prev = c;
        i += c.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
