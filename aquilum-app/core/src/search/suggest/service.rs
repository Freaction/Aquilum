use super::rank::rank;
use super::store::{containing, starting_with};
use crate::search::error::SearchError;
use crate::search::models::NoteSuggestion;
use crate::search::service::SearchService;

const CANDIDATE_FACTOR: usize = 4;

impl SearchService {
    pub fn suggest_notes(
        &self,
        workspace: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<NoteSuggestion>, SearchError> {
        let key = query.trim().to_lowercase();
        if key.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let pool = limit * CANDIDATE_FACTOR;
        let candidates = self.with_metadata(workspace, |_, connection| {
            let mut candidates = starting_with(connection, &key, pool)?;
            if candidates.len() < limit {
                candidates.extend(containing(connection, &key, pool)?);
            }
            Ok(candidates)
        })?;
        Ok(rank(candidates, &key, limit))
    }
}
