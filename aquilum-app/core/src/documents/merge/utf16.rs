pub fn utf16_len(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

pub fn utf16_byte(text: &str, units: usize) -> usize {
    let mut seen = 0;
    for (byte, character) in text.char_indices() {
        if seen >= units {
            return byte;
        }
        seen += character.len_utf16();
    }
    text.len()
}

pub fn utf16_slice(text: &str, from: usize, to: usize) -> String {
    let mut position = 0;
    let mut result = String::new();
    for character in text.chars() {
        if position >= to {
            break;
        }
        if position >= from {
            result.push(character);
        }
        position += character.len_utf16();
    }
    result
}
