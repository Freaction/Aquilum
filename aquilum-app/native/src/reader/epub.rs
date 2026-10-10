use std::collections::HashMap;

use super::cfi::{Cfi, element_path};
use super::dom::{Dom, NodeId};
use super::text::{decode, numeric_entities, parse_xml};
use super::zip::Zip;

pub struct Epub {
    zip: Zip,
    pub sections: Vec<(String, String)>,
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(byte) = text.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_owned())
}

fn join(base: &str, href: &str) -> String {
    let href = percent_decode(href.split('#').next().unwrap_or(href));
    let mut parts: Vec<&str> = base.split('/').filter(|p| !p.is_empty()).collect();
    for part in href.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    parts.join("/")
}

fn descendants<'a>(dom: &'a Dom, root: NodeId, name: &'a str) -> impl Iterator<Item = NodeId> + 'a {
    let mut stack = vec![root];
    std::iter::from_fn(move || {
        while let Some(node) = stack.pop() {
            stack.extend(dom.node(node).children.iter().rev().copied());
            if dom.name(node) == Some(name) {
                return Some(node);
            }
        }
        None
    })
}

pub fn document(bytes: &[u8]) -> Option<Dom> {
    let text = decode(bytes);
    if let Some(doc) = parse_xml(&text) {
        return Some(Dom::from_xml(&doc));
    }
    let fixed = numeric_entities(&text);
    parse_xml(&fixed).map(|doc| Dom::from_xml(&doc))
}

impl Epub {
    pub fn open(data: Vec<u8>) -> Option<Epub> {
        let zip = Zip::new(data)?;
        let container = document(&zip.read("META-INF/container.xml")?)?;
        let rootfile = descendants(&container, Dom::ROOT, "rootfile").next()?;
        let opf_path = container.attr(rootfile, "full-path")?.to_owned();
        let opf = document(&zip.read(&opf_path)?)?;
        let folder = opf_path.rsplit_once('/').map_or("", |(dir, _)| dir).to_owned();
        let manifest: HashMap<&str, &str> = descendants(&opf, Dom::ROOT, "item").filter_map(|item| Some((opf.attr(item, "id")?, opf.attr(item, "href")?))).collect();
        let spine = descendants(&opf, Dom::ROOT, "spine").next()?;
        let sections = opf
            .node(spine)
            .children
            .iter()
            .copied()
            .filter(|&c| opf.name(c) == Some("itemref"))
            .map(|itemref| {
                let href = opf.attr(itemref, "idref").and_then(|id| manifest.get(id)).map_or(String::new(), |href| join(&folder, href));
                (href, Cfi::Point(vec![element_path(&opf, itemref)]).to_string())
            })
            .collect();
        Some(Epub { zip, sections })
    }

    pub fn size(&self, index: usize) -> usize {
        self.sections.get(index).and_then(|(path, _)| self.zip.size(path)).unwrap_or(0)
    }

    pub fn resource(&self, index: usize, src: &str) -> Option<Vec<u8>> {
        let (path, _) = self.sections.get(index)?;
        let folder = path.rsplit_once('/').map_or("", |(dir, _)| dir);
        self.zip.read(&join(folder, src))
    }

    pub fn section(&self, index: usize) -> Option<Dom> {
        let (path, _) = self.sections.get(index)?;
        document(&self.zip.read(path)?)
    }
}
