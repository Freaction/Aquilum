//! The Aquilum core: notes on disk, documents, search, history, settings and UI state, with no
//! knowledge of the UI that hosts it.

pub mod app_core;
pub mod documents;
pub mod files;
pub mod history;
mod http;
pub mod link_title;
pub mod markdown;
pub mod mcp;
pub mod recent;
pub mod migration;
pub mod search;
pub mod settings;
pub mod sync;
pub mod text;
pub mod ui_state;
pub mod web_agent;
pub mod wikixiv;

pub use app_core::{Core, CoreEvent, EventSink, TaskFailed};
