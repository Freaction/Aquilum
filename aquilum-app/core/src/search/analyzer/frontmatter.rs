use crate::search::fields::note_fields;
use crate::search::fields::value::lower;
use std::borrow::Cow;
use std::ops::Range;

pub struct Bounds {
    pub yaml: Range<usize>,
    pub end: usize,
    pub body: usize,
}

pub fn fence(line: &str) -> bool {
    line.trim_end_matches([' ', '\t']) == "---"
}

pub fn bounds(text: &str) -> Option<Bounds> {
    let first = text.find('\n')?;
    if !fence(&text[..first]) {
        return None;
    }
    let mut at = first + 1;
    for line in text[at..].split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        if fence(content) {
            return Some(Bounds { yaml: first + 1..at, end: at + content.len(), body: at + line.len() });
        }
        at += line.len();
    }
    None
}

pub fn strip_frontmatter(text: &str) -> Cow<'_, str> {
    let Some(found) = bounds(text) else {
        return Cow::Borrowed(text);
    };
    let body = text[found.body..].trim_start_matches('\n');
    match field(text, "author") {
        Some(name) if body.is_empty() => Cow::Owned(name),
        Some(name) => Cow::Owned(format!("{name}\n{body}")),
        None => Cow::Borrowed(body),
    }
}

pub fn body(text: &str) -> &str {
    bounds(text).map_or(text, |found| &text[found.body..])
}

pub fn yaml_block(text: &str) -> Option<&str> {
    bounds(text).map(|found| &text[found.yaml])
}

pub fn same_key(left: &str, right: &str) -> bool {
    lower(left) == lower(right)
}

pub fn set(head: &str, updates: &[(&str, Option<&str>)], same: impl Fn(&str, &str) -> bool) -> Result<Option<(Range<usize>, String)>, String> {
    let Some(found) = bounds(head) else {
        if head.lines().next().is_some_and(fence) {
            return Err("Блок frontmatter открыт, но не закрыт разделителем «---» — исправьте заметку вручную".to_owned());
        }
        let created: String = updates.iter().filter_map(|(key, value)| Some(format!("{key}: {}\n", (*value)?))).collect();
        return Ok((!created.is_empty()).then(|| (0..0, format!("---\n{created}---\n"))));
    };
    let mut lines = head[found.yaml.clone()].lines().map(str::to_owned).collect::<Vec<_>>();
    for (key, value) in updates {
        let declares = |line: &String| line.split_once(':').is_some_and(|(name, _)| same(name.trim(), key));
        let Some(value) = value else {
            lines.retain(|line| !declares(line));
            continue;
        };
        match lines.iter().position(declares) {
            Some(index) => lines[index] = format!("{}: {value}", lines[index].split_once(':').map_or("", |(name, _)| name.trim())),
            None => lines.push(format!("{key}: {value}")),
        }
    }
    let yaml: String = lines.iter().map(|line| format!("{line}\n")).collect();
    Ok((head[found.yaml.clone()] != yaml).then_some((found.yaml, yaml)))
}

pub fn field(text: &str, wanted: &str) -> Option<String> {
    note_fields(text).into_iter().find(|field| field.key.eq_ignore_ascii_case(wanted) && !field.text.is_empty()).map(|field| field.text)
}

pub fn unquote(value: &str) -> String {
    ['"', '\''].into_iter().find_map(|quote| value.strip_prefix(quote)?.strip_suffix(quote)).map_or(value, str::trim).to_owned()
}

#[cfg(test)]
#[path = "frontmatter_tests.rs"]
mod tests;
