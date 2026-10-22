use super::*;
use super::moves::free;
use crate::files::attachments::resolve_attachments_impl;
use crate::search::wiki::parser::attachments;
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

pub(super) fn across(core: &Core, vault: &Vault, destination: &Vault, notes: &[PathBuf], targets: &[PathBuf]) -> Result<Value, String> {
    for target in targets {
        free(destination, target)?;
    }
    let moving: HashSet<String> = notes.iter().map(|note| vault.relative(note)).collect();
    let broken: BTreeSet<String> = notes
        .iter()
        .flat_map(|note| core.search.backlinks(&vault.root.to_string_lossy(), &note.to_string_lossy()).unwrap_or_default())
        .map(|link| vault.relative(Path::new(&link.path)))
        .filter(|path| !moving.contains(path))
        .collect();
    let files = embedded(vault, notes);
    let shared = shared(vault, &moving, &files);
    for (note, target) in notes.iter().zip(targets) {
        gate::move_across(core, note, target).map_err(|error| error.to_string())?;
    }
    let (mut moved, mut copied) = (Vec::new(), Vec::new());
    for file in files {
        let (from, to) = (vault.root.join(&file), destination.root.join(&file));
        if to.exists() {
            continue;
        }
        free(destination, &to)?;
        if shared.contains(&file) {
            gate::copy(core, &from, &to).map_err(|error| error.to_string())?;
            copied.push(file);
        } else {
            gate::move_across(core, &from, &to).map_err(|error| error.to_string())?;
            moved.push(file);
        }
    }
    Ok(json!({
        "moved": targets.iter().map(|target| destination.relative(target)).collect::<Vec<_>>(),
        "workspace": destination.root.to_string_lossy(),
        "brokenLinks": broken,
        "files": { "moved": moved, "copied": copied },
    }))
}

fn embedded(vault: &Vault, notes: &[PathBuf]) -> BTreeSet<String> {
    let names: Vec<String> = notes.iter().filter_map(|note| std::fs::read_to_string(note).ok()).flat_map(|text| attachments(&text)).collect();
    let found = resolve_attachments_impl(&vault.root, names.clone());
    names
        .into_iter()
        .zip(found)
        .filter_map(|(name, found)| found.or_else(|| vault.root.join(&name).is_file().then(|| name.replace('\\', "/"))))
        .collect()
}

fn shared(vault: &Vault, moving: &HashSet<String>, files: &BTreeSet<String>) -> HashSet<String> {
    let names: Vec<(&String, &str)> = files.iter().map(|file| (file, file.rsplit('/').next().unwrap_or(file))).collect();
    let mut shared = HashSet::new();
    for note in vault.markdown_files().filter(|note| !moving.contains(&vault.relative(note))) {
        let Ok(text) = std::fs::read_to_string(&note) else { continue };
        shared.extend(names.iter().filter(|(_, name)| text.contains(name)).map(|(file, _)| (*file).clone()));
        if shared.len() == names.len() {
            break;
        }
    }
    shared
}
