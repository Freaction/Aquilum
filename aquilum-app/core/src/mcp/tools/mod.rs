mod frontmatter;
mod history;
mod notes;
mod search;
mod trash;
mod workspace;
mod workspace_locator;
mod arguments;

pub use arguments::*;

use super::vault::Vault;
use std::path::Path;
use serde_json::{json, Value};
use crate::app_core::Core;


pub const INSTRUCTIONS: &str = concat!(
    "Aquilum — локальная база знаний из markdown-файлов. ",
    "Заметки адресуются относительным путём от корня базы (например «Проекты/Aquilum.md») ",
    "или точным названием. Абсолютные пути не нужны. ",
    "Если пользователь говорит «текущая заметка» или «эта заметка», сначала вызовите ",
    "get_workspace и возьмите путь из поля activeNote. ",
    "Перед выводами о содержимом базы проверяйте поле indexing: пока индекс строится, ",
    "поиск может вернуть неполный результат.\n",
    "Порядок работы: незнакомую базу начинайте с list_notes sort=links — сверху окажутся хабы, ",
    "на которые ссылается больше всего заметок; дальше ищите, а не перебирайте наугад; ",
    "длинную заметку читайте не целиком, а через note_outline и нужный раздел; правьте ",
    "разделом, а не всей заметкой.\n",
    "Фоновая работа с другой базой знаний: передайте её address из list_workspaces аргументом workspace — ",
    "поиск, чтение и правка пойдут в фоновом гостевом индексе, а у пользователя на экране ничего не переключится. ",
    "Запрещено вызывать switch_workspace и open_note для чтения или записи — они переключают видимое окно пользователя! ",
    "switch_workspace и open_note вызываются ИСКЛЮЧИТЕЛЬНО по прямой просьбе пользователя показать/открыть базу или заметку на экране."
);

pub fn definitions() -> Vec<Value> {
    let mut all = search::definitions();
    all.extend(notes::definitions());
    all.extend(frontmatter::definitions());
    all.extend(trash::definitions());
    all.extend(history::definitions());
    all.extend(workspace::definitions());
    all
}

pub fn call(core: &Core, name: &str, arguments: &Value) -> Result<Value, String> {
    search::call(core, name, arguments)
        .or_else(|| notes::call(core, name, arguments))
        .or_else(|| frontmatter::call(core, name, arguments))
        .or_else(|| trash::call(core, name, arguments))
        .or_else(|| history::call(core, name, arguments))
        .or_else(|| workspace::call(core, name, arguments))
        .unwrap_or_else(|| Err(format!("Неизвестный инструмент: {name}")))
}

pub fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false,
        },
    })
}

pub fn version_name_property() -> Value {
    json!({ "type": "string", "description": "Название версии, которую создаст эта правка: в истории заметки она сразу будет под этим именем" })
}

pub fn open_after_write(core: &Core, vault: &Vault, note: &Path, arguments: &Value) {
    if vault.visible && flag(arguments, "open", true) && !core.active_note.is(note) {
        super::bridge::open_note_after_write(core, note);
    }
}

pub fn workspace_property() -> Value {
    json!({ "type": "string", "description": "База знаний, если не текущая: address из list_workspaces или полный путь" })
}

impl super::vault::Vault {
    pub fn note_argument(&self, arguments: &Value) -> Result<std::path::PathBuf, String> {
        self.note(&text(arguments, "note")?)
    }

    pub fn folder_argument(&self, arguments: &Value) -> Result<std::path::PathBuf, String> {
        let folder = self.folder(&optional_text(arguments, "folder").unwrap_or_default())?;
        match folder.is_dir() {
            true => Ok(folder),
            false => Err(format!("Папка не найдена: {}", self.relative(&folder))),
        }
    }
}

pub fn shorten(value: &str, maximum: usize) -> String {
    if value.chars().count() <= maximum {
        return value.to_owned();
    }
    let head = value.chars().take(maximum).collect::<String>();
    format!("{head}…")
}

pub fn require_write(core: &Core) -> Result<(), String> {
    if core.settings.get_config().mcp.allow_write {
        return Ok(());
    }
    Err("Изменение заметок запрещено в настройках Aquilum".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{optional_names, text};
    use serde_json::json;

    #[test]
    fn path_from_a_search_result_works_as_note() {
        let found = json!({ "path": "Base/Заметка.md" });
        assert_eq!(text(&found, "note").unwrap(), "Base/Заметка.md");
        assert_eq!(text(&json!({ "note": "Заметка" }), "path").unwrap(), "Заметка");
    }

    #[test]
    fn the_named_argument_wins_over_its_alias() {
        let both = json!({ "note": "Своя.md", "path": "Чужая.md" });
        assert_eq!(text(&both, "note").unwrap(), "Своя.md");
    }

    #[test]
    fn the_error_lists_the_arguments_that_did_arrive() {
        let error = text(&json!({ "heading": "Раздел" }), "note").unwrap_err();
        assert!(error.contains("note"), "названо недостающее поле: {error}");
        assert!(error.contains("heading"), "названо переданное поле: {error}");
        assert!(text(&json!({}), "note").unwrap_err().contains("не передано"));
    }

    #[test]
    fn a_list_argument_is_absent_empty_or_valid() {
        assert!(optional_names(&json!({}), "notes").unwrap().is_none());
        assert!(optional_names(&json!({ "notes": null }), "notes").unwrap().is_none());
        assert!(optional_names(&json!({ "notes": [] }), "notes").is_err());
        assert!(optional_names(&json!({ "notes": "одна" }), "notes").is_err());
        assert!(optional_names(&json!({ "notes": ["a", " "] }), "notes").is_err());
        assert_eq!(
            optional_names(&json!({ "notes": ["a", " b "] }), "notes").unwrap(),
            Some(vec!["a".to_owned(), "b".to_owned()])
        );
    }
}
