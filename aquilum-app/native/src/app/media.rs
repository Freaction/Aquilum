use std::path::PathBuf;

use masonry::app::RenderRoot;
use masonry::kurbo::Point;

use super::App;
use crate::images::{MediaKind, kind};
use crate::ui::editor::{TextEditor, media_line};
use crate::vault_files::{self, Source};

fn natural_width(source: &Source<'_>) -> Option<f64> {
    let (width, _) = match source {
        Source::Path(path) => image::image_dimensions(path).ok()?,
        Source::Bytes(bytes, _) => crate::images::dimensions(bytes)?,
    };
    Some(f64::from(width))
}

impl App {
    fn paste_formatted(&self, root: &mut RenderRoot) -> bool {
        let Some(ids) = self.note_ids.as_ref().filter(|ids| root.focused_widget() == Some(ids.body.id())) else { return false };
        if !self.workspace.core.settings.get_config().editor.paste_formatting {
            return false;
        }
        let Some(markdown) = crate::clipboard::html().and_then(|html| aquilum_core::markdown::html::to_markdown(&html)) else { return false };
        ids.body.edit(root, |mut editor| TextEditor::paste_markdown(&mut editor, &markdown)).unwrap_or(false)
    }

    pub fn paste(&mut self, root: &mut RenderRoot, plain: bool) -> bool {
        if !plain && self.paste_formatted(root) {
            return true;
        }
        let text = crate::clipboard::text();
        if text.as_deref().is_none_or(clipboard_image_uri) && crate::clipboard::png().is_some_and(|png| self.paste_image(root, &png)) {
            return true;
        }
        let Some(text) = text else { return false };
        root.handle_text_event(masonry::core::TextEvent::ClipboardPaste(text));
        true
    }

    pub(in crate::app) fn paste_image(&mut self, root: &mut RenderRoot, png: &[u8]) -> bool {
        if self.note_ids.as_ref().is_none_or(|ids| root.focused_widget() != Some(ids.body.id())) {
            return false;
        }
        self.insert_media(root, &[Source::Bytes(png, "image.png")], None)
    }

    pub fn drop_files(&mut self, root: &mut RenderRoot, paths: &[PathBuf], at: Option<Point>) {
        let media: Vec<Source<'_>> = paths.iter().filter(|p| kind(&p.to_string_lossy()).is_some()).map(|p| Source::Path(p)).collect();
        self.insert_media(root, &media, at);
    }

    pub(in crate::app) fn insert_media(&mut self, root: &mut RenderRoot, sources: &[Source<'_>], at: Option<Point>) -> bool {
        let (Some(ids), Some(workspace)) = (&self.note_ids, self.tree.root()) else { return false };
        let Some(body) = root.get_widget(ids.body.id()) else { return false };
        let column = body.ctx().bounding_box().width();
        let lines: Vec<String> = sources
            .iter()
            .filter_map(|source| {
                let video = match source {
                    Source::Path(path) => kind(&path.to_string_lossy()) == Some(MediaKind::Video),
                    Source::Bytes(_, name) => kind(name) == Some(MediaKind::Video),
                };
                let (name, ext) = if video { ("video", "mp4") } else { ("image", "png") };
                let relative = vault_files::import(&self.workspace.core, workspace, *source, name, ext).ok()?;
                let width = if video { Some(column) } else { natural_width(source).map(|w| w.min(column)) };
                Some(media_line(&relative, width))
            })
            .collect();
        !lines.is_empty() && ids.body.edit(root, |mut editor| TextEditor::insert_media(&mut editor, &lines, at)).unwrap_or(false)
    }
}

fn clipboard_image_uri(text: &str) -> bool {
    let text = text.trim();
    text.is_empty() || text.starts_with("content:")
}
