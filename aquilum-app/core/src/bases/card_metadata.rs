use super::{note, BaseError};
use crate::search::analyzer::frontmatter_yaml;
use serde_json::Value;

pub(super) fn set(text: &str, key: &str, value: &Value) -> Result<String, BaseError> {
    let mut expected = note::fields(text)?;
    expected.insert(key.to_owned(), value.clone());
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let serialized_key = serde_json::to_string(key).unwrap();
    let entry = format!("{serialized_key}: {value}{newline}");
    let Some(yaml) = frontmatter_yaml(text) else {
        return Ok(format!("---{newline}{entry}---{newline}{text}"));
    };
    let yaml_start = yaml.as_ptr() as usize - text.as_ptr() as usize;
    let lines = yaml.split_inclusive('\n').collect::<Vec<_>>();
    let mut start = None;
    let mut end = yaml_start + yaml.len();
    let mut cursor = yaml_start;
    let mut existing_key = None;
    for line in lines {
        if !line.starts_with([' ', '\t']) && !line.trim().is_empty() {
            if let Some((declared, spelling)) = declared_key(line) {
                if start.is_some() {
                    end = cursor;
                    break;
                }
                if declared == key {
                    start = Some(cursor);
                    existing_key = Some(spelling);
                }
            } else if start.is_some() && line.starts_with('#') {
                end = cursor;
                break;
            }
        }
        cursor += line.len();
    }
    let start = start.unwrap_or(yaml_start + yaml.len());
    let spelling = existing_key.as_deref().unwrap_or(&serialized_key);
    let entry = format!("{spelling}: {value}{newline}");
    let updated = format!("{}{entry}{}", &text[..start], &text[end..]);
    if note::fields(&updated)? != expected {
        return Err(BaseError::InvalidYaml(
            "Frontmatter layout cannot be edited safely".into(),
        ));
    }
    Ok(updated)
}

fn declared_key(line: &str) -> Option<(String, String)> {
    for (index, _) in line.match_indices(':') {
        let spelling = line[..index].trim();
        let Ok(mapping) =
            serde_yaml_ng::from_str::<serde_yaml_ng::Mapping>(&format!("{spelling}: null"))
        else {
            continue;
        };
        if mapping.len() == 1 {
            let key = mapping.keys().next()?.as_str()?.to_owned();
            return Some((key, spelling.to_owned()));
        }
    }
    None
}
