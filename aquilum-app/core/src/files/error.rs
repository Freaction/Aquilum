use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "code", content = "details", rename_all = "snake_case")]
pub enum FileCommandError {
    NotFound {
        message: String,
    },
    Io {
        message: String,
    },
    InvalidUtf8 {
        message: String,
    },
    Conflict {
        expected_hash: String,
        actual_hash: Option<String>,
    },
    AlreadyExists {
        path: String,
    },
    Task {
        message: String,
    },
}

impl From<std::io::Error> for FileCommandError {
    fn from(error: std::io::Error) -> Self {
        if error.kind() == std::io::ErrorKind::NotFound {
            Self::NotFound { message: error.to_string() }
        } else {
            Self::Io { message: error.to_string() }
        }
    }
}

impl From<crate::app_core::TaskFailed> for FileCommandError {
    fn from(error: crate::app_core::TaskFailed) -> Self {
        Self::Task {
            message: error.to_string(),
        }
    }
}

impl std::fmt::Display for FileCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { message }
            | Self::Io { message }
            | Self::InvalidUtf8 { message }
            | Self::Task { message } => formatter.write_str(message),
            Self::Conflict { .. } => formatter.write_str("файл изменился на диске"),
            Self::AlreadyExists { path } => write!(formatter, "уже существует: {path}"),
        }
    }
}
