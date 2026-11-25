use std::collections::{HashMap, HashSet};
use std::path::Path;

use aquilum_health::source::{self, Source};
use aquilum_health::{tokens, verdict};
use quote::ToTokens;
use syn::visit::Visit;
use syn::{Attribute, Ident, Visibility};

struct Definitions<'a> {
    source: &'a Source,
    found: Vec<(String, usize, String)>,
    in_trait_impl: bool,
}

impl Definitions<'_> {
    fn add(&mut self, vis: &Visibility, attrs: &[Attribute], ident: &Ident) {
        let exported = attrs.iter().any(|attr| attr.path().is_ident("no_mangle") || attr.to_token_stream().to_string().contains("no_mangle"));
        let name = ident.to_string();
        if matches!(vis, Visibility::Inherited) || exported || name == "main" {
            return;
        }
        self.found.push((self.source.path.clone(), ident.span().start().line, name));
    }
}

impl<'ast> Visit<'ast> for Definitions<'_> {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.add(&item.vis, &item.attrs, &item.sig.ident);
        syn::visit::visit_item_fn(self, item);
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        self.add(&item.vis, &item.attrs, &item.ident);
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        self.add(&item.vis, &item.attrs, &item.ident);
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.add(&item.vis, &item.attrs, &item.ident);
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.add(&item.vis, &item.attrs, &item.ident);
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.add(&item.vis, &item.attrs, &item.ident);
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.add(&item.vis, &item.attrs, &item.ident);
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let outer = std::mem::replace(&mut self.in_trait_impl, item.trait_.is_some());
        syn::visit::visit_item_impl(self, item);
        self.in_trait_impl = outer;
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if !self.in_trait_impl {
            self.add(&item.vis, &item.attrs, &item.sig.ident);
        }
    }

    fn visit_impl_item_const(&mut self, item: &'ast syn::ImplItemConst) {
        if !self.in_trait_impl {
            self.add(&item.vis, &item.attrs, &item.ident);
        }
    }
}

fn mentions<'a>(sources: impl Iterator<Item = &'a Source>) -> HashMap<String, usize> {
    let mut count = HashMap::new();
    for source in sources {
        tokens::words(source.file.to_token_stream(), |word| *count.entry(word).or_default() += 1);
    }
    count
}

#[test]
fn every_named_item_is_used_outside_its_definition() {
    let sources = source::load();
    let production = mentions(source::production(&sources));
    let everywhere = mentions(sources.iter());
    let mut findings = Vec::new();
    for source in source::production(&sources) {
        let mut definitions = Definitions { source, found: Vec::new(), in_trait_impl: false };
        definitions.visit_file(&source.file);
        for (path, line, name) in definitions.found {
            if production.get(&name).copied().unwrap_or(0) > 1 {
                continue;
            }
            let reason = if everywhere.get(&name).copied().unwrap_or(0) > 1 { "нужен только тестам — перенести под #[cfg(test)]" } else { "нигде не используется" };
            findings.push(format!("{path}:{line} `{name}` {reason}"));
        }
    }
    verdict("мёртвый код", findings);
}

#[test]
fn every_source_file_is_part_of_a_crate() {
    let root = source::app_root();
    let sources = source::load();
    let known: HashSet<&str> = sources.iter().map(|source| source.path.as_str()).collect();
    let mut findings = Vec::new();
    for dir in source::SOURCE_DIRS {
        files(&root.join(dir), &mut |path| {
            let relative = source::relative(&root, path);
            if !known.contains(relative.as_str()) {
                findings.push(format!("{relative} не подключён ни одним `mod`"));
            }
        });
    }
    verdict("файлы вне модулей", findings);
}

fn files(dir: &Path, found: &mut impl FnMut(&Path)) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found(&path);
        }
    }
}
