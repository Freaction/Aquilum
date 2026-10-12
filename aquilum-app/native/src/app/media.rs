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
        Source::Bytes(bytes, _) => image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().ok()?.into_dimensions().ok()?,
    };
    Some(f64::from(width))
}

impl App {
    pub fn paste_image(&mut self, root: &mut RenderRoot, png: &[u8]) {
        self.insert_media(root, &[Source::Bytes(png, "image.png")], None);
    }

    pub fn drop_files(&mut self, root: &mut RenderRoot, paths: &[PathBuf], at: Point) {
        let media: Vec<Source<'_>> = paths.iter().filter(|p| kind(&p.to_string_lossy()).is_some()).map(|p| Source::Path(p)).collect();
        self.insert_media(root, &media, Some(at));
    }

    fn insert_media(&mut self, root: &mut RenderRoot, sources: &[Source<'_>], at: Option<Point>) {
        let (Some(ids), Some(workspace)) = (&self.note_ids, self.tree.root()) else { return };
        let Some(body) = root.get_widget(ids.body.id()) else { return };
        let column = body.ctx().bounding_box().width();
        let editor = body.downcast::<TextEditor>().map(|e| e.inner());
        let Some(offset) = editor.map(|e| at.map_or(e.caret_offset(), |p| e.offset_at(body.ctx().window_transform().inverse(), p))) else { return };
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
        if !lines.is_empty() {
            ids.body.edit(root, |mut editor| TextEditor::insert_block(&mut editor, offset, &lines));
        }
    }
}
