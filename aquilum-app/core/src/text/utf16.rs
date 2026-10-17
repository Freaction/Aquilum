pub fn utf16_len(text: &str) -> usize {
    text.chars().count() + text.as_bytes().iter().map(|&byte| usize::from(byte >= 0xF0)).sum::<usize>()
}

pub fn utf16_seek(text: &str, (mut byte, mut seen): (usize, usize), units: usize) -> (usize, usize) {
    for character in text[byte..].chars() {
        if seen >= units {
            break;
        }
        seen += character.len_utf16();
        byte += character.len_utf8();
    }
    (byte, seen)
}

pub fn utf16_byte(text: &str, units: usize) -> usize {
    utf16_seek(text, (0, 0), units).0
}

pub fn utf16_slice(text: &str, from: usize, to: usize) -> &str {
    let start = utf16_seek(text, (0, 0), from);
    &text[start.0..utf16_seek(text, start, to).0]
}

#[cfg(test)]
mod tests {
    use super::{utf16_byte, utf16_len, utf16_seek, utf16_slice};

    #[test]
    fn a_slice_takes_the_characters_that_start_inside_the_range() {
        let text = "аб😀в";
        assert_eq!(utf16_slice(text, 1, 4), "б😀");
        assert_eq!(utf16_slice(text, 3, 5), "в");
        assert_eq!(utf16_slice(text, 2, 3), "😀");
        assert_eq!(utf16_slice(text, 4, 1), "");
        assert_eq!(utf16_slice(text, 0, 99), text);
    }

    #[test]
    fn utf16_length_matches_the_encoder_for_every_character_width() {
        let text = "abc кириллица 中文字 😀🎉 𝄞 ё\n".repeat(37);
        for end in (0..=text.len()).filter(|&i| text.is_char_boundary(i)) {
            assert_eq!(utf16_len(&text[..end]), text[..end].encode_utf16().count());
        }
    }

    #[test]
    fn a_utf16_offset_lands_on_the_same_character() {
        let text = "аб😀в";
        assert_eq!(&text[utf16_byte(text, 4)..], "в");
        assert_eq!(utf16_byte(text, 3), utf16_byte(text, 4));
        assert_eq!(utf16_byte(text, 99), text.len());
    }

    #[test]
    fn seeking_from_a_midpoint_matches_seeking_from_the_start() {
        let text = "аб😀в г";
        let middle = utf16_seek(text, (0, 0), 2);
        for units in 2..8 {
            assert_eq!(utf16_seek(text, middle, units).0, utf16_byte(text, units));
        }
    }
}
