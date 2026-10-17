use super::{matches, selected, value_of, Filter};
use crate::search::fields::Field;

fn book() -> Vec<Field> {
    [("type", "book"), ("author", "Иван Ефремов"), ("status", "read"), ("Путь к файлу", "")]
        .map(|(key, text)| Field::scalar(key.to_owned(), text.to_owned()))
        .into()
}

fn filter(equals: &[(&str, &str)], has: &[&str], missing: &[&str]) -> Filter {
    let owned = |keys: &[&str]| keys.iter().map(|key| (*key).to_owned()).collect();
    Filter { equals: equals.iter().map(|(key, value)| ((*key).to_owned(), (*value).to_owned())).collect(), has: owned(has), missing: owned(missing) }
}

#[test]
fn equality_ignores_case_and_a_missing_key_never_matches() {
    assert!(matches(&book(), &filter(&[("TYPE", "Book")], &[], &[])));
    assert!(!matches(&book(), &filter(&[("type", "article")], &[], &[])));
    assert!(!matches(&book(), &filter(&[("rating", "5")], &[], &[])));
}

#[test]
fn empty_value_counts_as_missing_and_all_conditions_hold_together() {
    assert!(matches(&book(), &filter(&[], &[], &["Путь к файлу"])));
    assert!(!matches(&book(), &filter(&[], &["Путь к файлу"], &[])));
    assert!(matches(&book(), &filter(&[("type", "book")], &["author"], &["Путь к файлу", "rating"])));
    assert!(!matches(&book(), &filter(&[("type", "book")], &["rating"], &[])));
    assert!(matches(&[], &filter(&[], &[], &[])));
}

#[test]
fn only_requested_fields_come_back_by_any_case() {
    let fields = selected(&book(), &["author".to_owned(), "rating".to_owned()]);
    let object = fields.as_object().unwrap();
    assert_eq!(object.len(), 1, "несуществующее поле не превращается в null");
    assert_eq!(object["author"], "Иван Ефремов");
    assert!(selected(&book(), &[]).as_object().unwrap().is_empty());
    assert_eq!(value_of(&book(), "ПУТЬ К ФАЙЛУ"), Some(""));
}
