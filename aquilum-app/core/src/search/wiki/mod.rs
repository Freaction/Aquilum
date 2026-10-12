pub mod anchor;
pub mod backlinks;
pub mod edges;
pub mod outgoing;
pub mod parser;
pub mod rename;
pub mod repository;
pub mod resolver;
pub mod service;
pub mod target;

const LINK_LIST_LIMIT: usize = 200;

pub use backlinks::backlinks;
pub use edges::{for_each_link, read_documents};
pub use outgoing::outgoing_links;
pub use rename::{plan_for_rename, LinkRewrite};
pub use repository::{candidate_sources, index_document, open_schema, remove_document};
pub use resolver::resolve_many;
#[cfg(test)]
mod rename_tests;
