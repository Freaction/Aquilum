use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

pub fn load<T: DeserializeOwned>(path: &Path) -> Option<T> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

pub fn save<T: Serialize>(path: &Path, value: &T, what: &str) {
    let Ok(json) = serde_json::to_string(value) else { return };
    if let Err(error) = aquilum_core::files::document::write_content_atomic(path, &json, None) {
        report!("не удалось сохранить {what}: {error}");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_saved_value_loads_back_and_a_missing_or_broken_file_gives_nothing() {
        let dir = crate::test_dir::tempdir();
        let path = dir.path().join("state.json");
        assert_eq!(super::load::<Vec<u32>>(&path), None);
        super::save(&path, &vec![1u32, 2], "проверку");
        assert_eq!(super::load::<Vec<u32>>(&path), Some(vec![1, 2]));
        std::fs::write(&path, "{не json").unwrap();
        assert_eq!(super::load::<Vec<u32>>(&path), None);
    }
}
