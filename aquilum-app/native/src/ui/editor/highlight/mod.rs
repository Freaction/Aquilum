mod langs;
#[cfg(test)]
mod tests;
mod tokens;

pub use tokens::tokens;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    Math,
    Mermaid,
}
