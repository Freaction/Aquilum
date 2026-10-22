mod derive;
mod parse;
mod plan;
mod rank;
#[cfg(test)]
mod tests;

use super::analyzer::indexing_tokenizer;
use super::error::SearchError;
use super::hits::{self, note_pages, note_path};
use super::index::{SearchFields, SearchIndex};
use super::models::SearchResult;
use crate::text::cancel::Cancel;
use derive::Terms;
use parse::{parse, Word};
use plan::{plan, Plan};
use rank::{Acc, Candidate};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::Path;
use tantivy::tokenizer::TokenStream;
use tantivy::{DocAddress, DocId, Searcher};

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 100;

pub struct Request<'a> {
    pub text: &'a str,
    pub folder: Option<&'a Path>,
    pub limit: Option<usize>,
    pub cancel: Cancel,
    pub metadata: Option<&'a Path>,
}

impl<'a> Request<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text, folder: None, limit: None, cancel: Cancel::never(), metadata: None }
    }
}

pub struct Found {
    pub terms: Vec<String>,
    pub results: Vec<SearchResult>,
    pub has_more: bool,
}

pub fn stems(text: &str) -> Vec<String> {
    let mut stems = Vec::new();
    indexing_tokenizer().token_stream(text).process(&mut |token| stems.push(token.text.clone()));
    stems
}

pub fn search(index: &SearchIndex, request: &Request) -> Result<Found, SearchError> {
    let parsed = parse(request.text);
    if parsed.words.is_empty() {
        return Ok(Found { terms: Vec::new(), results: Vec::new(), has_more: false });
    }
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let searcher = index.reader.searcher();
    let plan = plan(&searcher, index.fields, &parsed, request.folder)?;
    let (mut segments, mut found, mut later) = (Vec::new(), Vec::new(), Vec::new());
    for (ord, segment) in searcher.segment_readers().iter().enumerate() {
        segments.push(rank::segment(segment, ord as u32, &plan, &mut found, &mut later)?);
        if request.cancel.is_cancelled() {
            return Err(SearchError::Cancelled);
        }
    }
    join_pages(&searcher, &plan, &segments, later, &mut found)?;
    found.sort_unstable_by(|left, right| right.rank.partial_cmp(&left.rank).unwrap_or(Ordering::Equal));
    let mut notes = walk(&searcher, plan.fields, &segments, found, &parsed.words, limit)?;
    let has_more = notes.len() > limit;
    notes.truncate(limit);
    let terms = highlighted(&parsed.words, &plan.terms);
    let results = hits::results(notes, &terms, request.metadata, &request.cancel)?;
    Ok(Found { terms, results, has_more })
}

pub struct Note {
    acc: Acc,
    title: u32,
    pages: Vec<(u32, DocId)>,
    hits: HashSet<String>,
}

fn gather(searcher: &Searcher, fields: SearchFields, segments: &[Vec<Acc>], path: &str) -> Result<Note, SearchError> {
    let mut note = Note { acc: Acc::default(), title: 0, pages: Vec::new(), hits: HashSet::new() };
    for ((ord, segment), accs) in searcher.segment_readers().iter().enumerate().zip(segments) {
        let titles = segment.get_fieldnorms_reader(fields.title)?;
        note_pages(segment, fields, path, |doc, key| {
            let page = &accs[doc as usize];
            note.acc.merge(page);
            note.title = note.title.max(titles.fieldnorm(doc));
            note.pages.push((ord as u32, doc));
            if page.body {
                note.hits.insert(String::from_utf8_lossy(key).into_owned());
            }
        })?;
    }
    Ok(note)
}

fn join_pages(searcher: &Searcher, plan: &Plan, segments: &[Vec<Acc>], later: Vec<(u32, DocId)>, found: &mut Vec<Candidate>) -> Result<(), SearchError> {
    let (mut joined, mut notes) = (HashSet::new(), Vec::new());
    for (ord, doc) in later {
        if joined.contains(&(ord, doc)) {
            continue;
        }
        let Some(path) = note_path(searcher, plan.fields, DocAddress::new(ord, doc))? else { continue };
        let note = gather(searcher, plan.fields, segments, &path)?;
        joined.extend(note.pages.iter().copied());
        if let Some(rank) = note.acc.rank(note.title, plan) {
            notes.push(Candidate { rank, ord, doc, joined: Some(Box::new((path, note))) });
        }
    }
    found.retain(|candidate| !joined.contains(&(candidate.ord, candidate.doc)));
    found.extend(notes);
    Ok(())
}

fn walk(searcher: &Searcher, fields: SearchFields, segments: &[Vec<Acc>], found: Vec<Candidate>, words: &[Word], limit: usize) -> Result<Vec<(SearchResult, HashSet<String>, usize)>, SearchError> {
    let mut notes = Vec::new();
    for candidate in found {
        if notes.len() > limit {
            break;
        }
        let (path, note) = match candidate.joined {
            Some(joined) => *joined,
            None => {
                let Some(path) = note_path(searcher, fields, DocAddress::new(candidate.ord, candidate.doc))? else { continue };
                let note = gather(searcher, fields, segments, &path)?;
                (path, note)
            }
        };
        if !note.acc.excluded {
            let matched = words.iter().enumerate().filter(|(index, _)| note.acc.any & 1 << index != 0).map(|(_, word)| word.stem.clone()).collect();
            notes.push((hits::found(path, note.acc.count as usize, matched), note.hits, note.pages.len()));
        }
    }
    Ok(notes)
}

fn highlighted(words: &[Word], terms: &Terms) -> Vec<String> {
    let mut found: Vec<String> = words.iter().map(|word| word.stem.clone()).collect();
    for (term, entry) in terms.iter().flatten() {
        if words.iter().enumerate().any(|(index, word)| entry.any & 1 << index != 0 && !term.starts_with(&word.stem)) {
            found.push(term.clone());
        }
    }
    found.sort();
    found.dedup();
    found
}
