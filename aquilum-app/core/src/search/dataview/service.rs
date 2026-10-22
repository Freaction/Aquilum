use super::ast::{BinaryOp, Clause, Expr, Query, Shape, Source, UnaryOp};
use super::constants::{OVER_NOTES_SECONDS, OWN_SECONDS};
use super::execute::{self, QueryOutput};
use super::rows::Row;
use super::value::Value;
use super::{parser, rows, source};
use crate::search::error::SearchError;
use crate::search::paths::canonical_path;
use crate::search::service::SearchService;
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::Path;

impl SearchService {
    pub fn run_dataview_query(
        &self,
        workspace: &str,
        origin: &str,
        text: &str,
        tz_offset_minutes: i64,
    ) -> Result<QueryOutput, SearchError> {
        crate::search::note_date::set_local_offset(tz_offset_minutes * 60);
        let query = parser::parse(text).map_err(|message| SearchError::Query { message })?;
        let over_notes = !matches!(query.source, Some(Source::Once));
        let rows = if over_notes {
            self.note_rows(workspace, origin, &query)?
        } else {
            Vec::new()
        };
        crate::search::note_date::take_now_used();
        let mut output =
            execute::run(&query, rows).map_err(|message| SearchError::Query { message })?;
        if output.refresh_seconds.is_none() && crate::search::note_date::take_now_used() {
            output.refresh_seconds =
                Some(if over_notes { OVER_NOTES_SECONDS } else { OWN_SECONDS });
        }
        Ok(output)
    }

    fn note_rows(
        &self,
        workspace: &str,
        origin: &str,
        query: &Query,
    ) -> Result<Vec<Row>, SearchError> {
        let origin = canonical_path(Path::new(origin));
        self.with_metadata(workspace, |root, connection| {
            let mut paths = source::resolve(connection, root, &origin, query.source.as_ref())?;
            if paths.is_empty() {
                return Ok(Vec::new());
            }

            if matches!(query.shape, Shape::Tasks { .. }) {
                paths = filter_task_paths(connection, paths, query)?;
                if paths.is_empty() {
                    return Ok(Vec::new());
                }
            } else {
                paths = filter_candidate_paths(connection, paths, query)?;
                if paths.is_empty() {
                    return Ok(Vec::new());
                }
            }

            let mut rows = rows::load(connection, root, &paths)?;
            if matches!(query.shape, Shape::Tasks { .. }) {
                rows::attach_tasks(connection, &mut rows, &paths)?;
            }
            Ok(rows)
        })
    }
}

fn filter_task_paths(
    connection: &Connection,
    paths: Vec<String>,
    query: &Query,
) -> Result<Vec<String>, SearchError> {
    let completed_status = task_completed_filter(query);
    let sql = match completed_status {
        Some(false) => "SELECT DISTINCT path FROM note_tasks WHERE done = 0",
        Some(true) => "SELECT DISTINCT path FROM note_tasks WHERE done = 1",
        None => "SELECT DISTINCT path FROM note_tasks",
    };
    let mut statement = connection.prepare(sql)?;
    let task_paths: HashSet<String> = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<_, _>>()?;
    Ok(paths.into_iter().filter(|path| task_paths.contains(path)).collect())
}

fn task_completed_filter(query: &Query) -> Option<bool> {
    for clause in &query.clauses {
        if let Clause::Where(expr) = clause {
            if let Some(status) = expr_completed_filter(expr) {
                return Some(status);
            }
        }
    }
    None
}

fn expr_completed_filter(expr: &Expr) -> Option<bool> {
    match expr {
        Expr::Unary { operator: UnaryOp::Not, operand } => {
            if is_completed_var(operand) {
                return Some(false);
            }
            None
        }
        Expr::Binary { operator: BinaryOp::Equal, left, right } => {
            if is_completed_var(left) {
                if let Expr::Literal(Value::Bool(b)) = **right {
                    return Some(b);
                }
            } else if is_completed_var(right) {
                if let Expr::Literal(Value::Bool(b)) = **left {
                    return Some(b);
                }
            }
            None
        }
        Expr::Binary { operator: BinaryOp::NotEqual, left, right } => {
            if is_completed_var(left) {
                if let Expr::Literal(Value::Bool(b)) = **right {
                    return Some(!b);
                }
            } else if is_completed_var(right) {
                if let Expr::Literal(Value::Bool(b)) = **left {
                    return Some(!b);
                }
            }
            None
        }
        Expr::Binary { operator: BinaryOp::And, left, right } => {
            expr_completed_filter(left).or_else(|| expr_completed_filter(right))
        }
        Expr::Variable(name) if is_completed_name(name) => Some(true),
        _ => None,
    }
}

fn is_completed_var(expr: &Expr) -> bool {
    matches!(expr, Expr::Variable(name) if is_completed_name(name))
}

fn is_completed_name(name: &str) -> bool {
    let lower = name.trim().to_lowercase();
    lower == "completed" || lower == "task.completed"
}

fn filter_candidate_paths(
    connection: &Connection,
    paths: Vec<String>,
    query: &Query,
) -> Result<Vec<String>, SearchError> {
    let mut equals = Vec::new();
    for clause in &query.clauses {
        if let Clause::Where(expr) = clause {
            collect_field_equals(expr, &mut equals);
        }
    }
    if equals.is_empty() {
        return Ok(paths);
    }
    let candidates = crate::search::fields::candidates(connection, &equals, &[])?;
    let candidate_set: HashSet<String> = candidates.into_iter().collect();
    Ok(paths.into_iter().filter(|path| candidate_set.contains(path)).collect())
}

fn collect_field_equals(expr: &Expr, equals: &mut Vec<(String, String)>) {
    match expr {
        Expr::Binary { operator: BinaryOp::Equal, left, right } => {
            if let (Expr::Variable(key), Expr::Literal(val)) = (&**left, &**right) {
                if is_frontmatter_field(key) {
                    if let Some(text) = literal_to_text(val) {
                        equals.push((key.clone(), text));
                    }
                }
            } else if let (Expr::Literal(val), Expr::Variable(key)) = (&**left, &**right) {
                if is_frontmatter_field(key) {
                    if let Some(text) = literal_to_text(val) {
                        equals.push((key.clone(), text));
                    }
                }
            }
        }
        Expr::Binary { operator: BinaryOp::And, left, right } => {
            collect_field_equals(left, equals);
            collect_field_equals(right, equals);
        }
        _ => {}
    }
}

fn is_frontmatter_field(name: &str) -> bool {
    let lower = name.trim().to_lowercase();
    lower != "file" && !lower.starts_with("file.") && lower != "item" && !lower.starts_with("item.")
}

fn literal_to_text(value: &Value) -> Option<String> {
    match value {
        Value::Text(t) => Some(t.clone()),
        Value::Number(n) => {
            if n.fract() == 0.0 {
                Some(format!("{:.0}", n))
            } else {
                Some(n.to_string())
            }
        }
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}
