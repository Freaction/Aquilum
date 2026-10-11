pub fn tempdir() -> tempfile::TempDir {
    let root = crate::workspace::normalize(&std::env::temp_dir());
    tempfile::Builder::new().tempdir_in(root).expect("временный каталог")
}
