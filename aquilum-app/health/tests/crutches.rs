use aquilum_health::source::{self, Source};
use aquilum_health::{tokens, verdict};
use proc_macro2::TokenTree;
use quote::ToTokens;
use syn::visit::Visit;

const GATEWAYS: [(&[&str], &str, &str); 4] = [
    (&["thread", "spawn"], "core/src/threads.rs", "поток — через aquilum_core::threads"),
    (&["thread", "Builder"], "core/src/threads.rs", "поток — через aquilum_core::threads"),
    (&["Connection", "open"], "core/src/sqlite.rs", "база SQLite — через aquilum_core::sqlite"),
    (&["Connection", "open_with_flags"], "core/src/sqlite.rs", "база SQLite — через aquilum_core::sqlite"),
];
const FORBIDDEN_CALLS: [&str; 4] = ["SendInput", "mouse_event", "keybd_event", "SetCursorPos"];
const FORBIDDEN_MACROS: [&str; 3] = ["todo", "unimplemented", "dbg"];

fn words(source: &Source) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    tokens::each(source.file.to_token_stream(), &mut |tree| {
        if let TokenTree::Ident(ident) = tree {
            found.push((ident.to_string(), ident.span().start().line));
        } else if let TokenTree::Punct(punct) = tree
            && punct.as_char() != ':'
        {
            found.push((punct.to_string(), punct.span().start().line));
        }
    });
    found
}

struct Suppressions<'a> {
    source: &'a Source,
    found: Vec<String>,
}

impl<'ast> Visit<'ast> for Suppressions<'_> {
    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        let text = attr.to_token_stream().to_string();
        if text.contains("allow") && (text.contains("dead_code") || text.contains("unused")) {
            self.found.push(format!("{}:{} `{text}` прячет неиспользуемый код", self.source.path, attr.pound_token.span.start().line));
        }
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(name) = mac.path.segments.last().map(|segment| segment.ident.to_string())
            && FORBIDDEN_MACROS.contains(&name.as_str())
        {
            self.found.push(format!("{}:{} `{name}!` — недоделка в коде", self.source.path, mac.path.segments[0].ident.span().start().line));
        }
        syn::visit::visit_macro(self, mac);
    }
}

#[test]
fn no_suppressed_warnings_or_unfinished_code() {
    let mut findings = Vec::new();
    for source in &source::load() {
        let mut suppressions = Suppressions { source, found: Vec::new() };
        suppressions.visit_file(&source.file);
        findings.extend(suppressions.found);
    }
    verdict("подавление предупреждений и недоделки", findings);
}

#[test]
fn shared_mechanisms_have_one_gateway() {
    let mut findings = Vec::new();
    let sources = source::load();
    for source in source::production(&sources) {
        let words = words(source);
        for (pattern, gateway, rule) in GATEWAYS {
            if source.path == gateway {
                continue;
            }
            for (index, window) in words.windows(pattern.len()).enumerate() {
                if window.iter().zip(pattern).all(|((word, _), expected)| word == expected) {
                    findings.push(format!("{}:{} `{}` — {rule}", source.path, words[index].1, pattern.join("::")));
                }
            }
        }
        for (word, line) in &words {
            if FORBIDDEN_CALLS.contains(&word.as_str()) {
                findings.push(format!("{}:{line} `{word}` — ввод пользователя трогать нельзя", source.path));
            }
        }
    }
    verdict("обход общих механизмов", findings);
}
