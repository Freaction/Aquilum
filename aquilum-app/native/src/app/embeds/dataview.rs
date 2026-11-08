use aquilum_core::search::dataview::execute::{CellPart, QueryOutput};

use super::Inner;
use aquilum_core::search::error::SearchError;
use crate::ui::editor::resolve::{Fetch, Grid, Part, Shape};

fn parts(cell: Vec<CellPart>) -> Vec<Part> {
    cell.into_iter()
        .map(|p| match p {
            CellPart::Text { text } => Part::Text(text),
            CellPart::Link { text, target } => Part::Link { text, target },
            CellPart::Progress { percent } => Part::Progress(percent),
            CellPart::Check { done, target, line } => Part::Check { done, target, line },
        })
        .collect()
}

fn grid(output: QueryOutput) -> Grid {
    let shape = match output.shape {
        "list" => Shape::List,
        "tasks" => Shape::Tasks,
        _ => Shape::Table,
    };
    let width = output.width.max(1);
    let mut cells = output.rows.into_iter();
    let mut rows = Vec::with_capacity(cells.len().div_ceil(width));
    while cells.len() > 0 {
        rows.push(cells.by_ref().take(width).map(|cell| parts(cell.parts)).collect());
    }
    Grid { shape, title: output.title, columns: output.columns, rows: rows.into() }
}

pub(super) fn run(inner: &Inner, query: &str) -> (Fetch<Grid>, Option<u32>) {
    let offset = crate::dates::offset_seconds() / 60;
    let (workspace, document) = (inner.workspace(), inner.source());
    match inner.core.search.run_dataview_query(&workspace, &document, query, offset) {
        Ok(output) => {
            let refresh = output.refresh_seconds;
            (Fetch::Ready(grid(output)), refresh)
        }
        Err(SearchError::Unavailable { .. }) => (Fetch::Pending, None),
        Err(error) => (Fetch::Missing(format!("{error}")), None),
    }
}
