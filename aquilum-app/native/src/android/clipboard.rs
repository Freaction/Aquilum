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
    None
}

pub fn release() {}
