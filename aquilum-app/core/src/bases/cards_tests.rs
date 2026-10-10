use super::{note, BaseDefinition};
use crate::app_core::{Core, CoreEvent};
use serde_json::json;
use std::fs;
use std::sync::{Arc, Mutex};

fn board(filters: &str) -> BaseDefinition {
    BaseDefinition::parse(&format!("{filters}\nviews:\n  - type: kanban\n    groupBy:\n      property: status\n    groupOrder: [todo, done, null]\n")).unwrap()
}

#[test]
fn base_note_types_and_tags_are_distinct() {
    let fields = note::fields("---\nrank: \"2\"\ndone: 'true'\nnumber: 2\nboolean: true\ntags: [board/nested]\ncomplex: {nested: 1}\n---\n#body/tag `#code` \\#escaped\n```\n#fenced\n```\n").unwrap();
    assert_eq!(fields["rank"], json!("2"));
    assert_eq!(fields["done"], json!("true"));
    assert_eq!(fields["number"], json!(2));
    assert_eq!(fields["boolean"], json!(true));
    assert_eq!(fields["tags"], json!(["board/nested"]));
    assert_eq!(
        note::tags(
            "---\ntags: [board/nested]\n---\n#body/tag `#code` \\#escaped\n```\n#fenced\n```\n",
            &fields
        ),
        vec!["board/nested", "body/tag"]
    );
    assert!(fields["complex"].is_object());
    assert!(note::fields("---\nx: [broken\n---\n").is_err());
}

#[test]
fn frontmatter_detection_requires_an_exact_delimiter_line() {
    for text in ["---abc\nbody\n", "----\nbody\n"] {
        assert!(note::fields(text).unwrap().is_empty(), "{text:?}");
    }
    assert!(note::fields(" ---\r\nstatus: todo\r\n").is_err());
}

#[test]
fn file_tag_filters_include_body_and_nested_tags_while_note_tags_stay_yaml_only() {
    let text = "---\ntags: yaml\n---\n#work/nested #other\n";
    let fields = note::fields(text).unwrap();
    let row = super::BaseRow {
        path: "a.md".into(),
        file_tags: note::tags(text, &fields),
        fields,
    };
    for (filter, expected) in [
        ("file.hasTag(\"work\")", 1),
        ("file.hasTag(\"wor\")", 0),
        ("note.tags == \"yaml\"", 1),
        ("note.tags == \"work\"", 0),
    ] {
        let definition = board(&format!("filters: '{filter}'"));
        let count: usize = super::BaseRows::evaluate(&definition, 0, std::slice::from_ref(&row))
            .unwrap()
            .iter()
            .map(|column| column.rows.len())
            .sum();
        assert_eq!(count, expected, "{filter}");
    }
}

#[test]
fn card_moves_preserve_other_fields_body_and_crlf_and_update_open_note() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("vault");
    fs::create_dir(&workspace).unwrap();
    let path = workspace.join("card.md");
    let before = "---\r\nstatus: todo\r\nother: '2' # keep\r\n---\r\nBody\r\n";
    fs::write(&path, before).unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = events.clone();
    let core = Core::open(
        &dir.path().join("data"),
        Arc::new(move |event: CoreEvent| captured.lock().unwrap().push(event.name())),
    );
    core.documents.open(&core, &path, 0).unwrap();
    core.move_base_card(
        &workspace,
        &board(""),
        0,
        &path,
        &json!("todo"),
        &json!("done"),
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        before.replace("status: todo", "status: \"done\"")
    );
    let opened = core.documents.read(&core, &path).unwrap();
    assert!(opened.text.contains("status: \"done\""));
    assert!(events.lock().unwrap().contains(&"document-changed"));
}

#[test]
fn card_writes_reject_complex_groups_and_ambiguous_membership() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("card.md");
    let core = Core::open(&dir.path().join("data"), Arc::new(|_: CoreEvent| {}));
    for text in [
        "---\nstatus: [todo]\n---\n",
        "---\nstatus: {value: todo}\n---\n",
        "---\nstatus: [broken\n---\n",
    ] {
        fs::write(&path, text).unwrap();
        assert!(core
            .move_base_card(
                dir.path(),
                &board(""),
                0,
                &path,
                &json!("todo"),
                &json!("done")
            )
            .is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
    }
    for filter in [
        "filters:\n  and: ['kind == \"task\"']",
        "filters: 'status == \"todo\"'",
        "filters: 'file.folder == \"tasks\"'",
    ] {
        assert!(core
            .create_base_card(dir.path(), &board(filter), 0, &path, "", &json!("done"))
            .is_err());
    }
}

#[test]
fn mutability_rejects_compound_overlapping_and_derived_grouping() {
    assert!(board("").can_mutate_group(0));
    assert!(board("filters: 'kind == \"task\"'").can_mutate_group(0));
    for filter in [
        "filters:\n  or: ['kind == \"task\"']",
        "filters: 'status == \"todo\"'",
        "filters: 'file.folder == \"tasks\"'",
    ] {
        assert!(!board(filter).can_mutate_group(0));
    }
    let mut definition = board("filters: 'kind == \"task\"'");
    definition.views[0].filters = Some(serde_yaml_ng::Value::String("kind == \"task\"".into()));
    assert!(!definition.can_mutate_group(0));
    for property in ["file.tags", "file.folder", "formula.status", "tags"] {
        let definition = BaseDefinition::parse(&format!(
            "views: [{{type: kanban, groupBy: {{property: {property}}}}}]"
        ))
        .unwrap();
        assert!(!definition.can_mutate_group(0));
    }
    assert!(!board("").can_mutate_group(99));
}

#[test]
fn creation_serializes_yaml_delimiters_and_uses_exact_configured_tag() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("card.md");
    let core = Core::open(&dir.path().join("data"), Arc::new(|_: CoreEvent| {}));
    let value = json!("a: b # c [d]\nnext");
    let mut definition = board("filters: 'file.hasTag(\"board\")'");
    definition.views[0].group_order = None;
    core.create_base_card(dir.path(), &definition, 0, &path, "# Body\n", &value)
        .unwrap();
    let text = fs::read_to_string(&path).unwrap();
    let fields = note::fields(&text).unwrap();
    assert_eq!(fields["status"], value);
    assert_eq!(fields["tags"], json!(["board"]));
    assert!(text.ends_with("# Body\n"));
    assert!(core
        .create_base_card(
            dir.path(),
            &board(""),
            0,
            &path,
            "overwritten",
            &json!("done")
        )
        .is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), text);
}

#[test]
fn property_edits_handle_quoted_keys_and_reject_alias_side_effects() {
    let text = "---\n\"state:name\": todo\nother: untouched\n---\nbody\n";
    let updated = super::card_metadata::set(text, "state:name", &json!("done")).unwrap();
    assert_eq!(updated, text.replace("todo", "\"done\""));
    let text = "---\nstatus: &group todo\nother: *group\n---\nbody\n";
    assert!(super::card_metadata::set(text, "status", &json!("done")).is_err());
    let duplicate = "---\nstatus: todo\nstatus: done\n---\nbody\n";
    assert!(super::card_metadata::set(duplicate, "status", &json!("done")).is_err());
}

#[test]
fn exact_tag_creation_keeps_nested_tags_and_scalar_membership_is_typed() {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::open(&dir.path().join("data"), Arc::new(|_: CoreEvent| {}));
    let path = dir.path().join("tag.md");
    core.create_base_card(
        dir.path(),
        &board("filters: 'file.hasTag(\"Work\")'"),
        0,
        &path,
        "---\ntags: [work/nested, elsewhere]\n---\n#body\n",
        &json!("todo"),
    )
    .unwrap();
    assert_eq!(
        note::fields(&fs::read_to_string(path).unwrap()).unwrap()["tags"],
        json!(["work/nested", "elsewhere", "Work"])
    );
    for (filter, expected) in [
        ("filters: 'rank == 2'", json!(2)),
        ("filters: 'rank == \"2\"'", json!("2")),
    ] {
        let path = dir.path().join(format!(
            "{}.md",
            if expected.is_number() {
                "number"
            } else {
                "text"
            }
        ));
        core.create_base_card(dir.path(), &board(filter), 0, &path, "", &json!("done"))
            .unwrap();
        assert_eq!(
            note::fields(&fs::read_to_string(path).unwrap()).unwrap()["rank"],
            expected
        );
    }
}

#[test]
fn retry_keeps_an_external_body_edit_and_conflicts_on_a_second_change() {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::open(&dir.path().join("data"), Arc::new(|_: CoreEvent| {}));
    let path = dir.path().join("card.md");
    fs::write(&path, "---\nstatus: todo\n---\ninitial\n").unwrap();
    core.move_base_card_with(
        dir.path(),
        &board(""),
        0,
        &path,
        &json!("todo"),
        &json!("done"),
        |attempt, path| {
            if attempt == 0 {
                fs::write(path, "---\nstatus: todo\n---\nexternal\n").unwrap();
            }
        },
    )
    .unwrap();
    assert!(fs::read_to_string(&path).unwrap().ends_with("external\n"));
    fs::write(&path, "---\nstatus: todo\n---\ninitial\n").unwrap();
    let result = core.move_base_card_with(
        dir.path(),
        &board(""),
        0,
        &path,
        &json!("todo"),
        &json!("done"),
        |attempt, path| {
            fs::write(
                path,
                format!("---\nstatus: todo\n---\nexternal {attempt}\n"),
            )
            .unwrap();
        },
    );
    assert!(matches!(
        result,
        Err(super::BaseCardError::File(
            crate::files::error::FileCommandError::Conflict { .. }
        ))
    ));
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "---\nstatus: todo\n---\nexternal 1\n"
    );
}

#[test]
fn retry_does_not_overwrite_an_external_group_change_or_lost_membership() {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::open(&dir.path().join("data"), Arc::new(|_: CoreEvent| {}));
    let path = dir.path().join("card.md");
    let initial = "---\nstatus: todo\nkind: task\n---\nbody\n";
    for external in [
        initial.replace("todo", "other"),
        initial.replace("kind: task", "kind: memo"),
    ] {
        fs::write(&path, initial).unwrap();
        let result = core.move_base_card_with(
            dir.path(),
            &board("filters: 'kind == \"task\"'"),
            0,
            &path,
            &json!("todo"),
            &json!("done"),
            |_, path| fs::write(path, &external).unwrap(),
        );
        if external.contains("status: other") {
            assert!(matches!(
                result,
                Err(super::BaseCardError::GroupConflict { .. })
            ));
        } else {
            assert!(result.is_err());
        }
        assert_eq!(fs::read_to_string(&path).unwrap(), external);
    }
}
