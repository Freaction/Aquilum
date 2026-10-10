use super::error::UiStateError;
use std::path::{Component, Path};

pub fn canonical_workspace(path: &str) -> Result<String, UiStateError> {
    let canonical = std::fs::canonicalize(path)?;
    Ok(canonical.to_string_lossy().into_owned())
}

pub fn display_workspace(path: &str) -> String {
    if let Some(share) = path.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{share}");
    }
    path.strip_prefix(r"\\?\").unwrap_or(path).to_owned()
}

pub fn normalize_relative(path: &str) -> Result<String, UiStateError> {
    let mut parts = Vec::new();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(value) => parts.push(value.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(UiStateError::InvalidInput {
                    message: format!("path is not relative: {path}"),
                });
            }
        }
    }
    if parts.is_empty() {
        return Err(UiStateError::InvalidInput {
            message: "document path is empty".to_owned(),
        });
    }
    Ok(parts.join("/"))
}

pub fn normalize_base_relative(path: &str) -> Result<String, UiStateError> {
    let relative = normalize_relative(path)?;
    if !Path::new(&relative)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("base"))
    {
        return Err(UiStateError::InvalidInput {
            message: "base path must have a .base extension".into(),
        });
    }
    Ok(relative)
}

#[cfg(test)]
mod tests {
    use super::{canonical_workspace, normalize_relative};

    #[test]
    fn relative_path_is_normalized_without_leaving_workspace() {
        assert_eq!(
            normalize_relative("folder/./note.md").unwrap(),
            "folder/note.md"
        );
        assert!(normalize_relative("../note.md").is_err());
        assert!(normalize_relative("").is_err());
    }

    #[test]
    fn workspace_path_is_canonical() {
        let directory = tempfile::tempdir().expect("directory");
        let expected = std::fs::canonicalize(directory.path()).expect("canonical");
        let actual = canonical_workspace(&directory.path().to_string_lossy()).expect("workspace");
        assert_eq!(actual, expected.to_string_lossy());
    }
}
