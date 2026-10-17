use super::store::Candidate;
use crate::search::models::NoteSuggestion;
use crate::text::matcher::{Matcher, Mode, Options, EXACT};

const WORD_START: Options = Options { mode: Mode::WordPrefix, ..EXACT };

pub fn rank(candidates: Vec<Candidate>, key: &str, limit: usize) -> Vec<NoteSuggestion> {
    let matcher = Matcher::new(key, WORD_START);
    let mut ranked = candidates
        .into_iter()
        .map(|candidate| ((tier(&matcher, &candidate.title_key), candidate.title_key.chars().count()), candidate))
        .collect::<Vec<_>>();
    ranked.sort_by(|(left_order, left), (right_order, right)| left_order.cmp(right_order).then_with(|| left.path.cmp(&right.path)));
    ranked
        .into_iter()
        .take(limit)
        .map(|(_, candidate)| suggestion(&candidate.path))
        .collect()
}

fn tier(matcher: &Matcher, title_key: &str) -> u8 {
    match matcher.first(title_key) {
        Some(found) if found.start == 0 => 0,
        Some(_) => 1,
        None => 2,
    }
}

fn suggestion(raw: &str) -> NoteSuggestion {
    NoteSuggestion {
        title: crate::search::paths::title(raw),
        path: raw.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::store::Candidate;
    use super::rank;
    use std::path::Path;

    fn candidate(path: &str) -> Candidate {
        Candidate {
            path: path.to_owned(),
            title_key: crate::search::wiki::target::title_key(Path::new(path)),
        }
    }

    #[test]
    fn name_start_beats_word_start_beats_middle() {
        let titles = rank(
            vec![
                candidate("C:/vault/Кто такой аудмитрий.md"),
                candidate("C:/vault/Заметки/Иван Дмитриев.md"),
                candidate("C:/vault/Дмитрий Чаплинский.md"),
            ],
            "дми",
            3,
        );
        assert_eq!(
            titles
                .iter()
                .map(|item| item.title.as_str())
                .collect::<Vec<_>>(),
            [
                "Дмитрий Чаплинский",
                "Иван Дмитриев",
                "Кто такой аудмитрий"
            ],
        );
    }

    #[test]
    fn title_is_file_stem_and_path_is_kept() {
        let titles = rank(
            vec![candidate("C:/vault/Заметки/Иван Дмитриев.md")],
            "дми",
            1,
        );
        assert_eq!(titles[0].title, "Иван Дмитриев");
        assert_eq!(titles[0].path, "C:/vault/Заметки/Иван Дмитриев.md");
    }

    #[test]
    fn limit_cuts_the_tail() {
        let titles = rank(
            vec![
                candidate("C:/vault/Дмитрий Чаплинский.md"),
                candidate("C:/vault/Дмитрий Иванов.md"),
                candidate("C:/vault/Дмитрий Петров.md"),
            ],
            "дми",
            2,
        );
        assert_eq!(titles.len(), 2);
    }
}
