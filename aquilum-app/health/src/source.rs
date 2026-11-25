use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::visit_mut::VisitMut;
use syn::{Attribute, Item, Meta};

const ROOTS: [&str; 5] = ["native/src/lib.rs", "native/src/main.rs", "native/build.rs", "core/src/lib.rs", "android/src/lib.rs"];
const CHECKS: &str = "health/src/lib.rs";
const CHECK_TESTS: &str = "health/tests";
pub const SOURCE_DIRS: [&str; 5] = ["native/src", "core/src", "android/src", "health/src", "health/tests"];

pub struct Source {
    pub path: String,
    pub file: syn::File,
    pub test: bool,
}

pub fn app_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("каталог приложения").to_path_buf()
}

pub fn load() -> Vec<Source> {
    let root = app_root();
    let mut found = Vec::new();
    for start in ROOTS {
        walk(&root, &root.join(start), false, &mut found);
    }
    walk(&root, &root.join(CHECKS), true, &mut found);
    for entry in std::fs::read_dir(root.join(CHECK_TESTS)).into_iter().flatten().flatten() {
        if entry.path().extension().is_some_and(|extension| extension == "rs") {
            walk(&root, &entry.path(), true, &mut found);
        }
    }
    found
}

pub fn production(sources: &[Source]) -> impl Iterator<Item = &Source> {
    sources.iter().filter(|source| !source.test)
}

pub fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

fn walk(root: &Path, path: &Path, test: bool, found: &mut Vec<Source>) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut file = syn::parse_file(&text).unwrap_or_else(|error| panic!("{}:{}: {error}", path.display(), error.span().start().line));
    let own = match path.file_name().and_then(|name| name.to_str()) {
        Some("mod.rs" | "lib.rs" | "main.rs" | "build.rs") => path.parent().map(Path::to_path_buf),
        _ => path.parent().map(|parent| parent.join(path.file_stem().unwrap_or_default())),
    }
    .unwrap_or_default();
    let parent = path.parent().unwrap_or(root).to_path_buf();
    children(root, &own, &parent, &file.items, test, found);
    if !test {
        Strip.visit_file_mut(&mut file);
    }
    found.push(Source { path: relative(root, path), file, test });
}

fn children(root: &Path, dir: &Path, parent: &Path, items: &[Item], test: bool, found: &mut Vec<Source>) {
    for item in items {
        let Item::Mod(module) = item else { continue };
        let test = test || is_test(&module.attrs);
        let name = module.ident.to_string();
        match &module.content {
            Some((_, inner)) => children(root, &dir.join(&name), parent, inner, test, found),
            None => {
                let (plain, alternatives) = paths(&module.attrs);
                let default = plain.map(|path| parent.join(path)).unwrap_or_else(|| {
                    let flat = dir.join(format!("{name}.rs"));
                    if flat.exists() { flat } else { dir.join(&name).join("mod.rs") }
                });
                for file in std::iter::once(default).chain(alternatives.into_iter().map(|path| parent.join(path))) {
                    walk(root, &file, test, found);
                }
            }
        }
    }
}

fn paths(attrs: &[Attribute]) -> (Option<String>, Vec<String>) {
    let quoted = |attr: &Attribute| attr.to_token_stream().to_string().split_once("path = \"").and_then(|(_, rest)| rest.split_once('"')).map(|(path, _)| path.to_owned());
    let plain = attrs.iter().find(|attr| attr.path().is_ident("path")).and_then(quoted);
    let alternatives = attrs.iter().filter(|attr| attr.path().is_ident("cfg_attr")).filter_map(quoted).collect();
    (plain, alternatives)
}

pub fn is_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("test") || (attr.path().is_ident("cfg") && test_only(&attr.meta)))
}

fn test_only(meta: &Meta) -> bool {
    let Meta::List(list) = meta else { return false };
    let Ok(inner) = list.parse_args::<Meta>() else { return false };
    match &inner {
        Meta::Path(path) => path.is_ident("test"),
        Meta::List(all) if all.path.is_ident("all") => {
            all.parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated).is_ok_and(|parts| parts.iter().any(|part| part.path().is_ident("test")))
        }
        _ => false,
    }
}

struct Strip;

impl VisitMut for Strip {
    fn visit_file_mut(&mut self, file: &mut syn::File) {
        file.items.retain(|item| !item_is_test(item));
        syn::visit_mut::visit_file_mut(self, file);
    }

    fn visit_item_mod_mut(&mut self, module: &mut syn::ItemMod) {
        if let Some((_, items)) = &mut module.content {
            items.retain(|item| !item_is_test(item));
        }
        syn::visit_mut::visit_item_mod_mut(self, module);
    }

    fn visit_item_impl_mut(&mut self, block: &mut syn::ItemImpl) {
        block.items.retain(|item| !matches!(item, syn::ImplItem::Fn(function) if is_test(&function.attrs)));
        syn::visit_mut::visit_item_impl_mut(self, block);
    }

    fn visit_block_mut(&mut self, block: &mut syn::Block) {
        block.stmts.retain(|stmt| !matches!(stmt, syn::Stmt::Item(item) if item_is_test(item)));
        syn::visit_mut::visit_block_mut(self, block);
    }
}

fn item_is_test(item: &Item) -> bool {
    let attrs = match item {
        Item::Mod(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Use(item) => &item.attrs,
        Item::Const(item) => &item.attrs,
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Macro(item) => &item.attrs,
        _ => return false,
    };
    is_test(attrs)
}
