const SPECIAL: [char; 10] = ['*', '_', '`', '|', '<', '>', '~', '\\', '[', ']'];

pub fn label(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match chars.peek() {
            Some(next) if c == '\\' && next.is_ascii_punctuation() => {
                out.push(c);
                out.extend(chars.next());
            }
            _ => {
                if SPECIAL.contains(&c) {
                    out.push('\\');
                }
                out.push(c);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::label;

    #[test]
    fn markup_characters_are_escaped_once() {
        assert_eq!(label("Текст [с] *скобками*"), r"Текст \[с\] \*скобками\*");
        assert_eq!(label(r"уже \*экранировано\*"), r"уже \*экранировано\*");
        assert_eq!(label(r"C:\Users"), r"C:\\Users");
        assert_eq!(label("конец\\"), "конец\\\\");
    }

    #[test]
    fn escaping_twice_changes_nothing() {
        for text in ["a*b_c", r"x\y", "[1](2)", "<tag> | ~", r"\\", ""] {
            let once = label(text);
            assert_eq!(label(&once), once, "{text}");
        }
    }
}
