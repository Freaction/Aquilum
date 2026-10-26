use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

const ATTEMPTS_FILE: &str = "update-attempt.json";
pub const MAX_ATTEMPTS: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    pub version: String,
    pub url: String,
    pub attempts: u32,
}

impl Attempt {
    fn file(data_dir: &Path) -> PathBuf {
        data_dir.join(ATTEMPTS_FILE)
    }

    pub fn load(data_dir: &Path) -> Option<Attempt> {
        let content = std::fs::read_to_string(Self::file(data_dir)).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self, data_dir: &Path) {
        if let Ok(json) = serde_json::to_string(self) {
            let _ = std::fs::write(Self::file(data_dir), json);
        }
    }

    pub fn clear(data_dir: &Path) {
        let _ = std::fs::remove_file(Self::file(data_dir));
    }

    pub fn check_on_start(data_dir: &Path, current_version: &str) -> Option<Attempt> {
        let attempt = Self::load(data_dir)?;
        if attempt.version == current_version {
            Self::clear(data_dir);
            None
        } else if attempt.attempts >= MAX_ATTEMPTS {
            Some(attempt)
        } else {
            None
        }
    }

    pub fn record(data_dir: &Path, version: &str, url: &str) {
        let mut attempt = Self::load(data_dir).unwrap_or_else(|| Attempt {
            version: version.to_owned(),
            url: url.to_owned(),
            attempts: 0,
        });
        if attempt.version != version {
            attempt.version = version.to_owned();
            attempt.url = url.to_owned();
            attempt.attempts = 0;
        }
        attempt.attempts += 1;
        attempt.save(data_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attempts_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        // 1. Initially nothing
        assert_eq!(Attempt::check_on_start(path, "0.3.15"), None);

        // 2. Record first attempt
        Attempt::record(path, "0.3.16", "https://example.com/setup.exe");
        let attempt = Attempt::load(path).unwrap();
        assert_eq!(attempt.attempts, 1);
        assert_eq!(attempt.version, "0.3.16");

        // 3. Restart still on 0.3.15 (attempt 1 < 2 -> None, but still recorded)
        assert_eq!(Attempt::check_on_start(path, "0.3.15"), None);

        // 4. Record second attempt
        Attempt::record(path, "0.3.16", "https://example.com/setup.exe");
        let attempt = Attempt::load(path).unwrap();
        assert_eq!(attempt.attempts, 2);

        // 5. Restart still on 0.3.15 (attempt 2 >= 2 -> Some(attempt))
        let failed = Attempt::check_on_start(path, "0.3.15").unwrap();
        assert_eq!(failed.version, "0.3.16");
        assert_eq!(failed.attempts, 2);

        // 6. After successful update to 0.3.16 -> file cleared
        assert_eq!(Attempt::check_on_start(path, "0.3.16"), None);
        assert!(Attempt::load(path).is_none());
    }
}
