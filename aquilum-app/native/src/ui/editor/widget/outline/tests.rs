use super::renumber;

#[test]
fn renumbering_touches_only_the_edited_list() {
    let text = "1. раз\n3. два\n\nабзац\n\n1. один\n5. другой";
    let at = text.find("3.").unwrap();
    let (range, numbers) = renumber(text, at..at).unwrap();
    let mut fixed = text.to_owned();
    fixed.replace_range(range, &numbers);
    assert_eq!(fixed, "1. раз\n2. два\n\nабзац\n\n1. один\n5. другой");
    let at = text.find("один").unwrap();
    let (range, numbers) = renumber(text, at..at).unwrap();
    let mut fixed = text.to_owned();
    fixed.replace_range(range, &numbers);
    assert_eq!(fixed, "1. раз\n3. два\n\nабзац\n\n1. один\n2. другой");
    assert!(renumber(text, 0..0).is_some());
    assert!(renumber("1. а\n2. б\n\nтекст", 0..0).is_none());
}
