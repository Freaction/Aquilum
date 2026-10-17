use super::{compose, read, single, starts, write, Region};
use crate::text::content::Splice;

fn apply(base: &str, regions: &[Region], text: &str) -> String {
    let mut out = String::new();
    let mut cursor = 0;
    for (region, at) in regions.iter().zip(starts(regions)) {
        out.push_str(&base[cursor..region.from]);
        out.push_str(&text[at..at + region.len]);
        cursor = region.to;
    }
    out.push_str(&base[cursor..]);
    out
}

fn edit(text: &str, at: usize, removed: usize, insert: &str) -> (String, Splice) {
    let next = format!("{}{insert}{}", &text[..at], &text[at + removed..]);
    (next, Splice { at, removed, inserted: insert.len() })
}

#[test]
fn composed_regions_rebuild_every_step_from_the_base() {
    let base = "0123456789abcdefghij";
    let steps = [(5, 0, "XX"), (0, 2, ""), (15, 3, "-"), (4, 4, "abc"), (2, 0, "Q"), (18, 0, "конец"), (0, 0, "")];
    let mut regions = Vec::new();
    let mut text = base.to_owned();
    for (at, removed, insert) in steps {
        let (next, splice) = edit(&text, at, removed, insert);
        regions = compose(&regions, splice);
        text = next;
        assert_eq!(apply(base, &regions, &text), text, "после {at} {removed} {insert:?}");
        assert!(regions.windows(2).all(|pair| pair[0].to < pair[1].from), "{regions:?}");
    }
}

#[test]
fn typing_at_the_end_keeps_one_region() {
    let mut regions = single(Splice { at: 10, removed: 0, inserted: 1 });
    for i in 1..50 {
        regions = compose(&regions, Splice { at: 10 + i, removed: 0, inserted: 1 });
    }
    assert_eq!(regions, [Region { from: 10, to: 10, len: 50 }]);
}

#[test]
fn a_delta_file_keeps_its_regions_and_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("1_dev_me.delta");
    let text = "было, и стало";
    let regions = single(Splice { at: 8, removed: 0, inserted: 1 });
    write(&path, Some("0_dev_start.md"), "hash", &regions, &text).unwrap();
    let delta = read(&path).unwrap();
    assert_eq!((delta.base.as_deref(), delta.hash.as_str(), delta.len, delta.regions.as_slice()), (Some("0_dev_start.md"), "hash", text.len(), regions.as_slice()));
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[delta.body as usize..], b",");
}
