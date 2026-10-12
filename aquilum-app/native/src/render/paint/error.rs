//! Ошибки исполнения команд сцены.

#[derive(Debug)]
pub enum PaintError {
    /// `pop_clip` / `pop_group` без парного `push`.
    Unbalanced(&'static str),
}

impl std::fmt::Display for PaintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaintError::Unbalanced(what) => write!(f, "несбалансированная сцена: {what}"),
        }
    }
}
