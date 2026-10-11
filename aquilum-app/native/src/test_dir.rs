pub fn tempdir() -> tempfile::TempDir {
    let root = crate::workspace::normalize(&std::env::temp_dir());
    tempfile::Builder::new().tempdir_in(root).expect("временный каталог")
}

pub fn command() -> masonry::core::keyboard::Modifiers {
    if cfg!(target_os = "macos") { masonry::core::keyboard::Modifiers::META } else { masonry::core::keyboard::Modifiers::CONTROL }
}
