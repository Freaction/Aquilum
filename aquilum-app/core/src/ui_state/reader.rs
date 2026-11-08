use super::database::UiStateDatabase;
use super::error::UiStateError;
use super::models::{LoadedReaderState, SaveReaderStateInput};
use rusqlite::{params, OptionalExtension};

const MAX_CFI_LEN: usize = 16_384;

impl UiStateDatabase {
    pub fn delete_reader_state(&self, workspace_id: uuid::Uuid, book_file: &str) -> Result<(), UiStateError> {
        self.connection.execute(
            "DELETE FROM reader_states WHERE workspace_id = ?1 AND book_file = ?2",
            params![workspace_id.to_string(), book_file],
        )?;
        Ok(())
    }

    pub fn load_reader_state(
        &self,
        workspace_id: uuid::Uuid,
        book_file: &str,
    ) -> Result<Option<LoadedReaderState>, UiStateError> {
        Ok(self
            .connection
            .query_row(
                "SELECT current, cfi FROM reader_states
                 WHERE workspace_id = ?1 AND book_file = ?2",
                params![workspace_id.to_string(), book_file],
                |row| {
                    Ok(LoadedReaderState {
                        current: row.get(0)?,
                        cfi: row.get(1)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn save_reader_state(&self, input: &SaveReaderStateInput) -> Result<(), UiStateError> {
        if input.current < 0 {
            return Err(UiStateError::InvalidInput {
                message: "reader current must be >= 0".to_owned(),
            });
        }
        let cfi = match &input.cfi {
            Some(value) if value.len() > MAX_CFI_LEN => {
                return Err(UiStateError::InvalidInput {
                    message: "reader cfi is too long".to_owned(),
                });
            }
            Some(value) if value.is_empty() => None,
            other => other.clone(),
        };
        self.connection.execute(
            "INSERT INTO reader_states(workspace_id, book_file, current, cfi, updated_at_ms)
             VALUES(?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(workspace_id, book_file) DO UPDATE SET
                 current = excluded.current,
                 cfi = excluded.cfi,
                 updated_at_ms = excluded.updated_at_ms",
            params![
                input.workspace_id.to_string(),
                input.book_file,
                input.current,
                cfi,
                input.now_ms
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_state_round_trips() {
        let mut database = UiStateDatabase::memory().expect("database");
        let workspace_id = database
            .resolve_workspace("C:/notes", 1)
            .expect("workspace");
        database
            .save_reader_state(&SaveReaderStateInput {
                workspace_id,
                book_file: "Files/book.epub".to_owned(),
                current: 42,
                cfi: Some("epubcfi(/6/2)".to_owned()),
                now_ms: 10,
            })
            .expect("save");
        let loaded = database
            .load_reader_state(workspace_id, "Files/book.epub")
            .expect("load")
            .expect("row");
        assert_eq!(loaded.current, 42);
        assert_eq!(loaded.cfi.as_deref(), Some("epubcfi(/6/2)"));
        database.delete_reader_state(workspace_id, "Files/book.epub").expect("delete");
        assert!(database.load_reader_state(workspace_id, "Files/book.epub").expect("load after delete").is_none());
    }
}
