use aquilum_core::text::matcher::{IGNORE_CASE, Matcher};

pub fn rank<'a>(items: &[&'a str], query: &str) -> Vec<&'a str> {
    let matcher = Matcher::new(query.trim(), IGNORE_CASE);
    if matcher.is_empty() {
        return items.to_vec();
    }
    let (mut first, rest): (Vec<_>, Vec<_>) = items.iter().filter_map(|&item| Some((item, matcher.first(item)?.start))).partition(|&(_, at)| at == 0);
    first.extend(rest);
    first.into_iter().map(|(item, _)| item).collect()
}

#[cfg(test)]
mod tests {
    use super::rank;

    #[test]
    fn prefix_matches_come_first_and_case_is_ignored() {
        let items = ["page_cover_position", "Page_cover", "cover", "Book_cover", "Путь к файлу"];
        assert_eq!(rank(&items, "COVER"), ["cover", "page_cover_position", "Page_cover", "Book_cover"]);
        assert_eq!(rank(&items, "book"), ["Book_cover"]);
        assert_eq!(rank(&items, "путь"), ["Путь к файлу"]);
        assert_eq!(rank(&items, "page"), ["page_cover_position", "Page_cover"]);
        assert_eq!(rank(&items, "").len(), items.len());
    }
}
