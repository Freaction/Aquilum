use std::cmp::Ordering;

pub type Key = (String, String, String);

pub fn key(text: &str) -> Key {
    let lower = text.to_lowercase();
    (lower.replace('ё', "е"), lower, text.to_owned())
}

pub fn compare(left: &str, right: &str) -> Ordering {
    key(left).cmp(&key(right))
}

#[cfg(test)]
mod tests {
    #[test]
    fn yo_sorts_as_ye_and_follows_it_when_otherwise_equal() {
        let mut names = vec!["Яблоко", "ёлка", "Жук", "Ель", "елка", "Ёж", "арбуз", "Арбуз"];
        names.sort_by(|left, right| super::compare(left, right));
        assert_eq!(names, ["Арбуз", "арбуз", "Ёж", "елка", "ёлка", "Ель", "Жук", "Яблоко"]);
    }

    #[test]
    fn the_key_orders_exactly_like_compare() {
        let names = ["Ёж", "ёж", "еж", "Еж", "ЕЖ"];
        for left in names {
            for right in names {
                assert_eq!(super::key(left).cmp(&super::key(right)), super::compare(left, right), "{left} / {right}");
            }
        }
        let mut sorted = names.to_vec();
        sorted.sort_by_cached_key(|name| super::key(name));
        assert_eq!(sorted, ["ЕЖ", "Еж", "еж", "Ёж", "ёж"]);
    }
}
