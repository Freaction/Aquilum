use super::database::UiStateDatabase;
use super::error::UiStateError;
use rusqlite::params;
use std::collections::HashMap;
use std::path::PathBuf;

impl UiStateDatabase {
    pub fn track_missing(&mut self, now_ms: i64) -> Result<(), UiStateError> {
        let documents = self
            .connection
            .prepare(
                "SELECT d.id, w.path, d.relative_path, d.missing_since_ms IS NOT NULL
                 FROM documents d JOIN workspaces w ON w.id = d.workspace_id",
            )?
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, bool>(3)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut roots: HashMap<String, Option<PathBuf>> = HashMap::new();
        let transaction = self.connection.transaction()?;
        for (id, workspace, relative_path, missing) in documents {
            let root = roots
                .entry(workspace)
                .or_insert_with_key(|path| Some(PathBuf::from(path)).filter(|root| root.is_dir()));
            let Some(root) = root else { continue };
            let present = root.join(&relative_path).exists();
            if present == missing {
                transaction.execute(
                    "UPDATE documents SET missing_since_ms = ?2 WHERE id = ?1",
                    params![id, (!present).then_some(now_ms)],
                )?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn purge_missing(&mut self, cutoff_ms: i64, limit: i64) -> Result<usize, UiStateError> {
        if limit <= 0 {
            return Ok(0);
        }
        let bounded_limit = limit.clamp(1, 5_000);
        let deleted = self.connection.execute(
            "DELETE FROM documents WHERE id IN (
                 SELECT id FROM documents
                 WHERE missing_since_ms <= ?1
                 ORDER BY missing_since_ms LIMIT ?2
             )",
            params![cutoff_ms, bounded_limit],
        )?;
        Ok(deleted)
    }
}
