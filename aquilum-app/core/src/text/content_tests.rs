use super::{chars_below, hash, splice, Splice, BLOCK};

#[test]
fn the_splice_is_the_part_between_the_common_ends() {
    assert_eq!(splice(&"было и стало", &"было, и стало").unwrap(), Splice { at: 8, removed: 0, inserted: 1 });
    assert_eq!(splice(&"abc", &"abc").unwrap(), Splice { at: 3, removed: 0, inserted: 0 });
    assert_eq!(splice(&"aaaa", &"aa").unwrap(), Splice { at: 2, removed: 2, inserted: 0 });
    assert_eq!(splice(&"", &"новое").unwrap(), Splice { at: 0, removed: 0, inserted: 10 });
}

#[test]
fn the_ends_are_found_across_blocks() {
    let old = "x".repeat(BLOCK * 3 + 17);
    let mut new = old.clone();
    new.insert_str(BLOCK * 2 + 5, "вставка");
    assert_eq!(splice(&old.as_str(), &new.as_str()).unwrap(), Splice { at: BLOCK * 2 + 5, removed: 0, inserted: "вставка".len() });
    new.push('!');
    let found = splice(&old.as_str(), &new.as_str()).unwrap();
    assert_eq!((found.at, found.at + found.removed), (BLOCK * 2 + 5, old.len()));
}

#[test]
fn the_hash_is_the_hash_of_the_bytes() {
    assert_eq!(hash(&"текст").unwrap(), blake3::hash("текст".as_bytes()).to_hex().to_string());
}

#[test]
fn characters_are_counted_not_bytes() {
    let text = "я".repeat(150);
    assert!(chars_below(&text.as_str(), [0..text.len()], 200).unwrap());
    assert!(!chars_below(&text.as_str(), [0..text.len()], 150).unwrap());
    assert!(!chars_below(&text.as_str(), [0..text.len() / 2, text.len() / 2..text.len()], 150).unwrap(), "символы считаются по всем кускам вместе");
}
