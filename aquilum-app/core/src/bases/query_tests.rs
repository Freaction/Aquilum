use crate::bases::{BaseDefinition, BaseError, BaseRow, BaseRows};
use serde_json::{json, Value};

fn row(path: &str, fields: Value) -> BaseRow {
    BaseRow {
        path: path.to_owned(),
        file_tags: match fields.get("tags") {
            Some(Value::Array(tags)) => tags
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            Some(Value::String(tag)) => vec![tag.clone()],
            _ => Vec::new(),
        },
        fields: fields.as_object().unwrap().clone(),
    }
}

fn paths(columns: &[crate::bases::BaseColumn]) -> Vec<&str> {
    columns
        .iter()
        .flat_map(|column| column.rows.iter().map(|row| row.path.as_str()))
        .collect()
}

#[test]
fn combines_global_and_view_filters_with_nested_boolean_mappings() {
    let base = BaseDefinition::parse(
        r#"filters:
  and:
    - 'project == "Alpha: #1"'
    - or:
      - 'file.hasTag("work")'
      - 'priority == "urgent"'
    - not:
      - 'status == "Done"'
      - 'archived == "true"'
views:
  - type: kanban
    filters: 'note["status"] == "Todo"'
    groupBy: {property: status}
"#,
    )
    .unwrap();
    let notes = vec![
        row(
            "yes.md",
            json!({"project":"Alpha: #1","tags":["work/sub"],"status":"Todo"}),
        ),
        row(
            "done.md",
            json!({"project":"Alpha: #1","tags":["work"],"status":"Done"}),
        ),
        row(
            "archived.md",
            json!({"project":"Alpha: #1","priority":"urgent","status":"Todo","archived":"true"}),
        ),
        row(
            "other.md",
            json!({"project":"other","tags":["work"],"status":"Todo"}),
        ),
    ];
    assert_eq!(
        paths(&BaseRows::evaluate(&base, 0, &notes).unwrap()),
        ["yes.md"]
    );
    assert_eq!(notes[0].fields["status"], "Todo");
}

#[test]
fn property_prefixes_and_quoted_literals_are_exact() {
    for filter in [
        "note.status == \"Todo: #1\"",
        "status == 'Todo: #1'",
        "note[\"status\"] == \"Todo: #1\"",
    ] {
        let raw = format!(
            "filters: {}\nviews: [{{type: kanban, groupBy: {{property: status}}}}]",
            serde_json::to_string(filter).unwrap()
        );
        let base = BaseDefinition::parse(&raw).unwrap();
        let notes = [
            row("yes.md", json!({"status":"Todo: #1"})),
            row("no.md", json!({"status":"todo: #1"})),
        ];
        assert_eq!(
            paths(&BaseRows::evaluate(&base, 0, &notes).unwrap()),
            ["yes.md"]
        );
    }
}

#[test]
fn tags_match_exact_or_nested_case_insensitively_but_not_prefixes() {
    let base = BaseDefinition::parse("filters: 'file.hasTag(\"#work\")'\nviews: [{type: kanban}]")
        .unwrap();
    let notes = [
        row("exact.md", json!({"tags":["WORK"]})),
        row("nested.md", json!({"tags":"work/project"})),
        row("prefix.md", json!({"tags":["working"]})),
        row("missing.md", json!({})),
    ];
    assert_eq!(
        paths(&BaseRows::evaluate(&base, 0, &notes).unwrap()),
        ["exact.md", "nested.md"]
    );
}

#[test]
fn configured_columns_keep_empty_groups_and_limit_visibility() {
    let base = BaseDefinition::parse(
        "views:\n- type: kanban\n  groupBy: {property: status}\n  groupOrder: [Todo, Done, null]",
    )
    .unwrap();
    let notes = [
        row("todo.md", json!({"status":"Todo"})),
        row("hidden.md", json!({"status":"Other"})),
        row("missing.md", json!({})),
    ];
    let columns = BaseRows::evaluate(&base, 0, &notes).unwrap();
    assert_eq!(
        columns.iter().map(|c| c.value.clone()).collect::<Vec<_>>(),
        vec![json!("Todo"), json!("Done"), Value::Null]
    );
    assert!(columns[1].rows.is_empty());
    assert_eq!(paths(&columns), ["todo.md", "missing.md"]);
    let hidden = BaseDefinition::parse(
        "views: [{type: kanban, groupBy: {property: status}, groupOrder: []}]",
    )
    .unwrap();
    assert!(BaseRows::evaluate(&hidden, 0, &notes).unwrap().is_empty());
}

#[test]
fn automatic_groups_use_direction_and_stable_unassigned_column() {
    let base = BaseDefinition::parse(
        "views: [{type: kanban, groupBy: {property: note.status, direction: DESC}}]",
    )
    .unwrap();
    let notes = [
        row("a.md", json!({"status":"A"})),
        row("b.md", json!({"status":"B"})),
        row("none.md", json!({})),
        row("empty.md", json!({"status":""})),
    ];
    let columns = BaseRows::evaluate(&base, 0, &notes).unwrap();
    assert_eq!(
        columns.iter().map(|c| c.value.clone()).collect::<Vec<_>>(),
        vec![json!("B"), json!("A"), Value::Null]
    );
    assert_eq!(columns[2].rows.len(), 2);
}

#[test]
fn stable_multi_property_sort_precedes_global_limit() {
    let base = BaseDefinition::parse("views:\n- type: kanban\n  groupBy: {property: status}\n  sort:\n  - {property: rank, direction: ASC}\n  - {property: file.name, direction: DESC}\n  limit: 3").unwrap();
    let notes = [
        row("first/same.md", json!({"status":"Todo","rank":1})),
        row("second/same.md", json!({"status":"Todo","rank":1})),
        row("z.md", json!({"status":"Todo","rank":1})),
        row("ten.md", json!({"status":"Done","rank":10})),
    ];
    assert_eq!(
        paths(&BaseRows::evaluate(&base, 0, &notes).unwrap()),
        ["z.md", "first/same.md", "second/same.md"]
    );
}

#[test]
fn scalar_equality_keeps_types_and_compares_numeric_values() {
    let notes = [
        row("typed.md", json!({"rank":2,"done":true})),
        row("string.md", json!({"rank":"2","done":"true"})),
        row(
            "wrong.md",
            json!({"rank":3,"done":false,"missing":"filled"}),
        ),
    ];
    for filter in ["rank == 2", "rank == 2.0", "done == true"] {
        let base =
            BaseDefinition::parse(&format!("filters: '{filter}'\nviews: [{{type: kanban}}]"))
                .unwrap();
        assert_eq!(
            paths(&BaseRows::evaluate(&base, 0, &notes).unwrap()),
            ["typed.md"]
        );
    }
    let base =
        BaseDefinition::parse("filters: 'missing == null'\nviews: [{type: kanban}]").unwrap();
    assert_eq!(
        paths(&BaseRows::evaluate(&base, 0, &notes).unwrap()),
        ["typed.md", "string.md"]
    );
}

#[test]
fn sorts_numbers_without_rounding_large_integers() {
    let base = BaseDefinition::parse("views: [{type: kanban, sort: [{property: rank}]}]").unwrap();
    let notes = [
        row("larger.md", json!({"rank":9007199254740993_u64})),
        row("large.md", json!({"rank":9007199254740992_u64})),
        row("ten.md", json!({"rank":10})),
        row("two.md", json!({"rank":2})),
        row("negative.md", json!({"rank":-1.5})),
    ];
    assert_eq!(
        paths(&BaseRows::evaluate(&base, 0, &notes).unwrap()),
        ["negative.md", "two.md", "ten.md", "large.md", "larger.md"]
    );
}

#[test]
fn supports_quoted_property_names_and_escaped_expression_strings() {
    for (filter, expected) in [
        (r#"note["a==b"] == "x\"y""#, "double.md"),
        (r#"note['a==b'] == 'x"y'"#, "double.md"),
        (r#"note['a==b'] == 'it\'s'"#, "single.md"),
    ] {
        let raw = format!(
            "filters: {}\nviews: [{{type: kanban}}]",
            serde_json::to_string(filter).unwrap()
        );
        let base = BaseDefinition::parse(&raw).unwrap();
        let notes = [
            row("double.md", json!({"a==b":"x\"y"})),
            row("single.md", json!({"a==b":"it's"})),
        ];
        assert_eq!(
            paths(&BaseRows::evaluate(&base, 0, &notes).unwrap()),
            [expected]
        );
    }
}

#[test]
fn configured_scalar_groups_keep_numeric_equivalence_and_boolean_types() {
    let base = BaseDefinition::parse(
        "views: [{type: kanban, groupBy: {property: stage}, groupOrder: [1, true, null]}]",
    )
    .unwrap();
    let notes = [
        row("float.md", json!({"stage":1.0})),
        row("bool.md", json!({"stage":true})),
        row("string.md", json!({"stage":"true"})),
    ];
    let columns = BaseRows::evaluate(&base, 0, &notes).unwrap();
    assert_eq!(paths(&columns), ["float.md", "bool.md"]);
    assert!(columns[2].rows.is_empty());
}

#[test]
fn file_properties_derive_from_path_without_mutation() {
    let base = BaseDefinition::parse("filters: 'file.ext == \"md\"'\nviews: [{type: kanban, groupBy: {property: file.folder}, sort: [{property: file.name}]}]").unwrap();
    let notes = [
        row("Work/z.md", json!({})),
        row("Work/a.md", json!({})),
        row("Work/a.base", json!({})),
    ];
    let columns = BaseRows::evaluate(&base, 0, &notes).unwrap();
    assert_eq!(columns[0].value, "Work");
    assert_eq!(paths(&columns), ["Work/a.md", "Work/z.md"]);
}

#[test]
fn rejects_unknown_filters_before_reading_any_rows_or_short_circuiting() {
    for filter in [
        "'status != \"Todo\"'",
        "'{status}'",
        "{xor: ['status == \"Todo\"']}",
        "{and: ['status == \"never\"', 'file.inFolder(\"Work\")']}",
        "{and: ['status == \"Todo\"'], or: []}",
        "{and: 'status == \"Todo\"'}",
        "'status == \"Todo\" || file.inFolder(\"Work\")'",
    ] {
        let base = BaseDefinition::parse(&format!("filters: {filter}\nviews: [{{type: kanban}}]"))
            .unwrap();
        assert!(
            matches!(
                BaseRows::evaluate(&base, 0, &[]),
                Err(BaseError::UnsupportedFilter(_))
            ),
            "{filter}"
        );
    }
}

#[test]
fn rejects_unsupported_properties_and_complex_grouping() {
    for property in ["formula.status", "file.mtime", "this.status"] {
        let base = BaseDefinition::parse(&format!(
            "views: [{{type: kanban, groupBy: {{property: {property}}}}}]"
        ))
        .unwrap();
        assert!(matches!(
            BaseRows::evaluate(&base, 0, &[]),
            Err(BaseError::UnsupportedProperty(_))
        ));
    }
    let base =
        BaseDefinition::parse("views: [{type: kanban, groupBy: {property: status}}]").unwrap();
    for value in [json!(["Todo", "Done"]), json!({"nested":"Todo"})] {
        assert!(matches!(
            BaseRows::evaluate(&base, 0, &[row("a.md", json!({"status":value}))]),
            Err(BaseError::InvalidGrouping(_))
        ));
    }
    assert!(matches!(
        BaseRows::evaluate(&base, 2, &[]),
        Err(BaseError::InvalidView(2))
    ));
}
