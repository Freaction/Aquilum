pub mod dates;
pub mod lists;
pub mod numbers;
pub mod periods;
pub mod text;
pub mod values;

use super::span;
use super::value::Value;

pub fn call(name: &str, arguments: Vec<Value>) -> Result<Value, String> {
    match name {
        "link" => values::link(arguments),
        "default" => Ok(values::default(arguments)),
        "choice" => Ok(values::choice(arguments)),
        "object" => Ok(values::object(arguments)),
        "typeof" => Ok(Value::Text(values::type_name(first(&arguments)).to_owned())),

        "length" => Ok(Value::Number(text::length(first(&arguments)) as f64)),
        "contains" => Ok(Value::Bool(text::contains(&arguments))),
        "containsword" => Ok(Value::Bool(text::contains_word(&arguments))),
        "lower" => Ok(text::text_of(first(&arguments), |value| value.to_lowercase())),
        "upper" => Ok(text::text_of(first(&arguments), |value| value.to_uppercase())),
        "join" => Ok(text::join(&arguments)),
        "split" => Ok(text::split(&arguments)),
        "replace" => Ok(text::replace(&arguments)),
        "substring" => Ok(text::substring(&arguments)),
        "startswith" => Ok(text::ends(&arguments, true)),
        "endswith" => Ok(text::ends(&arguments, false)),
        "padleft" => Ok(text::pad(&arguments, true)),
        "padright" => Ok(text::pad(&arguments, false)),
        "truncate" => Ok(text::truncate(&arguments)),

        "number" => Ok(match first(&arguments).as_number() {
            Some(number) => Value::Number(number),
            None => Value::Null,
        }),
        "string" => Ok(Value::Text(first(&arguments).text())),
        "round" => Ok(numbers::numeric(&arguments, |value| value.round())),
        "floor" => Ok(numbers::numeric(&arguments, f64::floor)),
        "ceil" => Ok(numbers::numeric(&arguments, f64::ceil)),
        "trunc" => Ok(numbers::numeric(&arguments, f64::trunc)),
        "abs" => Ok(numbers::numeric(&arguments, f64::abs)),
        "sum" => Ok(Value::Number(numbers::numbers(first(&arguments)).sum())),
        "product" => Ok(Value::Number(numbers::numbers(first(&arguments)).product())),
        "average" => Ok(numbers::average(first(&arguments))),
        "min" => Ok(numbers::extreme(first(&arguments), true)),
        "max" => Ok(numbers::extreme(first(&arguments), false)),

        "list" => Ok(Value::List(arguments)),
        "reverse" => Ok(lists::reverse(first(&arguments))),
        "sort" => Ok(lists::sorted(first(&arguments))),
        "unique" => Ok(lists::unique(first(&arguments))),
        "flat" => Ok(lists::flat(first(&arguments))),
        "slice" => Ok(lists::slice(&arguments)),
        "firstvalue" => Ok(lists::edge(first(&arguments), true)),
        "lastvalue" => Ok(lists::edge(first(&arguments), false)),
        "nonnull" => Ok(lists::nonnull(first(&arguments))),

        "date" => Ok(dates::date(first(&arguments))),
        "dur" => Ok(dates::duration_value(first(&arguments))),
        "dateformat" => Ok(dates::date_format(&arguments)),
        "striptime" => Ok(dates::striptime(first(&arguments))),

        "span" => Ok(periods::time_span(&arguments)),
        "bar" => Ok(periods::bar(first(&arguments))),
        "percent" => Ok(periods::percent(first(&arguments))),
        "remaining" => Ok(periods::remaining(first(&arguments))),
        "within" => Ok(periods::within(&arguments)),
        "count" => Ok(periods::count(&arguments)),
        "previous" => Ok(span::previous(first(&arguments)).unwrap_or(Value::Null)),
        "delta" => Ok(periods::delta(&arguments)),

        other => Err(format!("Нет функции «{other}»: есть {}", available())),
    }
}

pub(in crate::search::dataview) fn available() -> String {
    [
        "link", "default", "choice", "object", "typeof",
        "length", "contains", "containsword", "lower", "upper", "join", "split", "replace",
        "substring", "startswith", "endswith", "padleft", "padright", "truncate",
        "number", "string", "round", "floor", "ceil", "trunc", "abs", "sum", "product",
        "average", "min", "max",
        "list", "reverse", "sort", "unique", "flat", "slice", "firstvalue", "lastvalue",
        "nonnull", "filter", "map", "any", "all",
        "date", "dur", "dateformat", "striptime",
        "span", "bar", "percent", "remaining", "within", "count", "previous", "delta",
    ]
    .join(", ")
}

pub fn first(arguments: &[Value]) -> &Value {
    arguments.first().unwrap_or(&Value::Null)
}

pub(super) fn items(value: &Value) -> &[Value] {
    match value {
        Value::List(items) => items,
        other => std::slice::from_ref(other),
    }
}

#[cfg(test)]
mod tests {
    use super::call;
    use super::super::value::{Link, Value};

    fn text(value: &str) -> Value {
        Value::Text(value.to_owned())
    }

    fn run(name: &str, arguments: Vec<Value>) -> Value {
        call(name, arguments).expect("функция посчиталась")
    }

    #[test]
    fn link_takes_a_path_and_a_label() {
        let value = run("link", vec![text("Книги/Пороки.md"), text("Пять пороков")]);
        assert_eq!(
            value,
            Value::Link(Link {
                target: "Книги/Пороки.md".to_owned(),
                display: Some("Пять пороков".to_owned()),
            })
        );
    }

    #[test]
    fn link_keeps_the_target_and_replaces_only_the_label() {
        let existing = Value::Link(Link {
            target: "Книги/Пороки.md".to_owned(),
            display: Some("старая".to_owned()),
        });
        let value = run("link", vec![existing, text("новая")]);
        assert_eq!(value.text(), "новая");
    }

    #[test]
    fn link_without_a_label_falls_back_to_the_note_name() {
        let value = run("link", vec![text("Книги/Пороки.md"), Value::Null]);
        assert_eq!(value.text(), "Пороки");
    }

    #[test]
    fn default_replaces_an_empty_value_but_keeps_a_zero() {
        assert_eq!(run("default", vec![Value::Null, text("нет")]).text(), "нет");
        assert_eq!(run("default", vec![text(""), text("нет")]).text(), "нет");
        assert_eq!(
            run("default", vec![Value::List(Vec::new()), text("нет")]).text(),
            "нет"
        );
        assert_eq!(
            run("default", vec![Value::Number(0.0), text("нет")]),
            Value::Number(0.0),
            "ноль — это значение, а не пустота"
        );
        assert_eq!(
            run("default", vec![Value::Bool(false), text("нет")]),
            Value::Bool(false)
        );
    }

    #[test]
    fn contains_looks_inside_a_list_and_inside_text() {
        let tags = Value::List(vec![text("fiction"), text("classic")]);
        assert!(run("contains", vec![tags.clone(), text("fiction")]).truthy());
        assert!(run("contains", vec![tags, text("FICTION")]).truthy());
        assert!(run("contains", vec![text("Иван Ефремов"), text("ефремов")]).truthy());
        assert!(!run("contains", vec![Value::Null, text("а")]).truthy());
    }

    #[test]
    fn length_counts_items_of_a_list_and_characters_of_text() {
        assert_eq!(
            run("length", vec![Value::List(vec![text("а"), text("б")])]),
            Value::Number(2.0)
        );
        assert_eq!(run("length", vec![text("Пороки")]), Value::Number(6.0));
        assert_eq!(run("length", vec![Value::Null]), Value::Number(0.0));
    }

    #[test]
    fn join_uses_a_comma_unless_told_otherwise() {
        let list = Value::List(vec![text("а"), text("б")]);
        assert_eq!(run("join", vec![list.clone()]).text(), "а, б");
        assert_eq!(run("join", vec![list, text(" / ")]).text(), "а / б");
    }

    #[test]
    fn aggregates_ignore_empty_values() {
        let list = Value::List(vec![Value::Number(3.0), Value::Null, Value::Number(10.0)]);
        assert_eq!(run("sum", vec![list.clone()]), Value::Number(13.0));
        assert_eq!(run("product", vec![list.clone()]), Value::Number(30.0));
        assert_eq!(run("average", vec![list.clone()]), Value::Number(6.5));
        assert_eq!(run("min", vec![list.clone()]), Value::Number(3.0));
        assert_eq!(run("max", vec![list]), Value::Number(10.0));
        for name in ["min", "max", "average"] {
            assert_eq!(run(name, vec![Value::Null]), Value::Null, "{name}");
            assert_eq!(run(name, vec![Value::Number(7.0)]), Value::Number(7.0), "{name}");
        }
    }

    #[test]
    fn flat_and_slice_preserve_nested_values_and_range_edges() {
        let list = Value::List(vec![Value::List(vec![text("а"), Value::Null]), text("б"), Value::List(Vec::new())]);
        let flat = run("flat", vec![list]);
        assert_eq!(flat, Value::List(vec![text("а"), Value::Null, text("б")]));
        assert_eq!(run("slice", vec![flat.clone(), Value::Number(1.0), Value::Number(3.0)]), Value::List(vec![Value::Null, text("б")]));
        assert_eq!(run("slice", vec![flat, Value::Number(3.0), Value::Number(1.0)]), Value::List(Vec::new()));
        assert_eq!(run("slice", vec![text("а")]), Value::List(vec![text("а")]));
    }

    #[test]
    fn an_unknown_function_names_the_ones_that_exist() {
        let error = call("dataviewjs", Vec::new()).expect_err("такой функции нет");
        assert!(error.contains("dataviewjs"));
        assert!(error.contains("link"));
    }
}
