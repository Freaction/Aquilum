//! Строки интерфейса — локали `assets/locales/*.json` и те же правила, что у Tauri-версии: язык `ru`, если так в настройках, иначе `en`; ключ без
//! перевода ищется в английской локали, потом возвращается как есть; `{{имя}}` подставляется.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::Value;

static RUSSIAN: AtomicBool = AtomicBool::new(false);

fn locale(russian: bool) -> &'static Value {
    static RU: OnceLock<Value> = OnceLock::new();
    static EN: OnceLock<Value> = OnceLock::new();
    let (cell, source) = if russian {
        (&RU, include_str!("../assets/locales/ru.json"))
    } else {
        (&EN, include_str!("../assets/locales/en.json"))
    };
    cell.get_or_init(|| serde_json::from_str(source).expect("локаль фронтенда — корректный JSON"))
}

/// Язык по настройке `ui.language`, как `resolveLocale`.
pub fn set_language(setting: &str) {
    RUSSIAN.store(setting == "ru", Ordering::Relaxed);
}

fn lookup(russian: bool, key: &str) -> Option<&'static str> {
    key.split('.').try_fold(locale(russian), |node, part| node.get(part))?.as_str()
}

fn translate(key: &str) -> Option<&'static str> {
    lookup(RUSSIAN.load(Ordering::Relaxed), key).or_else(|| lookup(false, key))
}

/// Строка по ключу.
pub fn is_russian() -> bool {
    RUSSIAN.load(Ordering::Relaxed)
}

pub fn t(key: &str) -> String {
    translate(key).unwrap_or(key).to_owned()
}

/// Строка по ключу с подстановкой `{{имя}}`.
pub fn t_with(key: &str, params: &[(&str, &str)]) -> String {
    let mut text = t(key);
    for (name, value) in params {
        text = text.replace(&format!("{{{{{name}}}}}"), value);
    }
    text
}

/// Форма множественного числа (`.one`, `.few`, `.many`, `.other`) с `{{count}}`.
pub fn plural(key: &str, count: u64) -> String {
    let form = if RUSSIAN.load(Ordering::Relaxed) {
        // CLDR для целых: 1, 21, 31… — one; 2–4, 22–24… — few; остальное — many.
        match (count % 10, count % 100) {
            (1, n) if n != 11 => "one",
            (2..=4, n) if !(12..=14).contains(&n) => "few",
            _ => "many",
        }
    } else if count == 1 {
        "one"
    } else {
        "other"
    };
    let template = translate(&format!("{key}.{form}")).or_else(|| translate(&format!("{key}.other"))).unwrap_or(key);
    template.replace("{{count}}", &count.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn russian_plurals_and_params() {
        set_language("ru");
        assert_eq!(plural("fileTree.dragCount", 1), "1 заметка");
        assert_eq!(plural("fileTree.dragCount", 3), "3 заметки");
        assert_eq!(plural("fileTree.dragCount", 12), "12 заметок");
        assert_eq!(t_with("confirm.deleteMany", &[("count", "5")]), "Удалить 5");
        assert_eq!(t("нет.такого.ключа"), "нет.такого.ключа");
        set_language("");
        assert_eq!(t("common.delete"), "Delete");
    }
}
