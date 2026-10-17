use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldKind {
    Text,
    Number,
    Bool,
    List,
}

impl FieldKind {
    pub fn stored(self) -> i64 {
        match self {
            FieldKind::Text => 0,
            FieldKind::Number => 1,
            FieldKind::Bool => 2,
            FieldKind::List => 3,
        }
    }

    pub fn from_stored(stored: i64) -> Self {
        match stored {
            1 => FieldKind::Number,
            2 => FieldKind::Bool,
            3 => FieldKind::List,
            _ => FieldKind::Text,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Field {
    pub key: String,
    pub kind: FieldKind,
    pub text: String,
    pub items: Vec<String>,
}

impl Field {
    pub fn scalar(key: String, text: String) -> Self {
        let kind = scalar_kind(&text);
        Self {
            key,
            kind,
            text,
            items: Vec::new(),
        }
    }

    pub fn list(key: String, items: Vec<String>) -> Self {
        Self {
            key,
            kind: FieldKind::List,
            text: items.join(", "),
            items,
        }
    }

    pub fn stored(key: String, kind: FieldKind, text: String, items: Option<String>) -> Self {
        let items = items
            .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
            .unwrap_or_default();
        Self {
            key,
            kind,
            text,
            items,
        }
    }

    pub fn push_item(&mut self, item: String) -> bool {
        if self.kind != FieldKind::List {
            if !self.text.trim().is_empty() {
                return false;
            }
            self.kind = FieldKind::List;
        }
        self.items.push(item);
        self.text = self.items.join(", ");
        true
    }

    pub fn json(&self) -> Value {
        match self.kind {
            FieldKind::List => Value::Array(
                self.items
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect::<Vec<_>>(),
            ),
            _ => Value::String(self.text.clone()),
        }
    }
}

fn scalar_kind(text: &str) -> FieldKind {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return FieldKind::Text;
    }
    if trimmed.eq_ignore_ascii_case("true") || trimmed.eq_ignore_ascii_case("false") {
        return FieldKind::Bool;
    }
    if starts_like_number(trimmed) && trimmed.parse::<f64>().is_ok() {
        return FieldKind::Number;
    }
    FieldKind::Text
}

fn starts_like_number(trimmed: &str) -> bool {
    trimmed
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_digit() || first == '-' || first == '+')
}

pub fn lower(text: &str) -> String {
    text.trim().to_lowercase()
}
