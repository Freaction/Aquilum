pub fn rank<'a>(items: &[&'a str], query: &str) -> Vec<&'a str> {
    let query = query.trim().to_lowercase();
    let (mut first, rest): (Vec<&str>, Vec<&str>) = items
        .iter()
        .copied()
        .filter(|item| item.to_lowercase().contains(&query))
        .partition(|item| item.to_lowercase().starts_with(&query));
    first.extend(rest);
    first
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
