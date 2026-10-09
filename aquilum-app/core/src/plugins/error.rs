use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "code", content = "details", rename_all = "snake_case")]
pub enum PluginError {
    Io { message: String },
    Invalid { message: String },
    Task { message: String },
}

impl From<std::io::Error> for PluginError {
    fn from(error: std::io::Error) -> Self {
        Self::Io { message: error.to_string() }
    }
}

impl From<crate::TaskFailed> for PluginError {
    fn from(error: crate::TaskFailed) -> Self {
        Self::Task { message: error.to_string() }
    }
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { message } | Self::Invalid { message } | Self::Task { message } => formatter.write_str(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PluginError;

    #[test]
    fn plugin_errors_keep_the_command_error_shape() {
        let errors = [
            (PluginError::from(std::io::Error::other("io")), "io"),
            (PluginError::Invalid { message: "invalid".to_owned() }, "invalid"),
            (PluginError::from(crate::TaskFailed("task".to_owned())), "task"),
        ];
        for (error, code) in errors {
            assert_eq!(serde_json::to_value(error).unwrap(), serde_json::json!({"code":code,"details":{"message":code}}));
        }
    }
}
