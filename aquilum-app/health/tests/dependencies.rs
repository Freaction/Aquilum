use std::collections::HashSet;

use aquilum_health::source::{self, app_root};
use aquilum_health::{tokens, verdict};
use quote::ToTokens;

const CRATES: [&str; 4] = ["native", "core", "android", "health"];

fn names(manifest: &toml::Table) -> Vec<String> {
    let mut tables: Vec<&toml::Table> = ["dependencies", "build-dependencies", "dev-dependencies"].iter().filter_map(|key| manifest.get(*key)?.as_table()).collect();
    if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
        for target in targets.values().filter_map(toml::Value::as_table) {
            tables.extend(["dependencies", "build-dependencies", "dev-dependencies"].iter().filter_map(|key| target.get(*key)?.as_table()));
        }
    }
    let mut found: Vec<String> = tables.iter().flat_map(|table| table.keys().map(|name| name.replace('-', "_"))).collect();
    found.sort();
    found.dedup();
    found
}

#[test]
fn every_dependency_is_used_by_its_crate() {
    let root = app_root();
    let sources = source::load();
    let mut findings = Vec::new();
    for krate in CRATES {
        let text = std::fs::read_to_string(root.join(krate).join("Cargo.toml")).expect("Cargo.toml");
        let manifest: toml::Table = text.parse().expect("Cargo.toml разбирается");
        let mut used = HashSet::new();
        for source in sources.iter().filter(|source| source.path.starts_with(&format!("{krate}/"))) {
            tokens::words(source.file.to_token_stream(), |word| {
                used.insert(word);
            });
        }
        for name in names(&manifest) {
            if !used.contains(&name) {
                findings.push(format!("{krate}/Cargo.toml: `{name}` не используется в коде крейта"));
            }
        }
    }
    verdict("лишние зависимости", findings);
}
