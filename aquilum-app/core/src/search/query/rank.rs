use super::derive::{Place, PLACES};
use super::plan::Plan;
use super::Note;
use crate::search::analysis::bm25f::score::{normalized_tf, saturate};
use crate::search::analysis::bm25f::terms::idf;
use crate::search::error::SearchError;
use crate::settings::models::Bm25fParams;
use std::cmp::Reverse;
use tantivy::postings::Postings;
use tantivy::schema::IndexRecordOption;
use tantivy::{DocId, DocSet, SegmentReader, TERMINATED};

#[derive(Clone, Copy, Default)]
pub struct Acc {
    pub any: u32,
    exact: u32,
    near: u32,
    title: u32,
    headings: u32,
    pub count: u32,
    score: f32,
    pub body: bool,
    allowed: bool,
    pub excluded: bool,
}

#[derive(PartialEq, PartialOrd)]
pub struct Rank {
    words: u32,
    typos: Reverse<u32>,
    title: u32,
    exact_title: bool,
    headings: u32,
    score: f32,
}

pub struct Candidate {
    pub rank: Rank,
    pub ord: u32,
    pub doc: DocId,
    pub joined: Option<Box<(String, Note)>>,
}

impl Acc {
    pub fn merge(&mut self, page: &Acc) {
        (self.any, self.exact, self.near, self.title, self.headings) = (self.any | page.any, self.exact | page.exact, self.near | page.near, self.title | page.title, self.headings | page.headings);
        (self.count, self.score) = (self.count + page.count, self.score + page.score);
        (self.body, self.allowed, self.excluded) = (self.body || page.body, self.allowed || page.allowed, self.excluded || page.excluded);
    }

    pub fn rank(&self, title_length: u32, plan: &Plan) -> Option<Rank> {
        let fits = self.any != 0 && self.any & plan.words == plan.words && (plan.required.is_none() || self.allowed) && !self.excluded;
        fits.then(|| Rank {
            words: self.any.count_ones(),
            typos: Reverse((self.any & !self.exact).count_ones() + (self.any & !self.near).count_ones()),
            title: self.title.count_ones(),
            exact_title: self.title == plan.all && title_length == plan.all.count_ones(),
            headings: self.headings.count_ones(),
            score: self.score,
        })
    }
}

pub fn segment(segment: &SegmentReader, ord: u32, plan: &Plan, found: &mut Vec<Candidate>, later: &mut Vec<(u32, DocId)>) -> Result<Vec<Acc>, SearchError> {
    let mut accs = vec![Acc::default(); segment.max_doc() as usize];
    let norms = segment.get_fieldnorms_reader(plan.fields.body)?;
    let params = Bm25fParams::default();
    for (place, terms) in PLACES.into_iter().zip(&plan.terms) {
        let inverted = segment.inverted_index(place.field(plan.fields))?;
        for entry in terms.values() {
            let Some(info) = &entry.infos[ord as usize] else { continue };
            let mut postings = inverted.read_postings_from_terminfo(info, IndexRecordOption::WithFreqs)?;
            let idf = idf((entry.doc_freq as f64).min(plan.documents), plan.documents);
            while postings.doc() != TERMINATED {
                let (doc, tf) = (postings.doc(), postings.term_freq());
                let acc = &mut accs[doc as usize];
                (acc.any, acc.exact, acc.near) = (acc.any | entry.any, acc.exact | entry.exact, acc.near | entry.near);
                match place {
                    Place::Title => (acc.title, acc.count) = (acc.title | entry.any, acc.count + tf),
                    Place::Headings => acc.headings |= entry.any,
                    Place::Body => {
                        let weight = normalized_tf(f64::from(tf), norms.fieldnorm(doc), plan.average, f64::from(params.b_body));
                        (acc.count, acc.score, acc.body) = (acc.count + tf, acc.score + (idf * saturate(weight, f64::from(params.k1))) as f32, true);
                    }
                }
                postings.advance();
            }
        }
    }
    if let Some(weight) = &plan.required {
        weight.for_each_no_score(segment, &mut |docs| docs.iter().for_each(|&doc| accs[doc as usize].allowed = true))?;
    }
    if let Some(weight) = &plan.excluded {
        weight.for_each_no_score(segment, &mut |docs| docs.iter().for_each(|&doc| accs[doc as usize].excluded = true))?;
    }
    let (titles, alive) = (segment.get_fieldnorms_reader(plan.fields.title)?, segment.alive_bitset());
    for (doc, acc) in accs.iter().enumerate().map(|(doc, acc)| (doc as DocId, acc)) {
        if acc.any == 0 || alive.is_some_and(|alive| alive.is_deleted(doc)) {
            continue;
        }
        match titles.fieldnorm(doc) {
            0 => later.push((ord, doc)),
            title => found.extend(acc.rank(title, plan).map(|rank| Candidate { rank, ord, doc, joined: None })),
        }
    }
    Ok(accs)
}
