use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "code", content = "details", rename_all = "snake_case")]
pub enum SearchError {
    Io { message: String },
    Index { message: String },
    Database { message: String },
    InvalidWorkspace { message: String },
    Task { message: String },
    Unavailable { message: String },
    Query { message: String },
    Cancelled,
}

impl SearchError {
    pub fn task(error: impl ToString) -> Self {
        Self::Task {
            message: error.to_string(),
        }
    }

    pub fn invalid_workspace(message: &str) -> Self {
        Self::InvalidWorkspace {
            message: message.to_owned(),
        }
    }
}

impl From<std::io::Error> for SearchError {
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            message: error.to_string(),
        }
    }
}

impl From<tantivy::TantivyError> for SearchError {
    fn from(error: tantivy::TantivyError) -> Self {
        Self::Index {
            message: error.to_string(),
        }
    }
}

impl From<rusqlite::Error> for SearchError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database {
            message: error.to_string(),
        }
    }
}

impl From<notify::Error> for SearchError {
    fn from(error: notify::Error) -> Self {
        Self::Io {
            message: error.to_string(),
        }
    }
}

impl From<crate::app_core::TaskFailed> for SearchError {
    fn from(error: crate::app_core::TaskFailed) -> Self {
        Self::task(error)
    }
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { message }
            | Self::Index { message }
            | Self::Database { message }
            | Self::InvalidWorkspace { message }
            | Self::Task { message }
            | Self::Unavailable { message }
            | Self::Query { message } => formatter.write_str(message),
            Self::Cancelled => formatter.write_str("Search cancelled"),
        }
    }
}
