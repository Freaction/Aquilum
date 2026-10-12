use super::Lang;

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

    pub(super) fn keywords(self) -> &'static [&'static str] {
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

    pub(super) fn comment(self) -> &'static [&'static str] {
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
