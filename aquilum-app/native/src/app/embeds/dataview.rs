use aquilum_core::search::dataview::execute::{CellPart, QueryOutput};

use super::{Inner, indexed};
use crate::ui::editor::resolve::{Fetch, Grid, Part, Shape};

fn parts(cell: &[CellPart]) -> Vec<Part> {
    cell.iter()
        .map(|p| match p {
            CellPart::Text { text } => Part::Text(text.clone()),
            CellPart::Link { text, target } => Part::Link { text: text.clone(), target: target.clone() },
            CellPart::Progress { percent } => Part::Progress(*percent),
            CellPart::Check { done, target, line } => Part::Check { done: *done, target: target.clone(), line: *line },
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
    let cells: Vec<Vec<Part>> = output.rows.iter().map(|c| parts(&c.parts)).collect();
    let rows = cells.chunks(width).map(<[Vec<Part>]>::to_vec).collect();
    Grid { shape, title: output.title, columns: output.columns, rows }
}

pub(super) fn run(inner: &Inner, query: &str) -> (Fetch<Grid>, Option<u32>) {
    let offset = i64::from(chrono::Local::now().offset().local_minus_utc() / 60);
    let (workspace, document) = (inner.workspace(), inner.source());
    match indexed(|| inner.core.search.run_dataview_query(&workspace, &document, query, offset)) {
        Ok(output) => {
            let refresh = output.refresh_seconds;
            (Fetch::Ready(grid(output)), refresh)
        }
        Err(error) => (Fetch::Missing(format!("{error}")), None),
    }
}
