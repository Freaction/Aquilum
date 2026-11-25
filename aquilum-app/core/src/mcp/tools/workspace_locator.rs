use super::super::vault::Vault;
use crate::search::paths::{canonical_path, same_path};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};
use crate::app_core::Core;

const GUEST_SCAN_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

pub fn workspace_vault(core: &Core, input: &str) -> Result<Vault, String> {
    let root = known_workspace(core, input)?;
    let visible = core
        .search
        .active_root()
        .is_some_and(|active| same_path(&active, &root));
    Ok(Vault { root, visible })
}

pub fn open_guest(core: &Core, root: &Path) -> Result<(), String> {
    let service = &core.search;
    let workspace = root.to_string_lossy();
    service
        .open_guest(&workspace)
        .map_err(|error| error.to_string())?;
    if wait_until(GUEST_SCAN_TIMEOUT, || !service.status(Some(&workspace)).updating) {
        Ok(())
    } else {
        Err(format!(
            "База знаний {workspace} ещё индексируется, поиск по ней пока неполон — повторите запрос через несколько секунд"
        ))
    }
}

pub fn known_workspace(core: &Core, input: &str) -> Result<PathBuf, String> {
    let root = if Path::new(input).is_absolute() {
        let known = core.ui_state.workspace(Path::new(input)).ok_or_else(|| unknown_workspace(input))?;
        canonical_path(Path::new(&known.path))
    } else {
        workspace_by_tail(&known_paths(core)?, input)?
    };
    if !root.is_dir() {
        return Err(format!("Папка базы знаний не найдена: {}", root.to_string_lossy()));
    }
    Ok(root)
}

pub fn known_paths(core: &Core) -> Result<Vec<PathBuf>, String> {
    core.ui_state
        .workspace_roots()
        .map_err(|error| format!("{error:?}"))
}

pub fn address(known: &[PathBuf], root: &Path) -> String {
    let parts = root_parts(root);
    for length in 1..=parts.len() {
        let tail = &parts[parts.len() - length..];
        let lowered = tail.iter().map(|part| part.to_lowercase()).collect::<Vec<_>>();
        if known.iter().filter(|other| ends_with(other, &lowered)).count() == 1 {
            return tail.join("/");
        }
    }
    root.to_string_lossy().into_owned()
}

pub fn wait_until(timeout: Duration, ready: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if ready() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn workspace_by_tail(known: &[PathBuf], input: &str) -> Result<PathBuf, String> {
    let wanted = input
        .split(['/', '\\'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    let matching = known
        .iter()
        .filter(|root| ends_with(root, &wanted))
        .collect::<Vec<_>>();
    match matching.as_slice() {
        [root] => Ok((*root).clone()),
        [] => Err(unknown_workspace(input)),
        many => Err(format!(
            "Под «{input}» подходят несколько баз, уточните: {}",
            many.iter()
                .map(|root| address(known, root))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn ends_with(root: &Path, wanted: &[String]) -> bool {
    let parts = root_parts(root);
    !wanted.is_empty()
        && parts.len() >= wanted.len()
        && parts[parts.len() - wanted.len()..]
            .iter()
            .zip(wanted)
            .all(|(part, wanted)| part.to_lowercase() == *wanted)
}

fn root_parts(root: &Path) -> Vec<String> {
    root.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

fn unknown_workspace(input: &str) -> String {
    format!("База знаний «{input}» не открывалась в Aquilum — доступные базы покажет list_workspaces")
}

#[cfg(test)]
mod tests {
    use super::{address, workspace_by_tail};
    use std::path::PathBuf;

    fn known() -> Vec<PathBuf> {
        [
            "D:/Games/zombie voxel/Knowledge base",
            "D:/My programs/Aquilum/knowledge base",
            "D:/База знаний копия/NeuroNet",
            "C:/Users/Dmitriy/OneDrive/Документы/NeuroNet",
            "C:/Users/Dmitriy/Desktop/Solo",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect()
    }

    #[test]
    fn the_address_is_the_shortest_unique_tail() {
        let known = known();
        let addresses = known.iter().map(|root| address(&known, root)).collect::<Vec<_>>();
        assert_eq!(
            addresses,
            [
                "zombie voxel/Knowledge base",
                "Aquilum/knowledge base",
                "База знаний копия/NeuroNet",
                "Документы/NeuroNet",
                "Solo",
            ]
        );
    }

    #[test]
    fn a_tail_selects_one_workspace_or_names_the_candidates() {
        let known = known();
        assert_eq!(workspace_by_tail(&known, "zombie voxel/knowledge base").unwrap(), known[0]);
        assert_eq!(workspace_by_tail(&known, "solo").unwrap(), known[4]);
        let ambiguous = workspace_by_tail(&known, "NeuroNet").unwrap_err();
        assert!(
            ambiguous.contains("База знаний копия/NeuroNet") && ambiguous.contains("Документы/NeuroNet"),
            "{ambiguous}"
        );
        assert!(workspace_by_tail(&known, "Нет такой").is_err());
        assert!(
            workspace_by_tail(&known, "voxel/Knowledge").is_err(),
            "хвост сравнивается целыми папками"
        );
    }
}
