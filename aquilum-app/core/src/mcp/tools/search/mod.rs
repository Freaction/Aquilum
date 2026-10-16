mod links;
mod query;
mod related;

use super::{tool, workspace_property};
use crate::app_core::Core;
use crate::search::analysis::models::AnalysisMethod;
use serde_json::{json, Value};

const SNIPPET_LIMIT: usize = 300;

pub fn definitions() -> Vec<Value> {
    vec![
        tool(
            "search_notes",
            "Полнотекстовый поиск по базе знаний со стеммингом. Слова запроса объединяются по «ИЛИ» и ранжируются, поэтому длинный запрос находит и частичные совпадения: смотрите matchedTerms, какие слова нашлись. У каждого результата есть heading — раздел с совпадением, и backlinks — сколько заметок на него ссылаются. Поле hasMore говорит, осталось ли что-то за пределами выдачи. Ищет по тексту заметок: frontmatter в полнотекстовый индекс не входит, по его полям ищет find_notes.",
            json!({
                "query": { "type": "string", "description": "Поисковый запрос: чем больше слов, тем выше в выдаче заметки, где нашлись все" },
                "folder": { "type": "string", "description": "Искать только в этой папке, пусто — вся база" },
                "limit": { "type": "integer", "description": "Сколько заметок вернуть, по умолчанию 10" },
                "workspace": workspace_property(),
            }),
            &["query"],
        ),
        tool(
            "dataview_syntax",
            "Язык блоков ```dataview: формы запроса, части, поля и функции — с примерами. Вызовите без аргументов, чтобы получить справку, или передайте query, чтобы проверить запрос перед вставкой в заметку.",
            json!({
                "query": { "type": "string", "description": "Текст запроса для проверки; пусто — вернуть справку" },
            }),
            &[],
        ),
        tool(
            "related_notes",
            "Похожие заметки по тексту (BM25F): что писать рядом, что дублируется, что развить.",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "limit": { "type": "integer", "description": "Сколько заметок вернуть, по умолчанию 10" },
                "workspace": workspace_property(),
            }),
            &["note"],
        ),
        tool(
            "linked_notes",
            "Связанные заметки по графу ссылок [[...]] (Adamic-Adar): общие соседи, а не общий текст.",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "limit": { "type": "integer", "description": "Сколько заметок вернуть, по умолчанию 10" },
                "workspace": workspace_property(),
            }),
            &["note"],
        ),
        tool(
            "backlinks",
            "Заметки, которые ссылаются на указанную: по одной строке на заметку, mentions — сколько раз ссылка встречается, heading и context — раздел и строка первого упоминания.",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "workspace": workspace_property(),
            }),
            &["note"],
        ),
        tool(
            "outgoing_links",
            "Ссылки [[...]] из указанной заметки, включая ещё не созданные, по одной строке на цель; mentions — сколько раз она упомянута.",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "workspace": workspace_property(),
            }),
            &["note"],
        ),
        tool(
            "wiki_sources",
            "Статьи Wikipedia по теме заметки.",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "workspace": workspace_property(),
            }),
            &["note"],
        ),
        tool(
            "get_index_status",
            "Состояние поискового индекса: готов ли он и сколько заметок проиндексировано.",
            json!({
                "workspace": workspace_property(),
            }),
            &[],
        ),
    ]
}

pub fn call(
    core: &Core,
    name: &str,
    arguments: &Value,
) -> Option<Result<Value, String>> {
    Some(match name {
        "search_notes" => query::search_notes(core, arguments),
        "dataview_syntax" => query::dataview_syntax(arguments),
        "related_notes" => related::analyze(core, arguments, AnalysisMethod::Bm25f),
        "linked_notes" => related::analyze(core, arguments, AnalysisMethod::AdamicAdar),
        "backlinks" => links::backlinks(core, arguments),
        "outgoing_links" => links::outgoing_links(core, arguments),
        "wiki_sources" => related::wiki_sources(core, arguments),
        "get_index_status" => query::index_status(core, arguments),
        _ => return None,
    })
}
