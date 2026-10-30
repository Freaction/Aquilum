use std::fmt;
use std::path::{Path, PathBuf};

pub struct Destination {
    name: String,
    target: imp::Target,
}

impl fmt::Display for Destination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

#[cfg(not(target_os = "android"))]
mod imp {
    use super::*;

    pub type Target = PathBuf;

    impl Destination {
        pub fn write(&self, bytes: &[u8]) -> Result<(), String> {
            std::fs::write(&self.target, bytes).map_err(|error| error.to_string())
        }
    }

    pub fn pick_file(filter: &str, extensions: &[&str]) -> Option<PathBuf> {
        rfd::FileDialog::new().add_filter(filter, extensions).pick_file()
    }

    pub fn pick_folder(start: Option<&Path>) -> Option<PathBuf> {
        let picker = rfd::FileDialog::new();
        match start {
            Some(start) => picker.set_directory(start),
            None => picker,
        }
        .pick_folder()
    }

    pub fn save_file(filter: &str, extensions: &[&str], name: &str) -> Option<Destination> {
        let path = rfd::FileDialog::new().add_filter(filter, extensions).set_file_name(name).save_file()?;
        Some(Destination { name: path.display().to_string(), target: path })
    }

    pub fn confirm(title: &str, text: &str) -> bool {
        rfd::MessageDialog::new().set_title(title).set_description(text).set_buttons(rfd::MessageButtons::YesNo).show() == rfd::MessageDialogResult::Yes
    }

    pub fn alert(text: &str) {
        rfd::MessageDialog::new().set_title("Aquilum").set_description(text).set_level(rfd::MessageLevel::Error).show();
    }
}

#[cfg(target_os = "android")]
mod imp {
    use super::*;
    use crate::android::documents;

    pub type Target = String;

    impl Destination {
        pub fn write(&self, bytes: &[u8]) -> Result<(), String> {
            documents::write(&self.target, bytes)
        }
    }

    pub fn pick_file(_: &str, extensions: &[&str]) -> Option<PathBuf> {
        documents::pick(extensions)
    }

    pub fn pick_folder(_: Option<&Path>) -> Option<PathBuf> {
        None
    }

    pub fn save_file(_: &str, _: &[&str], name: &str) -> Option<Destination> {
        documents::create(name).map(|uri| Destination { name: name.to_owned(), target: uri })
    }

    pub fn confirm(_: &str, _: &str) -> bool {
        false
    }
}

pub use imp::*;
