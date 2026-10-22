pub mod analyzer;
pub mod analysis;
pub mod created;
pub mod dataview;
pub mod error;
pub mod guest;
pub mod fields;
pub mod graph;
pub mod headings;
pub mod hits;
pub mod index;
pub mod index_document;
#[cfg(test)]
mod index_tests;
#[cfg(test)]
mod benches;
pub mod metadata;
pub mod models;
pub mod note_date;
pub mod pages;
pub mod paths;
pub mod progress;
pub mod query;
pub mod schema;
mod lookup;
pub mod service;
mod storage;
pub mod suggest;
pub mod sync;
pub mod tasks;
pub mod wiki;
pub mod worker;

pub use analyzer::ANALYZER_VERSION;

use std::sync::Arc;

pub use models::IndexRevision;

pub use service::SearchService;
pub type IndexNotifier = Arc<dyn Fn(IndexRevision) + Send + Sync>;
