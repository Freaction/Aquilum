const CP1251: [u16; 128] = [
    0x0402, 0x0403, 0x201A, 0x0453, 0x201E, 0x2026, 0x2020, 0x2021, 0x20AC, 0x2030, 0x0409, 0x2039, 0x040A, 0x040C, 0x040B, 0x040F, 0x0452, 0x2018, 0x2019,
    0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0xFFFD, 0x2122, 0x0459, 0x203A, 0x045A, 0x045C, 0x045B, 0x045F, 0x00A0, 0x040E, 0x045E, 0x0408, 0x00A4, 0x0490,
    0x00A6, 0x00A7, 0x0401, 0x00A9, 0x0404, 0x00AB, 0x00AC, 0x00AD, 0x00AE, 0x0407, 0x00B0, 0x00B1, 0x0406, 0x0456, 0x0491, 0x00B5, 0x00B6, 0x00B7, 0x0451,
    0x2116, 0x0454, 0x00BB, 0x0458, 0x0405, 0x0455, 0x0457, 0x0410, 0x0411, 0x0412, 0x0413, 0x0414, 0x0415, 0x0416, 0x0417, 0x0418, 0x0419, 0x041A, 0x041B,
    0x041C, 0x041D, 0x041E, 0x041F, 0x0420, 0x0421, 0x0422, 0x0423, 0x0424, 0x0425, 0x0426, 0x0427, 0x0428, 0x0429, 0x042A, 0x042B, 0x042C, 0x042D, 0x042E,
    0x042F, 0x0430, 0x0431, 0x0432, 0x0433, 0x0434, 0x0435, 0x0436, 0x0437, 0x0438, 0x0439, 0x043A, 0x043B, 0x043C, 0x043D, 0x043E, 0x043F, 0x0440, 0x0441,
    0x0442, 0x0443, 0x0444, 0x0445, 0x0446, 0x0447, 0x0448, 0x0449, 0x044A, 0x044B, 0x044C, 0x044D, 0x044E, 0x044F,
];

fn declared(bytes: &[u8]) -> Option<String> {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(200)]).into_owned();
    let decl = head.strip_prefix('\u{feff}').unwrap_or(&head).trim_start().strip_prefix("<?xml")?.split("?>").next()?.to_owned();
    let at = decl.find("encoding")?;
    let rest = decl[at + 8..].trim_start().strip_prefix('=')?.trim_start();
    let quote = rest.chars().next()?;
    Some(rest[1..].split(quote).next()?.to_ascii_lowercase())
}

pub fn decode(bytes: &[u8]) -> String {
    match declared(bytes).as_deref() {
        Some("windows-1251" | "cp1251") => bytes.iter().map(|&b| if b < 0x80 { char::from(b) } else { char::from_u32(u32::from(CP1251[usize::from(b - 0x80)])).unwrap_or('\u{fffd}') }).collect(),
        _ => {
            let text = String::from_utf8_lossy(bytes);
            text.strip_prefix('\u{feff}').unwrap_or(&text).to_owned()
        }
    }
}

const ENTITIES: [(&str, u32); 40] = [
    ("nbsp", 160), ("iexcl", 161), ("cent", 162), ("pound", 163), ("curren", 164), ("yen", 165), ("brvbar", 166), ("sect", 167), ("uml", 168), ("copy", 169),
    ("ordf", 170), ("laquo", 171), ("not", 172), ("shy", 173), ("reg", 174), ("macr", 175), ("deg", 176), ("plusmn", 177), ("acute", 180), ("micro", 181),
    ("para", 182), ("middot", 183), ("raquo", 187), ("times", 215), ("divide", 247), ("ndash", 8211), ("mdash", 8212), ("lsquo", 8216), ("rsquo", 8217),
    ("sbquo", 8218), ("ldquo", 8220), ("rdquo", 8221), ("bdquo", 8222), ("bull", 8226), ("hellip", 8230), ("prime", 8242), ("euro", 8364), ("trade", 8482),
    ("minus", 8722), ("thinsp", 8201),
];

pub fn parse_xml(text: &str) -> Option<roxmltree::Document<'_>> {
    let options = roxmltree::ParsingOptions { allow_dtd: true, ..roxmltree::ParsingOptions::default() };
    roxmltree::Document::parse_with_options(text, options).ok()
}

pub fn numeric_entities(text: &str) -> String {
    let mut out = text.to_owned();
    for (name, code) in ENTITIES {
        out = out.replace(&format!("&{name};"), &format!("&#{code};"));
    }
    out
}
