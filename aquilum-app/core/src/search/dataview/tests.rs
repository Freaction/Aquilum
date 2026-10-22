use super::execute::{self, Cell, CellPart, QueryOutput};
use super::parser::parse;
use super::rows::Row;
use crate::search::fields::Field;
use crate::search::tasks::Task;

fn note(relative: &str, created: i64, fields: Vec<Field>) -> Row {
    let name = relative
        .rsplit('/')
        .next()
        .unwrap_or(relative)
        .strip_suffix(".md")
        .unwrap_or(relative)
        .to_owned();
    Row {
        source: format!("C:/vault/{relative}"),
        tasks: Vec::new(),
        relative: relative.to_owned(),
        name,
        folder: relative.rsplit_once('/').map(|(head, _)| head.to_owned()).unwrap_or_default(),
        created,
        modified: created,
        size: 100,
        fields,
    }
}

fn scalar(key: &str, value: &str) -> Field {
    Field::scalar(key.to_owned(), value.to_owned())
}

fn list(key: &str, items: &[&str]) -> Field {
    Field::list(
        key.to_owned(),
        items.iter().map(|item| (*item).to_owned()).collect(),
    )
}

fn run(query: &str, rows: Vec<Row>) -> QueryOutput {
    execute::run(&parse(query).expect("запрос разобран"), rows).expect("запрос посчитан")
}

fn plain(cell: &Cell) -> String {
    cell.parts
        .iter()
        .map(|part| match part {
            CellPart::Text { text } => text.clone(),
            CellPart::Link { text, .. } => text.clone(),
            CellPart::Progress { percent } => format!("{percent:.0}%"),
            CellPart::Check { done, .. } => (if *done { "[x] " } else { "[ ] " }).to_owned(),
        })
        .collect()
}

fn targets(cell: &Cell) -> Vec<String> {
    cell.parts
        .iter()
        .filter_map(|part| match part {
            CellPart::Link { target, .. } => Some(target.clone()),
            CellPart::Text { .. } | CellPart::Progress { .. } | CellPart::Check { .. } => None,
        })
        .collect()
}

const DAY: i64 = 86_400 * 1_000_000_000;

#[test]
fn the_backlinks_query_builds_labelled_links_newest_first() {
    let rows = vec![
        note("Заметки/Старая.md", DAY, vec![scalar("title", "Про доверие")]),
        note("Заметки/Новая.md", DAY * 3, vec![scalar("title", "Про конфликт")]),
        note("Заметки/Безымянная.md", DAY * 2, Vec::new()),
    ];
    let output = run(
        "TABLE WITHOUT ID\nlink(file.link, title) AS \"Ссылающиеся заметки\"\nSORT file.ctime DESC",
        rows,
    );

    assert_eq!(output.shape, "table");
    assert_eq!(output.columns, vec!["Ссылающиеся заметки"]);
    assert_eq!(output.width, 1);
    assert_eq!(output.total, 3);

    assert_eq!(plain(&output.rows[0]), "Про конфликт");
    assert_eq!(targets(&output.rows[0]), vec!["Заметки/Новая.md"]);
    assert_eq!(
        plain(&output.rows[1]),
        "Безымянная",
        "без поля title подписью становится имя заметки"
    );
    assert_eq!(plain(&output.rows[2]), "Про доверие");
}

#[test]
fn a_table_without_without_id_leads_with_the_note_itself() {
    let output = run(
        "TABLE title",
        vec![note("Книга.md", DAY, vec![scalar("title", "Пороки")])],
    );
    assert_eq!(output.columns, vec!["Заметка", "title"]);
    assert_eq!(output.width, 2);
    assert_eq!(targets(&output.rows[0]), vec!["Книга.md"]);
    assert_eq!(plain(&output.rows[1]), "Пороки");
}

#[test]
fn where_filters_by_a_frontmatter_field() {
    let rows = vec![
        note("Книга.md", DAY, vec![scalar("type", "book")]),
        note("Статья.md", DAY, vec![scalar("type", "article")]),
    ];
    let output = run("TABLE WITHOUT ID file.name WHERE type = \"BOOK\"", rows);
    assert_eq!(output.total, 1);
    assert_eq!(plain(&output.rows[0]), "Книга");
}

#[test]
fn sorting_uses_the_type_of_the_field_not_its_text() {
    let rows = vec![
        note("А.md", DAY, vec![scalar("rating", "5")]),
        note("Б.md", DAY, vec![scalar("rating", "10")]),
    ];
    let output = run("TABLE WITHOUT ID file.name SORT rating DESC", rows);
    assert_eq!(
        plain(&output.rows[0]),
        "Б",
        "десять больше пяти, хотя как текст «10» меньше «5»"
    );
}

#[test]
fn a_date_shaped_field_sorts_as_a_date() {
    let rows = vec![
        note("Раньше.md", DAY, vec![scalar("created", "07.10.2024")]),
        note("Позже.md", DAY, vec![scalar("created", "2024-11-01")]),
    ];
    let output = run("TABLE WITHOUT ID file.name SORT created DESC", rows);
    assert_eq!(plain(&output.rows[0]), "Позже");
}

#[test]
fn a_missing_field_sorts_to_the_end_in_both_directions() {
    let rows = vec![
        note("Пустая.md", DAY, Vec::new()),
        note("Полная.md", DAY, vec![scalar("rating", "3")]),
    ];
    assert_eq!(
        plain(&run("TABLE WITHOUT ID file.name SORT rating", rows.clone_rows()).rows[0]),
        "Полная"
    );
    assert_eq!(
        plain(&run("TABLE WITHOUT ID file.name SORT rating DESC", rows).rows[0]),
        "Полная"
    );
}

#[test]
fn tags_are_a_list_and_contains_looks_inside_it() {
    let rows = vec![
        note("Фантастика.md", DAY, vec![list("tags", &["книги", "фантастика"])]),
        note("Проза.md", DAY, vec![list("tags", &["книги"])]),
    ];
    let output = run(
        "TABLE WITHOUT ID join(file.tags, \" / \") WHERE contains(file.tags, \"фантастика\")",
        rows,
    );
    assert_eq!(output.total, 1);
    assert_eq!(plain(&output.rows[0]), "книги / фантастика");
}

#[test]
fn a_list_of_links_stays_clickable_part_by_part() {
    let output = run(
        "TABLE WITHOUT ID list(link(\"А.md\"), link(\"Б.md\"))",
        vec![note("Заметка.md", DAY, Vec::new())],
    );
    assert_eq!(targets(&output.rows[0]), vec!["А.md", "Б.md"]);
    assert_eq!(plain(&output.rows[0]), "А, Б");
}

#[test]
fn a_list_query_shows_notes_and_can_show_a_value_instead() {
    let rows = vec![note("Книга.md", DAY, vec![scalar("title", "Пороки")])];
    let plain_list = run("LIST", rows.clone_rows());
    assert_eq!(plain_list.shape, "list");
    assert_eq!(plain(&plain_list.rows[0]), "Книга");

    let valued = run("LIST WITHOUT ID title", rows);
    assert_eq!(plain(&valued.rows[0]), "Пороки");
}

#[test]
fn limit_and_the_hard_cap_both_report_what_was_dropped() {
    let rows = (0..3)
        .map(|index| note(&format!("Заметка {index}.md"), DAY, Vec::new()))
        .collect::<Vec<_>>();
    let output = run("LIST LIMIT 2", rows);
    assert_eq!(output.rows.len(), 2);
    assert_eq!(output.total, 2, "LIMIT обрезает до подсчёта итога");
    assert!(!output.truncated);
}

#[test]
fn an_unknown_file_property_says_which_ones_exist() {
    let error = execute::run(
        &parse("TABLE file.автор").expect("разобран"),
        vec![note("Книга.md", DAY, Vec::new())],
    )
    .expect_err("такого поля у файла нет");
    assert!(error.contains("автор"));
    assert!(error.contains("ctime"));
}

#[test]
fn title_names_the_output_and_leaves_a_field_named_title_alone() {
    let named = run(
        "TASK TITLE \"Невыполненные задачи\"",
        vec![note("Дела.md", 1, vec![scalar("title", "Ефремов")])],
    );
    assert_eq!(named.title.as_deref(), Some("Невыполненные задачи"));

    let field = run(
        "TABLE WITHOUT ID title",
        vec![note("Дела.md", 1, vec![scalar("title", "Ефремов")])],
    );
    assert_eq!(field.title, None);
    assert_eq!(field.columns, vec!["title".to_owned()]);
    assert_eq!(plain(&field.rows[0]), "Ефремов");
}

#[test]
fn default_fills_an_empty_field_in_a_column() {
    let rows = vec![note("Книга.md", DAY, vec![scalar("title", "")])];
    let output = run("TABLE WITHOUT ID default(title, file.name)", rows);
    assert_eq!(plain(&output.rows[0]), "Книга");
}

trait CloneRows {
    fn clone_rows(&self) -> Vec<Row>;
}

impl CloneRows for Vec<Row> {
    fn clone_rows(&self) -> Vec<Row> {
        self.iter()
            .map(|row| Row {
                source: row.source.clone(),
                tasks: row.tasks.clone(),
                relative: row.relative.clone(),
                name: row.name.clone(),
                folder: row.folder.clone(),
                created: row.created,
                modified: row.modified,
                size: row.size,
                fields: row.fields.clone(),
            })
            .collect()
    }
}


const HOME_STATS: &str = r#"TABLE WITHOUT ID
    item.Period AS "Стата заметок",
    item.Count AS "Всего",
    item.Diff AS "Динамика"
FROM ""
GROUP BY ""
FLATTEN length(filter(rows.file.ctime, (t) => t >= date(today))) AS c_today
FLATTEN length(filter(rows.file.ctime, (t) => t >= date(today) - dur(1 week))) AS c_week
FLATTEN length(filter(rows.file.ctime, (t) => t >= date(today) - dur(2 weeks) and t < date(today) - dur(1 week))) AS c_prevWeek
FLATTEN list(
    object("Period", "Сегодня", "Count", c_today, "Diff", "—"),
    object("Period", "Неделя", "Count", c_week, "Diff", choice(c_week - c_prevWeek > 0, "+" + (c_week - c_prevWeek), "—")),
    object("Period", "За всё время", "Count", length(rows), "Diff", "—")
) AS item"#;

fn today() -> i64 {
    crate::search::note_date::start_of_day(crate::search::note_date::now())
}

#[test]
fn the_recent_notes_query_formats_its_dates() {
    let rows = vec![
        note("Свежая.md", today(), vec![scalar("title", "Свежая мысль")]),
        note("Старая.md", today() - DAY * 30, vec![scalar("title", "Старая мысль")]),
    ];
    let output = run(
        "TABLE WITHOUT ID link(file.link, title) AS \"Последние заметки\", dateformat(file.ctime, \"d MMM\") AS \"Дата\" WHERE file.ctime >= date(today) - dur(1 week) SORT file.ctime DESC LIMIT 10",
        rows,
    );

    assert_eq!(output.total, 1, "заметка месячной давности не попала в неделю");
    assert_eq!(plain(&output.rows[0]), "Свежая мысль");
    assert_eq!(
        plain(&output.rows[1]),
        crate::search::dataview::dateformat::format(today(), "d MMM")
    );
}

#[test]
fn the_stats_query_groups_the_whole_vault_into_one_table() {
    let rows = vec![
        note("Сегодня.md", today(), Vec::new()),
        note("Вчера.md", today() - DAY, Vec::new()),
        note("Позавчера.md", today() - DAY * 2, Vec::new()),
        note("Десять дней назад.md", today() - DAY * 10, Vec::new()),
    ];
    let output = run(HOME_STATS, rows);

    assert_eq!(output.width, 3);
    assert_eq!(output.total, 3, "три строки сводки из одной группы");
    assert_eq!(plain(&output.rows[0]), "Сегодня");
    assert_eq!(plain(&output.rows[1]), "1");
    assert_eq!(plain(&output.rows[3]), "Неделя");
    assert_eq!(plain(&output.rows[4]), "3");
    assert_eq!(plain(&output.rows[5]), "+2", "на прошлой неделе была одна заметка");
    assert_eq!(plain(&output.rows[6]), "За всё время");
    assert_eq!(plain(&output.rows[7]), "4");
}

#[test]
fn a_group_keeps_the_key_and_counts_its_rows() {
    let rows = vec![
        note("Книги/Одна.md", DAY, Vec::new()),
        note("Книги/Две.md", DAY, Vec::new()),
        note("Мысли/Три.md", DAY, Vec::new()),
    ];
    let output = run("TABLE length(rows) AS \"Сколько\" GROUP BY file.folder", rows);

    assert_eq!(output.columns, vec!["Заметка", "Сколько"]);
    assert_eq!(output.total, 2);
    assert_eq!(plain(&output.rows[0]), "Книги");
    assert_eq!(plain(&output.rows[1]), "2");
    assert_eq!(plain(&output.rows[2]), "Мысли");
    assert_eq!(plain(&output.rows[3]), "1");
}

#[test]
fn flatten_spreads_a_list_field_into_rows() {
    let rows = vec![note("Книга.md", DAY, vec![list("tags", &["книги", "мысль"])])];
    let output = run("TABLE WITHOUT ID tag FROM \"\" FLATTEN file.tags AS tag SORT tag", rows);

    assert_eq!(output.total, 2);
    assert_eq!(plain(&output.rows[0]), "книги");
    assert_eq!(plain(&output.rows[1]), "мысль");
}

const HOME_PROGRESS: &str = r#"TABLE WITHOUT ID
    item.name AS "Период", bar(item.span) AS "Прогресс",
    percent(item.span) + "%" AS "Процент", remaining(item.span) AS "Осталось"
FROM once
FLATTEN date("2001-05-17") AS birth
FLATTEN list(
    object("name", "День", "span", day),
    object("name", "Неделя", "span", week),
    object("name", "Месяц", "span", month),
    object("name", season_name, "span", season),
    object("name", "Год", "span", year),
    object("name", "Жизнь", "span", span(birth, birth + dur(80 years)))
) AS item"#;

#[test]
fn the_progress_query_needs_no_notes_at_all() {
    let output = run(HOME_PROGRESS, Vec::new());

    assert_eq!(output.width, 4);
    assert_eq!(output.total, 6, "шесть периодов");
    assert_eq!(plain(&output.rows[0]), "День");
    assert_eq!(plain(&output.rows[20]), "Жизнь");

    let shares: Vec<f64> = output
        .rows
        .iter()
        .flat_map(|cell| cell.parts.iter())
        .filter_map(|part| match part {
            CellPart::Progress { percent } => Some(*percent),
            _ => None,
        })
        .collect();
    assert_eq!(shares.len(), 6, "в каждой строке своя полоса");
    assert!(shares.iter().all(|share| (0.0..=100.0).contains(share)));
    assert!(shares[5] > 20.0 && shares[5] < 60.0, "жизнь прожита примерно на треть");
}

#[test]
fn a_query_from_once_ignores_the_notes_it_was_given() {
    let output = run("TABLE WITHOUT ID 2 + 2 AS \"Сумма\" FROM once", vec![note("Лишняя.md", DAY, Vec::new())]);
    assert_eq!(output.total, 1);
    assert_eq!(plain(&output.rows[0]), "4");
}

#[test]
fn a_query_from_once_is_answered_without_an_index() {
    use crate::search::models::IndexRevision;
    use crate::search::service::SearchService;
    use std::path::PathBuf;
    use std::sync::Arc;

    let service = SearchService::new(
        PathBuf::from("."),
        Arc::new(|_: IndexRevision| {}),
    );
    let output = service
        .run_dataview_query("", "", "TABLE WITHOUT ID 2 + 2 AS \"Сумма\" FROM once", 0)
        .expect("запрос посчитан без открытого индекса");
    assert_eq!(output.total, 1);
    assert_eq!(plain(&output.rows[0]), "4");
}

#[test]
fn the_new_functions_work_on_real_values() {
    let rows = vec![note("Книга.md", DAY, vec![scalar("title", "Пять пороков команды")])];
    let output = run(
        "TABLE WITHOUT ID truncate(title, 10) AS \"Кратко\", upper(substring(title, 0, 4)) AS \"Начало\", length(split(title, \" \")) AS \"Слов\", containsword(title, \"пороков\") AS \"Есть\"",
        rows,
    );
    assert_eq!(plain(&output.rows[0]), "Пять поро…");
    assert_eq!(plain(&output.rows[1]), "ПЯТЬ");
    assert_eq!(plain(&output.rows[2]), "3");
    assert_eq!(plain(&output.rows[3]), "true");
}

const SHORT_STATS: &str = r#"TABLE WITHOUT ID
    item.name AS "Стата заметок",
    count(rows.file.ctime, item.span) AS "Всего",
    delta(count(rows.file.ctime, item.span), count(rows.file.ctime, previous(item.span))) AS "Динамика"
FROM ""
GROUP BY ""
FLATTEN list(
    object("name", "Сегодня", "span", span(today, now)),
    object("name", "Неделя", "span", span(today - dur(1 week), now)),
    object("name", "За всё время", "span", ever)
) AS item"#;

#[test]
fn the_short_stats_query_counts_the_same_as_the_long_one() {
    let rows = vec![
        note("Сегодня.md", today(), Vec::new()),
        note("Вчера.md", today() - DAY, Vec::new()),
        note("Позавчера.md", today() - DAY * 2, Vec::new()),
        note("Десять дней назад.md", today() - DAY * 10, Vec::new()),
    ];
    let output = run(SHORT_STATS, rows);

    assert_eq!(output.width, 3);
    assert_eq!(output.total, 3);
    assert_eq!(plain(&output.rows[0]), "Сегодня");
    assert_eq!(plain(&output.rows[1]), "1");
    assert_eq!(plain(&output.rows[3]), "Неделя");
    assert_eq!(plain(&output.rows[4]), "3");
    assert_eq!(plain(&output.rows[5]), "+2", "на прошлой неделе была одна заметка");
    assert_eq!(plain(&output.rows[6]), "За всё время");
    assert_eq!(plain(&output.rows[7]), "4");
    assert_eq!(plain(&output.rows[8]), "\u{2014}");
}

#[test]
fn a_delta_shows_direction_and_zero_or_dash() {
    let output = run(
        "TABLE WITHOUT ID delta(5, 3) AS \"Рост\", delta(3, 5) AS \"Спад\", delta(4, 4) AS \"Ровно\", delta(4, 4, \"—\") AS \"Прочерк\" FROM once",
        Vec::new(),
    );
    assert_eq!(plain(&output.rows[0]), "+2");
    assert_eq!(plain(&output.rows[1]), "-2");
    assert_eq!(plain(&output.rows[2]), "0");
    assert_eq!(plain(&output.rows[3]), "\u{2014}");
}

#[test]
fn a_period_boundary_belongs_to_one_side_only() {
    let rows = vec![note("Ровно в полночь.md", today(), Vec::new())];
    let output = run(
        "TABLE WITHOUT ID count(rows.file.ctime, span(today, now)) AS \"Сегодня\", count(rows.file.ctime, previous(span(today, now))) AS \"Вчера\" FROM \"\" GROUP BY \"\"",
        rows,
    );
    assert_eq!(plain(&output.rows[0]), "1");
    assert_eq!(plain(&output.rows[1]), "0", "конец отрезка в него не входит");
}

fn with_tasks(relative: &str, tasks: &[(bool, &str)]) -> Row {
    let mut row = note(relative, DAY, Vec::new());
    row.tasks = tasks
        .iter()
        .enumerate()
        .map(|(index, (done, text))| Task {
            line: index + 1,
            done: *done,
            text: (*text).to_owned(),
        })
        .collect();
    row
}

fn checkboxes(output: &QueryOutput) -> Vec<bool> {
    output
        .rows
        .iter()
        .flat_map(|cell| cell.parts.iter())
        .filter_map(|part| match part {
            CellPart::Check { done, .. } => Some(*done),
            _ => None,
        })
        .collect()
}

#[test]
fn a_task_query_lists_checkboxes_not_notes() {
    let rows = vec![
        with_tasks("Дела.md", &[(false, "написать"), (true, "отправить")]),
        with_tasks("Пусто.md", &[]),
    ];
    let output = run("TASK", rows);

    assert_eq!(output.shape, "tasks");
    assert_eq!(output.total, 2, "две задачи из одной заметки");
    assert_eq!(checkboxes(&output), vec![false, true]);
    assert_eq!(plain(&output.rows[0]), "[ ] написать");
    assert_eq!(targets(&output.rows[0]), vec!["Дела.md"]);
}

#[test]
fn a_condition_filters_the_tasks_themselves() {
    let rows = vec![with_tasks("Дела.md", &[(false, "написать"), (true, "отправить")])];
    let output = run("TASK WITHOUT ID WHERE !completed", rows);

    assert_eq!(output.total, 1);
    assert_eq!(plain(&output.rows[0]), "[ ] написать");
    assert!(targets(&output.rows[0]).is_empty(), "«WITHOUT ID» убирает ссылку на заметку");
}

#[test]
fn tasks_can_be_searched_sorted_and_cut() {
    let rows = vec![with_tasks(
        "Дела.md",
        &[(false, "бета"), (false, "альфа"), (false, "гамма")],
    )];
    let output = run("TASK WITHOUT ID WHERE contains(text, \"а\") SORT text LIMIT 2", rows);

    assert_eq!(output.total, 2);
    assert_eq!(plain(&output.rows[0]), "[ ] альфа");
    assert_eq!(plain(&output.rows[1]), "[ ] бета");
}

#[test]
fn a_task_knows_which_note_and_line_it_came_from() {
    let rows = vec![with_tasks("Дела.md", &[(false, "написать")])];
    let output = run("TASK WITHOUT ID WHERE line = 1 and file.name = \"Дела\"", rows);
    assert_eq!(output.total, 1);
}

const HOME_CLOCK: &str = r#"TABLE WITHOUT ID
    dateformat(now, "EEEE, d MMMM") AS "Сегодня",
    dateformat(now, "HH:mm:ss") AS "Сейчас"
FROM once
EVERY 1s"#;

const HOME_PERIODS: &str = r#"TABLE WITHOUT ID
    item.name AS "Период",
    bar(item.span) AS "Прогресс",
    percent(item.span) + "%" AS "Процент",
    remaining(item.span) AS "Осталось"
FROM once
EVERY 10s
FLATTEN date("2001-05-17") AS birth
FLATTEN list(
    object("name", "День", "span", day),
    object("name", "Неделя", "span", week),
    object("name", "Месяц", "span", month),
    object("name", season_name, "span", season),
    object("name", "Год", "span", year),
    object("name", "Жизнь", "span", span(birth, birth + dur(80 years)))
) AS item"#;

#[test]
fn the_home_page_blocks_keep_their_own_pace() {
    let clock = run(HOME_CLOCK, Vec::new());
    assert_eq!(clock.refresh_seconds, Some(1));
    assert_eq!(clock.total, 1);
    assert_eq!(clock.width, 2);

    let periods = run(HOME_PERIODS, Vec::new());
    assert_eq!(periods.refresh_seconds, Some(10));
    assert_eq!(periods.total, 6, "шесть периодов");
    assert_eq!(plain(&periods.rows[0]), "День");
}

#[test]
fn a_forgotten_comma_between_columns_is_named_as_such() {
    let error = parse("TABLE WITHOUT ID dateformat(now, \"HH:mm\") AS \"Сейчас\" item.name")
        .expect_err("запрос не разобран");
    assert!(error.contains("запятая"), "{error}");
}

#[test]
fn a_working_day_is_a_span_between_two_hours_of_today() {
    let output = run(
        r#"TABLE WITHOUT ID item.name AS "Период", percent(item.span) AS "Процент"
FROM once
FLATTEN list(
    object("name", "До конца работы", "span", span(today, today + dur(21 hours))),
    object("name", "Рабочий день", "span", span(today + dur(9 hours), today + dur(21 hours)))
) AS item"#,
        Vec::new(),
    );

    assert_eq!(output.total, 2);
    assert_eq!(plain(&output.rows[0]), "До конца работы");
    let share: f64 = plain(&output.rows[1]).parse().expect("процент — число");
    assert!((0.0..=100.0).contains(&share), "доля дня в пределах ста процентов: {share}");
}

#[test]
fn a_block_that_asks_the_clock_keeps_itself_fresh_without_every() {
    crate::search::note_date::take_now_used();
    run("TABLE WITHOUT ID 2 + 2 AS \"Сумма\" FROM once", Vec::new());
    assert!(!crate::search::note_date::take_now_used(), "арифметика часов не трогает");

    run("TABLE WITHOUT ID percent(day) AS \"День\" FROM once", Vec::new());
    assert!(crate::search::note_date::take_now_used(), "период спрашивает текущий момент");

    run("TABLE WITHOUT ID dateformat(now, \"HH:mm\") AS \"Часы\" FROM once", Vec::new());
    assert!(crate::search::note_date::take_now_used(), "now спрашивает текущий момент");
}

#[test]
fn every_sets_how_often_the_block_recounts_itself() {
    let rows = vec![note("Дела.md", 1, Vec::new())];
    assert_eq!(run("TABLE EVERY 30s", rows.clone_rows()).refresh_seconds, Some(30));
    assert_eq!(run("TASK EVERY 5 minutes", rows.clone_rows()).refresh_seconds, Some(300));
    assert_eq!(run("TABLE", rows).refresh_seconds, None);
}

#[test]
fn every_explains_a_wrong_interval_and_leaves_a_field_named_every_alone() {
    assert!(parse("TABLE EVERY 0s").unwrap_err().contains("секунды"));
    assert!(parse("TABLE EVERY 30 попугаев").unwrap_err().contains("промежуток"));
    assert!(parse("TABLE EVERY 30s EVERY 1 minute").unwrap_err().contains("дважды"));

    let field = run(
        "TABLE WITHOUT ID every",
        vec![note("Дела.md", 1, vec![scalar("every", "день")])],
    );
    assert_eq!(field.refresh_seconds, None);
    assert_eq!(plain(&field.rows[0]), "день");
}

#[test]
fn situation_01_delta_positive() {
    let output = run("TABLE WITHOUT ID delta(10, 4) AS \"D\" FROM once", Vec::new());
    assert_eq!(plain(&output.rows[0]), "+6");
}

#[test]
fn situation_02_delta_negative() {
    let output = run("TABLE WITHOUT ID delta(4, 10) AS \"D\" FROM once", Vec::new());
    assert_eq!(plain(&output.rows[0]), "-6");
}

#[test]
fn situation_03_delta_minus_one() {
    let output = run("TABLE WITHOUT ID delta(0, 1) AS \"D\" FROM once", Vec::new());
    assert_eq!(plain(&output.rows[0]), "-1");
}

#[test]
fn situation_04_delta_zero_shows_zero() {
    let output = run("TABLE WITHOUT ID delta(7, 7) AS \"D\" FROM once", Vec::new());
    assert_eq!(plain(&output.rows[0]), "0");
}

#[test]
fn situation_05_delta_zero_custom_dash() {
    let output = run("TABLE WITHOUT ID delta(7, 7, \"—\") AS \"D\" FROM once", Vec::new());
    assert_eq!(plain(&output.rows[0]), "\u{2014}");
}

#[test]
fn situation_06_delta_zero_custom_text() {
    let output = run("TABLE WITHOUT ID delta(7, 7, \"ровно\") AS \"D\" FROM once", Vec::new());
    assert_eq!(plain(&output.rows[0]), "ровно");
}

#[test]
fn situation_07_delta_null_before() {
    let output = run("TABLE WITHOUT ID delta(10, null) AS \"D\" FROM once", Vec::new());
    assert_eq!(plain(&output.rows[0]), "\u{2014}");
}

#[test]
fn situation_08_delta_null_before_custom() {
    let output = run("TABLE WITHOUT ID delta(10, null, \"нет данных\") AS \"D\" FROM once", Vec::new());
    assert_eq!(plain(&output.rows[0]), "нет данных");
}

#[test]
fn situation_09_delta_null_current() {
    let output = run("TABLE WITHOUT ID delta(null, 10) AS \"D\" FROM once", Vec::new());
    assert_eq!(plain(&output.rows[0]), "\u{2014}");
}

#[test]
fn situation_10_delta_both_null() {
    let output = run("TABLE WITHOUT ID delta(null, null) AS \"D\" FROM once", Vec::new());
    assert_eq!(plain(&output.rows[0]), "\u{2014}");
}

#[test]
fn situation_11_previous_ever_is_null() {
    let output = run("TABLE WITHOUT ID previous(ever) AS \"P\" FROM once", Vec::new());
    assert_eq!(plain(&output.rows[0]), "");
}

#[test]
fn situation_12_previous_zero_duration_is_null() {
    let output = run("TABLE WITHOUT ID previous(span(today, today)) AS \"P\" FROM once", Vec::new());
    assert_eq!(plain(&output.rows[0]), "");
}

#[test]
fn situation_13_count_list_without_period() {
    let output = run("TABLE WITHOUT ID count(list(1, 2, 3, 4, 5)) AS \"C\" FROM once", Vec::new());
    assert_eq!(plain(&output.rows[0]), "5");
}

#[test]
fn situation_14_count_empty_list_without_period() {
    let output = run("TABLE WITHOUT ID count(list()) AS \"C\" FROM once", Vec::new());
    assert_eq!(plain(&output.rows[0]), "0");
}

#[test]
fn situation_15_count_with_null_period_returns_null() {
    let output = run("TABLE WITHOUT ID count(list(today, today), null) AS \"C\" FROM once", Vec::new());
    assert_eq!(plain(&output.rows[0]), "");
}

#[test]
fn situation_16_count_with_previous_ever_returns_null() {
    let output = run("TABLE WITHOUT ID count(list(today), previous(ever)) AS \"C\" FROM once", Vec::new());
    assert_eq!(plain(&output.rows[0]), "");
}

#[test]
fn situation_17_delta_with_count_and_previous_ever_shows_dash() {
    let output = run(
        "TABLE WITHOUT ID delta(count(list(today), ever), count(list(today), previous(ever))) AS \"D\" FROM once",
        Vec::new(),
    );
    assert_eq!(plain(&output.rows[0]), "\u{2014}");
}

#[test]
fn situation_18_count_period_boundary() {
    let t = today();
    let rows = vec![
        note("Начало.md", t, Vec::new()),
        note("Конец.md", t + DAY, Vec::new()),
    ];
    let output = run(
        "TABLE WITHOUT ID count(rows.file.ctime, span(today, today + dur(1 day))) AS \"C\" FROM \"\" GROUP BY \"\"",
        rows,
    );
    assert_eq!(plain(&output.rows[0]), "1");
}

#[test]
fn situation_19_within_period() {
    let output = run(
        "TABLE WITHOUT ID within(today, day) AS \"In\", within(today - dur(2 days), day) AS \"Out\" FROM once",
        Vec::new(),
    );
    assert_eq!(plain(&output.rows[0]), "true");
    assert_eq!(plain(&output.rows[1]), "false");
}

#[test]
fn situation_20_remaining_past_shows_finished() {
    let output = run(
        "TABLE WITHOUT ID remaining(span(today - dur(2 days), today - dur(1 day))) AS \"R\" FROM once",
        Vec::new(),
    );
    assert_eq!(plain(&output.rows[0]), "конец");
}

#[test]
fn situation_21_bar_progress_clamps() {
    let output = run(
        "TABLE WITHOUT ID bar(150) AS \"Over\", bar(-20) AS \"Under\" FROM once",
        Vec::new(),
    );
    assert_eq!(plain(&output.rows[0]), "100%");
    assert_eq!(plain(&output.rows[1]), "0%");
}

#[test]
fn situation_22_home_stats_table_all_five_rows() {
    let t = today();
    let rows = vec![
        note("Сегодня1.md", t, Vec::new()),
        note("Сегодня2.md", t + 100, Vec::new()),
        note("Вчера.md", t - 10_000_000_000, Vec::new()),
        note("Прошлая неделя.md", t - DAY * 5, Vec::new()),
        note("Две недели назад.md", t - DAY * 10, Vec::new()),
        note("Полгода назад.md", t - DAY * 150, Vec::new()),
        note("Два года назад.md", t - DAY * 500, Vec::new()),
    ];
    let query = r#"TABLE WITHOUT ID
        item.name AS "Стата заметок",
        count(rows.file.ctime, item.span) AS "Всего",
        delta(count(rows.file.ctime, item.span),
              count(rows.file.ctime, previous(item.span))) AS "Динамика"
    FROM ""
    GROUP BY ""
    FLATTEN list(
        object("name", "Сегодня", "span", span(today, now)),
        object("name", "Неделя", "span", span(today - dur(1 week), now)),
        object("name", "Месяц", "span", span(today - dur(1 month), now)),
        object("name", "Год", "span", span(today - dur(1 year), now)),
        object("name", "За всё время", "span", ever)
    ) AS item"#;

    let output = run(query, rows);
    assert_eq!(output.width, 3);
    assert_eq!(output.total, 5);

    assert_eq!(plain(&output.rows[0]), "Сегодня");
    assert_eq!(plain(&output.rows[1]), "2");
    assert_eq!(plain(&output.rows[2]), "+1");

    assert_eq!(plain(&output.rows[3]), "Неделя");
    assert_eq!(plain(&output.rows[4]), "4");
    assert_eq!(plain(&output.rows[5]), "+3");

    assert_eq!(plain(&output.rows[6]), "Месяц");
    assert_eq!(plain(&output.rows[7]), "5");
    assert_eq!(plain(&output.rows[8]), "+5");

    assert_eq!(plain(&output.rows[9]), "Год");
    assert_eq!(plain(&output.rows[10]), "6");
    assert_eq!(plain(&output.rows[11]), "+5");

    assert_eq!(plain(&output.rows[12]), "За всё время");
    assert_eq!(plain(&output.rows[13]), "7");
    assert_eq!(plain(&output.rows[14]), "\u{2014}");
}
