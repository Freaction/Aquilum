use crate::markdown::{fence, list};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Task {
    pub line: usize,
    pub done: bool,
    pub text: String,
}

pub fn note_tasks(body: &str) -> Vec<Task> {
    fence::outside(body)
        .into_iter()
        .filter_map(|(index, line)| {
            let item = list::item(line)?;
            let task = item.task?;
            Some(Task { line: index + 1, done: task.done, text: line[item.content..].trim().to_owned() })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{note_tasks, Task};

    fn task(line: usize, done: bool, text: &str) -> Task {
        Task { line, done, text: text.to_owned() }
    }

    #[test]
    fn an_open_and_a_finished_task_are_told_apart() {
        assert_eq!(
            note_tasks("- [ ] купить хлеб\n- [x] позвонить"),
            vec![task(1, false, "купить хлеб"), task(2, true, "позвонить")]
        );
    }

    #[test]
    fn any_mark_but_a_space_means_the_task_is_closed() {
        let found = note_tasks("- [/] в работе\n- [-] отменена\n- [ ] открыта");
        assert_eq!(found.iter().filter(|item| item.done).count(), 2);
    }

    #[test]
    fn a_nested_task_keeps_its_line_and_loses_its_marker() {
        let found = note_tasks("# Заголовок\n\n    - [ ] вложенная\n3. [x] нумерованная");
        assert_eq!(found, vec![task(3, false, "вложенная"), task(4, true, "нумерованная")]);
    }

    #[test]
    fn a_checkbox_inside_a_code_block_is_not_a_task() {
        let text = "```md\n- [ ] пример разметки\n```\n- [ ] настоящая";
        assert_eq!(note_tasks(text), vec![task(4, false, "настоящая")]);
    }

    #[test]
    fn a_line_without_a_checkbox_is_not_a_task() {
        assert!(note_tasks("- обычный пункт\n[ ] без маркера\n- [] пустые скобки").is_empty());
    }

    #[test]
    fn every_list_marker_can_carry_a_task() {
        assert_eq!(
            note_tasks("- [ ] дефис\n1. [ ] точка\n1) [x] скобка\n* [ ] звезда\n+ [x] плюс"),
            vec![task(1, false, "дефис"), task(2, false, "точка"), task(3, true, "скобка"), task(4, false, "звезда"), task(5, true, "плюс")]
        );
    }

    #[test]
    fn an_empty_task_is_still_a_task() {
        assert_eq!(note_tasks("- [ ]"), vec![task(1, false, "")]);
    }
}
