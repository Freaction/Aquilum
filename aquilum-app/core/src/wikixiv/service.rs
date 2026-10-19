use super::cache::content_hash;
use super::extract::{body_term_lang, extract_queries, has_enough_text, is_cyrillic};
use super::http::HttpClient;
use super::models::{SearchRequest, WikixivHit, WikixivSearchResult};
use super::wikipedia;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::recent::Recent;

const MAX_TOTAL_HITS: usize = 20;
const CACHED_SEARCHES: usize = 32;

#[derive(Clone)]
pub struct WikixivService {
    inner: Arc<Inner>,
}

struct Inner {
    client: HttpClient,
    generation: AtomicU64,
    cache: Mutex<Recent<blake3::Hash, Vec<WikixivHit>>>,
}

impl WikixivService {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                client: HttpClient::new(),
                generation: AtomicU64::new(0),
                cache: Mutex::new(Recent::new(CACHED_SEARCHES, usize::MAX)),
            }),
        }
    }

    pub fn forget(&self) {
        if let Ok(mut cache) = self.inner.cache.lock() {
            cache.clear();
        }
    }

    pub fn search(
        &self,
        request: SearchRequest,
        enabled: bool,
    ) -> Result<WikixivSearchResult, String> {
        if !enabled {
            return Ok(empty(request.generation, false, false));
        }

        self.inner
            .generation
            .store(request.generation, Ordering::SeqCst);

        let text = request.text.as_str();
        let extracted = extract_queries(text, request.document_path.as_deref());
        if extracted.title.is_none()
            && extracted.embedded_titles.is_empty()
            && !has_enough_text(extracted.word_count)
        {
            return Ok(empty(request.generation, false, true));
        }
        if extracted.title.is_none()
            && extracted.embedded_titles.is_empty()
            && extracted.body_terms.is_empty()
        {
            return Ok(empty(request.generation, false, false));
        }

        let cache_key = content_hash(request.document_path.as_deref(), text);
        if let Ok(mut cache) = self.inner.cache.lock() {
            if let Some(hits) = cache.get(&cache_key).cloned() {
                if self.is_current(request.generation) {
                    return Ok(WikixivSearchResult {
                        hits,
                        generation: request.generation,
                        offline: false,
                        insufficient_text: false,
                    });
                }
            }
        }

        let result = collect(&self.inner.client, &extracted);
        if !self.is_current(request.generation) {
            return Ok(empty(request.generation, false, false));
        }

        match result {
            Ok(mut hits) => {
                hits.truncate(MAX_TOTAL_HITS);
                if !hits.is_empty() {
                    if let Ok(mut cache) = self.inner.cache.lock() {
                        cache.put(cache_key, hits.clone(), 0);
                    }
                }
                Ok(WikixivSearchResult {
                    hits,
                    generation: request.generation,
                    offline: false,
                    insufficient_text: false,
                })
            }
            Err(error) if error.is_offline() => Ok(empty(request.generation, true, false)),
            Err(_) => Ok(empty(request.generation, false, false)),
        }
    }

    fn is_current(&self, generation: u64) -> bool {
        self.inner.generation.load(Ordering::SeqCst) == generation
    }
}

fn empty(generation: u64, offline: bool, insufficient_text: bool) -> WikixivSearchResult {
    WikixivSearchResult {
        hits: Vec::new(),
        generation,
        offline,
        insufficient_text,
    }
}

fn collect(
    client: &HttpClient,
    extracted: &super::extract::ExtractedQueries,
) -> Result<Vec<WikixivHit>, super::http::HttpError> {
    let mut hits = Vec::new();
    let mut seen = HashSet::new();
    let mut last_error = None;

    let explicit_titles = extracted.title.iter().map(|title| (title, true)).chain(
        extracted.embedded_titles.iter().map(|title| (title, false)),
    );
    for (title, from_filename) in explicit_titles {
        if hits.len() >= MAX_TOTAL_HITS {
            break;
        }
        let preferred_lang = if title.chars().any(is_cyrillic) { "ru" } else { "en" };
        let mut resolved_directly = false;

        for lang in [preferred_lang, if preferred_lang == "ru" { "en" } else { "ru" }] {
            match wikipedia::summary(client, lang, title) {
                Ok(Some(mut hit)) => {
                    resolved_directly = true;
                    hit.from_filename = from_filename;
                    hit.matched_terms = vec![title.clone()];
                    push(&mut hits, &mut seen, hit);
                    break;
                }
                Ok(None) => {}
                Err(error) => last_error = Some(error),
            }
        }

        if !resolved_directly {
            for lang in [preferred_lang, if preferred_lang == "ru" { "en" } else { "ru" }] {
                match wikipedia::search_title(client, lang, title) {
                    Ok(Some(mut hit)) => {
                        hit.from_filename = from_filename;
                        hit.matched_terms = vec![title.clone()];
                        push(&mut hits, &mut seen, hit);
                        break;
                    }
                    Ok(None) => {}
                    Err(error) => last_error = Some(error),
                }
            }
        }
    }

    if !hits.is_empty() {
        return Ok(hits);
    }

    for term in &extracted.body_terms {
        if hits.len() >= MAX_TOTAL_HITS {
            break;
        }
        match wikipedia::search_intitle(client, body_term_lang(term), term) {
            Ok(batch) => {
                for mut hit in batch {
                    hit.matched_terms = vec![term.clone()];
                    push(&mut hits, &mut seen, hit);
                }
            }
            Err(error) => last_error = Some(error),
        }
    }

    if hits.is_empty() {
        if let Some(error) = last_error {
            return Err(error);
        }
    }
    Ok(hits)
}

fn push(hits: &mut Vec<WikixivHit>, seen: &mut HashSet<String>, hit: WikixivHit) {
    if seen.insert(hit.url.to_ascii_lowercase()) {
        hits.push(hit);
    }
}

