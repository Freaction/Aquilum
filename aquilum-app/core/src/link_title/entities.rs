fn named(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" | "ensp" | "emsp" | "thinsp" => Some(' '),
        "copy" => Some('©'),
        "reg" => Some('®'),
        "trade" => Some('™'),
        "bull" => Some('•'),
        "middot" => Some('·'),
        "deg" => Some('°'),
        "plusmn" => Some('±'),
        "times" => Some('×'),
        "divide" => Some('÷'),
        "minus" => Some('−'),
        "euro" => Some('€'),
        "pound" => Some('£'),
        "yen" => Some('¥'),
        "cent" => Some('¢'),
        "rarr" => Some('→'),
        "larr" => Some('←'),
        "uarr" => Some('↑'),
        "darr" => Some('↓'),
        "sect" => Some('§'),
        "para" => Some('¶'),
        "frac12" => Some('½'),
        "ne" => Some('≠'),
        "le" => Some('≤'),
        "ge" => Some('≥'),
        "infin" => Some('∞'),
        "mdash" => Some('—'),
        "ndash" => Some('–'),
        "hellip" => Some('…'),
        "laquo" => Some('«'),
        "raquo" => Some('»'),
        "ldquo" => Some('“'),
        "rdquo" => Some('”'),
        "lsquo" => Some('‘'),
        "rsquo" => Some('’'),
        _ => None,
    }
}

fn numeric(body: &str) -> Option<char> {
    let code = match body.strip_prefix('x').or_else(|| body.strip_prefix('X')) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => body.parse::<u32>().ok()?,
    };
    char::from_u32(code)
}

fn resolve(body: &str) -> Option<char> {
    if body.is_empty() || body.len() > 8 {
        return None;
    }
    match body.strip_prefix('#') {
        Some(number) => numeric(number),
        None => named(&body.to_ascii_lowercase()),
    }
}

pub fn decode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let tail = &rest[start + 1..];
        match tail
            .find(';')
            .and_then(|end| resolve(&tail[..end]).map(|character| (character, end)))
        {
            Some((character, end)) => {
                out.push(character);
                rest = &tail[end + 1..];
            }
            None => {
                out.push('&');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn decodes_named_entities() {
        assert_eq!(decode("Tom &amp; Jerry"), "Tom & Jerry");
        assert_eq!(decode("&laquo;Мозг&raquo;"), "«Мозг»");
    }

    #[test]
    fn decodes_decimal_and_hex_entities() {
        assert_eq!(decode("We&#39;re"), "We're");
        assert_eq!(decode("&#1053;&#x435;&#x439;"), "Ней");
    }

    #[test]
    fn leaves_an_ampersand_that_is_not_an_entity_alone() {
        assert_eq!(decode("R&D and & more"), "R&D and & more");
        assert_eq!(decode("a &notanentity; b"), "a &notanentity; b");
    }

    #[test]
    fn a_run_of_entities_decodes_without_losing_the_text_between_them() {
        assert_eq!(decode("&lt;a&gt; и &lt;b&gt;"), "<a> и <b>");
    }
}
