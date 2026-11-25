use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use aquilum_health::source::{self, Source};
use aquilum_health::{tokens, verdict};
use quote::ToTokens;
use syn::visit::Visit;

const MIN_TOKENS: usize = 50;
const MIN_MEMBERS: usize = 5;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Place {
    path: String,
    start: usize,
    end: usize,
}

impl Place {
    fn inside(&self, other: &Place) -> bool {
        self.path == other.path && other.start <= self.start && self.end <= other.end
    }

    fn touches(&self, other: &Place) -> bool {
        self.path == other.path && self.start <= other.end + 1 && other.start <= self.end + 1
    }
}

#[derive(Default)]
struct Found {
    code: HashMap<u64, Vec<Place>>,
    types: HashMap<u64, Vec<Place>>,
}

struct Bodies<'a> {
    source: &'a Source,
    found: &'a mut Found,
}

impl Bodies<'_> {
    fn body(&mut self, block: &syn::Block) {
        let mut flat = Vec::new();
        tokens::flat(block.to_token_stream(), &mut flat);
        let hashes: Vec<u64> = flat.iter().map(|(text, _)| hash(text)).collect();
        for (first, window) in hashes.windows(MIN_TOKENS).enumerate() {
            let place = Place { path: self.source.path.clone(), start: flat[first].1, end: flat[first + MIN_TOKENS - 1].1 };
            self.found.code.entry(hash(&window)).or_default().push(place);
        }
    }

    fn members(&mut self, kind: &str, names: Vec<String>, item: proc_macro2::TokenStream) {
        if names.len() >= MIN_MEMBERS {
            let (start, end) = tokens::lines(item);
            self.found.types.entry(hash(&(kind, names))).or_default().push(Place { path: self.source.path.clone(), start, end });
        }
    }
}

impl<'ast> Visit<'ast> for Bodies<'_> {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.body(&item.block);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.body(&item.block);
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if let Some(block) = &item.default {
            self.body(block);
        }
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        let fields = item.fields.iter().map(|field| format!("{}: {}", field.ident.as_ref().map(ToString::to_string).unwrap_or_default(), field.ty.to_token_stream())).collect();
        self.members("struct", fields, item.to_token_stream());
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        let variants = item.variants.iter().map(|variant| variant.ident.to_string()).collect();
        self.members("enum", variants, item.to_token_stream());
    }
}

fn hash(value: &(impl Hash + ?Sized)) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn regions(groups: impl Iterator<Item = Vec<Place>>) -> Vec<Vec<Place>> {
    let mut groups: Vec<Vec<Place>> = groups
        .filter(|places| places.len() > 1)
        .map(|mut places| {
            places.sort();
            places.dedup();
            places
        })
        .filter(|places| places.len() > 1 && !places.windows(2).all(|pair| pair[0].touches(&pair[1])))
        .collect();
    groups.sort();
    let mut merged: Vec<Vec<Place>> = Vec::new();
    for group in groups {
        match merged.iter_mut().rev().find(|last| last.len() == group.len() && last.iter().zip(&group).all(|(a, b)| a.touches(b))) {
            Some(last) => {
                for (a, b) in last.iter_mut().zip(&group) {
                    a.start = a.start.min(b.start);
                    a.end = a.end.max(b.end);
                }
            }
            None => merged.push(group),
        }
    }
    let all = merged.clone();
    merged.retain(|group| !all.iter().any(|outer| outer != group && group.iter().all(|place| outer.iter().any(|wider| place.inside(wider)))));
    merged
}

fn describe(group: &[Place]) -> String {
    group.iter().map(|place| format!("{}:{}-{}", place.path, place.start, place.end)).collect::<Vec<_>>().join("  =  ")
}

#[test]
fn no_copied_code() {
    let sources = source::load();
    let mut found = Found::default();
    for source in source::production(&sources) {
        Bodies { source, found: &mut found }.visit_file(&source.file);
    }
    let code = regions(found.code.into_values()).into_iter().map(|group| format!("одинаковый код: {}", describe(&group)));
    let types = regions(found.types.into_values()).into_iter().map(|group| format!("одинаковые поля или варианты: {}", describe(&group)));
    verdict("дубли", code.chain(types).collect());
}
