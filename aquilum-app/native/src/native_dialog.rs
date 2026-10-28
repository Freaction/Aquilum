use std::path::{Path, PathBuf};

pub const PICKS_FOLDERS: bool = !cfg!(target_os = "android");

#[cfg(not(target_os = "android"))]
mod imp {
    use super::*;

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

    pub fn save_file(filter: &str, extensions: &[&str], name: &str) -> Option<PathBuf> {
        rfd::FileDialog::new().add_filter(filter, extensions).set_file_name(name).save_file()
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

    pub fn pick_file(_: &str, _: &[&str]) -> Option<PathBuf> {
        None
    }

    pub fn pick_folder(_: Option<&Path>) -> Option<PathBuf> {
        None
    }

    pub fn save_file(_: &str, _: &[&str], _: &str) -> Option<PathBuf> {
        None
    }

    pub fn confirm(_: &str, _: &str) -> bool {
        false
    }
}

pub use imp::*;
