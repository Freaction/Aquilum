use super::BaseError;
use crate::search::analyzer::{frontmatter_body, frontmatter_yaml};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use serde_json::{Map, Value};

pub(crate) fn fields(text: &str) -> Result<Map<String, Value>, BaseError> {
    let Some(yaml) = frontmatter_yaml(text) else {
        return if text
            .trim_start()
            .lines()
            .next()
            .is_some_and(|line| line.trim() == "---")
        {
            Err(BaseError::InvalidYaml("Unclosed frontmatter".into()))
        } else {
            Ok(Map::new())
        };
    };
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(yaml).map_err(|error| BaseError::InvalidYaml(error.to_string()))?;
    if value.is_null() {
        return Ok(Map::new());
    }
    if !value.is_mapping() {
        return Err(BaseError::InvalidYaml(
            "Frontmatter must be a mapping".into(),
        ));
    }
    serde_json::to_value(value)
        .map_err(|error| BaseError::InvalidYaml(error.to_string()))?
        .as_object()
        .cloned()
        .ok_or_else(|| BaseError::InvalidYaml("Frontmatter must be a mapping".into()))
}

pub(crate) fn tags(text: &str, fields: &Map<String, Value>) -> Vec<String> {
    let mut tags = match fields.get("tags") {
        Some(Value::String(tag)) => vec![tag.clone()],
        Some(Value::Array(tags)) => tags
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    };
    let body = frontmatter_body(text);
    let mut excluded = 0;
    for (event, range) in Parser::new(body).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_) | Tag::Link { .. } | Tag::Image { .. }) => excluded += 1,
            Event::End(TagEnd::CodeBlock | TagEnd::Link | TagEnd::Image) => excluded -= 1,
            Event::Text(_) if excluded == 0 => {
                for (index, character) in body[range.clone()].char_indices() {
                    if character != '#' {
                        continue;
                    }
                    let start = range.start + index;
                    if body[..start]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | '/' | '\\'))
                    {
                        continue;
                    }
                    let tag = body[start + 1..]
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/'))
                        .collect::<String>();
                    if !tag.is_empty() && tag.chars().any(|c| !c.is_ascii_digit() && c != '/') {
                        tags.push(tag);
                    }
                }
            }
            _ => {}
        }
    }
    let mut unique = Vec::new();
    for tag in tags {
        let tag = tag.trim_start_matches('#').to_owned();
        if !tag.is_empty()
            && !unique
                .iter()
                .any(|known: &String| known.to_lowercase() == tag.to_lowercase())
        {
            unique.push(tag);
        }
    }
    unique
}
