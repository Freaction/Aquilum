use crate::links::reader::href;

pub fn format(text: &str, cfi: &str, book: Option<&str>, number: usize) -> String {
    let cleaned = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if cfi.is_empty() {
        return format!("> [!quote] {cleaned}\n");
    }
    format!("> [!quote] {} [{number}]({})\n", aquilum_core::markdown::escape::label(&cleaned), href(cfi, book))
}
