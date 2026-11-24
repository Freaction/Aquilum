use super::error::UiStateError;
use super::migrations::migrate;
use rusqlite::Connection;
use std::path::Path;

pub struct UiStateDatabase {
    pub connection: Connection,
}

impl UiStateDatabase {
    pub fn open(path: &Path) -> Result<Self, UiStateError> {
        let connection = crate::sqlite::open(path)?;
        migrate(&connection)?;
        Ok(Self { connection })
    }

    #[cfg(test)]
    pub fn memory() -> Result<Self, UiStateError> {
        Self::open(Path::new(":memory:"))
    }
}
