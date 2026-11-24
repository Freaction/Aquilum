use jni::jni_str;

use super::java;

pub fn copy(text: impl Into<String>) -> bool {
    java::send_text(jni_str!("copyText"), &text.into())
}

pub fn text() -> Option<String> {
    java::read_text(jni_str!("pasteText"))
}

pub fn html() -> Option<String> {
    java::read_text(jni_str!("pasteHtml"))
}

pub fn png() -> Option<Vec<u8>> {
    let path = java::read_text(jni_str!("pasteImage"))?;
    let bytes = std::fs::read(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    let image = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let mut out = std::io::Cursor::new(Vec::new());
    image.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

pub fn release() {}
