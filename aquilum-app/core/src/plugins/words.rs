use regex::Regex;
use std::sync::LazyLock;

fn body(md: &str) -> &str {
    let mut lines = md.split_inclusive('\n');
    if lines.next().is_some_and(|line| line.trim_end_matches(['\r', '\n']) == "---") {
        let mut offset = md.find('\n').map_or(md.len(), |i| i + 1);
        for line in lines {
            offset += line.len();
            if line.trim_end_matches(['\r', '\n']) == "---" { return &md[offset..]; }
        }
    }
    md
}

pub fn count_words(md: &str) -> usize {
    static WORDS: LazyLock<Regex> = LazyLock::new(|| Regex::new(
        r"(?:[0-9]+(?:[,.][0-9]+)*|[[\p{L}\p{M}-]--[\p{Han}\p{Hiragana}\p{Katakana}]])+|[\p{Han}\p{Hiragana}\p{Katakana}]"
    ).expect("регулярное выражение слов"));
    WORDS.find_iter(body(md)).count()
}

pub fn has_open_tasks(md: &str) -> bool {
    body(md).lines().any(|line| {
        let line = line.trim_start_matches([' ', '\t']);
        line.starts_with("- [ ]") || line.starts_with("* [ ]")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wip_words_without_frontmatter() {
        assert_eq!(count_words("---\r\ntitle: ignored\r\n---\r\n你好 かな カナ 1,5 1.000,5 кто-то abc123"), 10);
        assert_eq!(count_words("hello-world -- 123"), 3);
        assert_eq!(count_words(""), 0);
        assert_eq!(count_words("---\nonly frontmatter\n---"), 0);
        assert_eq!(count_words("---\nunclosed"), 2);
    }
    #[test]
    fn tasks_start_at_indented_line() {
        assert!(has_open_tasks("text\n  - [ ] todo\n"));
        assert!(has_open_tasks("\t* [ ] todo"));
        assert!(!has_open_tasks("text - [ ] todo\n- [x] done"));
        assert!(!has_open_tasks("---\n- [ ] metadata\n---\n"));
    }
}
