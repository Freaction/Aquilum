use crate::search::paths::title;
use super::models::AnalysisResult;
use super::MAX_REASONS;
use crate::search::paths::identity;
use crate::search::error::SearchError;
use crate::search::wiki;
use rusqlite::Connection;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

pub struct GraphSnapshot {
    neighbours: HashMap<String, HashSet<String>>,
    outgoing: HashMap<String, HashSet<String>>,
    paths: HashMap<String, String>,
}

struct CandidateScore {
    raw_score: f64,
    common_neighbours: HashSet<String>,
}

impl GraphSnapshot {
    pub fn load(connection: &Connection, root: &Path) -> Result<Self, SearchError> {
        let documents = wiki::read_documents(connection)?;
        let mut neighbours = HashMap::<String, HashSet<String>>::new();
        let mut outgoing = HashMap::<String, HashSet<String>>::new();
        let mut paths = HashMap::<String, String>::new();
        wiki::for_each_link(
            connection,
            root,
            documents.iter().map(|document| {
                (
                    document.path.as_str(),
                    document.relative_key.as_str(),
                    document.title_key.as_str(),
                )
            }),
            |source, target| {
                let source_path = source;
                let target_path = target;
                let source = identity(Path::new(source_path));
                let target = identity(Path::new(target_path));
                if source == target {
                    return;
                }
                paths
                    .entry(source.clone())
                    .or_insert_with(|| source_path.to_owned());
                paths
                    .entry(target.clone())
                    .or_insert_with(|| target_path.to_owned());
                neighbours
                    .entry(source.clone())
                    .or_default()
                    .insert(target.clone());
                neighbours
                    .entry(target.clone())
                    .or_default()
                    .insert(source.clone());
                outgoing.entry(source).or_default().insert(target);
            },
        )?;
        Ok(Self {
            neighbours,
            outgoing,
            paths,
        })
    }

    pub fn incoming_counts(&self, documents: &[PathBuf]) -> Vec<usize> {
        let mut counts = HashMap::<&str, usize>::new();
        for targets in self.outgoing.values() {
            for target in targets {
                *counts.entry(target.as_str()).or_default() += 1;
            }
        }
        documents
            .iter()
            .map(|document| counts.get(identity(document).as_str()).copied().unwrap_or(0))
            .collect()
    }

    pub fn adamic_adar(&self, document: &Path, limit: usize) -> Vec<AnalysisResult> {
        let current = identity(document);
        let Some(current_neighbours) = self.neighbours.get(&current) else {
            return Vec::new();
        };
        let mut shared = HashMap::<String, CandidateScore>::new();
        for neighbour in current_neighbours {
            let weight = self.neighbour_weight(neighbour);
            if weight == 0.0 {
                continue;
            }
            if let Some(candidates) = self.neighbours.get(neighbour) {
                for candidate in candidates {
                    if candidate != &current {
                        let entry =
                            shared
                                .entry(candidate.clone())
                                .or_insert_with(|| CandidateScore {
                                    raw_score: 0.0,
                                    common_neighbours: HashSet::new(),
                                });
                        entry.raw_score += weight;
                        entry.common_neighbours.insert(neighbour.clone());
                    }
                }
            }
        }
        self.rank(shared, limit)
    }

    fn neighbour_weight(&self, neighbour: &str) -> f64 {
        let out_degree = self.outgoing.get(neighbour).map_or(0, HashSet::len);
        if out_degree == 0 {
            return 0.0;
        }
        1.0 / (out_degree.max(2) as f64).ln()
    }

    fn rank(&self, values: HashMap<String, CandidateScore>, limit: usize) -> Vec<AnalysisResult> {
        let mut ranked = values
            .into_iter()
            .filter(|(_, candidate)| candidate.raw_score > 0.0)
            .filter_map(|(identity, candidate)| {
                let path = self.paths.get(&identity)?;
                Some((title(path), path, candidate))
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|(left_title, _, left), (right_title, _, right)| {
            right
                .raw_score
                .partial_cmp(&left.raw_score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| right.common_neighbours.len().cmp(&left.common_neighbours.len()))
                .then_with(|| left_title.cmp(right_title))
        });
        ranked.truncate(limit);
        ranked
            .into_iter()
            .map(|(title, path, candidate)| AnalysisResult {
                title,
                path: path.clone(),
                raw_score: candidate.raw_score,
                similarity: None,
                confidence: None,
                reasons: self.reasons(&candidate.common_neighbours),
            })
            .collect()
    }

    fn reasons(&self, neighbours: &HashSet<String>) -> Vec<String> {
        let mut weighted = neighbours
            .iter()
            .filter_map(|identity| Some((self.neighbour_weight(identity), title(self.paths.get(identity)?))))
            .collect::<Vec<_>>();
        weighted.sort_by(|left, right| right.0.total_cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        let mut reasons = weighted.into_iter().take(MAX_REASONS).map(|(_, title)| title).collect::<Vec<_>>();
        reasons.sort();
        reasons
    }
}
