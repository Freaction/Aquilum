mod keys;
mod mouse;
#[cfg(feature = "memory-probe")]
mod probe;

use super::*;
use crate::ui::editor::embed::media::{self, EmbedLine};
use crate::ui::editor::embed::{MediaHit, MediaOverlay, MediaView, View};
use crate::ui::editor::resolve::Request;

const FULL: [f64; 4] = [0.0, 0.0, 100.0, 100.0];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    Resize { x0: f64, w0: f64, step: f64 },
    Crop { handle: Option<u8>, at: Point, from: [f64; 4] },
    Seek,
}

#[derive(Default)]
pub(super) struct MediaState {
    hover: Option<(usize, MediaHit)>,
    selected: Option<usize>,
    crop: Option<(usize, [f64; 4])>,
    drag: Option<(usize, Drag)>,
    bar: Option<(usize, crate::ui::widgets::Intro)>,
}

impl MediaState {
    pub(super) fn remap(&mut self, map: impl Fn(usize) -> usize) {
        self.hover = self.hover.map(|(start, hit)| (map(start), hit));
        self.selected = self.selected.map(&map);
        self.crop = self.crop.map(|(start, crop)| (map(start), crop));
        self.drag = self.drag.map(|(start, drag)| (map(start), drag));
        if let Some((start, _)) = &mut self.bar {
            *start = map(*start);
        }
    }
}

impl TextEditor {
    fn media_view(&self, start: usize) -> Option<&MediaView> {
        let p = self.doc.paragraph(self.doc.at_offset(start));
        match (p.start == start, p.embed.map(|e| &e.view)) {
            (true, Some(View::Media(view))) => Some(view),
            _ => None,
        }
    }

    fn media_under(&self, point: Point) -> Option<(usize, MediaHit)> {
        if self.doc.markdown.is_none() || !self.laid_out() {
            return None;
        }
        let i = self.doc.at_y(point.y.max(0.0));
        let p = self.doc.paragraph(i);
        let Some(View::Media(view)) = p.embed.map(|e| &e.view).filter(|_| matches!(p.fold, Fold::Head | Fold::Above)) else { return None };
        let local = Point::new(point.x - self.shift().x, point.y - self.doc.top(i));
        if local.y >= view.height {
            return None;
        }
        let overlay = MediaOverlay { hovered: true, ..self.media_overlay(p.start) };
        Some((p.start, view.hit(local, &overlay))).filter(|(_, hit)| *hit != MediaHit::Outside)
    }

    pub(super) fn paint_videos(&self, painter: &mut masonry::imaging::Painter<'_>) {
        let (first, last) = (self.doc.at_y(self.painted.0), self.doc.at_y(self.painted.1));
        for (i, top, _) in self.doc.placed(first..=last) {
            let p = self.doc.paragraph(i);
            if let (Fold::Head | Fold::Above, Some(View::Media(view))) = (p.fold, p.embed.map(|e| &e.view)) {
                let overlay = self.media_overlay(p.start);
                if overlay.video.is_some() {
                    view.paint(painter, self.shift().x, top, &overlay);
                }
            }
        }
    }

    pub(super) fn media_overlay(&self, start: usize) -> MediaOverlay {
        let m = &self.media;
        let crop = m.crop.filter(|(s, _)| *s == start).map(|(_, c)| c);
        let hover = m.hover.filter(|(s, _)| *s == start).map(|(_, h)| h);
        let video = self.media_view(start).filter(|v| v.video).and_then(|v| v.path.as_deref()).filter(|p| self.videos.has(p)).and_then(crate::video::state);
        let fade = m.bar.as_ref().filter(|(s, _)| *s == start).map_or(1.0, |(_, intro)| intro.value() as f32);
        MediaOverlay { hovered: hover.is_some() || crop.is_some(), selected: m.selected == Some(start), hover, crop, video, fade }
    }

    fn rewrite(&mut self, start: usize, line: &EmbedLine) {
        let range = self.doc.paragraph(self.doc.at_offset(start)).range();
        let text = media::format(line);
        if self.doc.slice(range.clone()) != text {
            self.dispatch(vec![(range, text)], None, Kind::Block);
        }
    }

    fn end_crop(&mut self, apply: bool) {
        let Some((start, crop)) = self.media.crop.take() else { return };
        self.doc.uncropped = None;
        if apply && let Some(mut line) = self.media_view(start).map(|v| v.line.clone()) {
            line.params.crop = (crop != FULL).then_some(crop);
            self.rewrite(start, &line);
        }
    }

    fn show_source(&mut self, ctx: &mut EventCtx<'_>, start: usize) {
        self.end_crop(false);
        self.media.selected = None;
        let at = (start + 2).min(self.doc.len());
        self.anchor = at;
        self.focus = at;
        self.doc.source = Some(start);
        ctx.request_focus();
        ctx.request_layout();
    }

    fn request(&self, request: Request) {
        if let Some(resolver) = &self.doc.resolver {
            resolver.request(request);
        }
    }

    pub(super) fn media_tick(&mut self, ctx: &mut UpdateCtx<'_>, interval: u64) {
        if let Some((_, intro)) = &mut self.media.bar {
            if intro.advance(interval) {
                ctx.request_anim_frame();
            }
            ctx.request_render();
        }
    }

    pub(super) fn media_leave(&mut self, ctx: &mut EventCtx<'_>) {
        if self.media.drag.is_none() && self.media.hover.take().is_some() {
            ctx.request_render();
        }
    }

    pub(super) fn media_cursor(&self) -> Option<CursorIcon> {
        if let Some((_, Drag::Resize { .. })) = self.media.drag {
            return Some(CursorIcon::NwseResize);
        }
        Some(match self.media.hover?.1 {
            MediaHit::Grip | MediaHit::Handle(0 | 4) => CursorIcon::NwseResize,
            MediaHit::Handle(2 | 6) => CursorIcon::NeswResize,
            MediaHit::Handle(1 | 5) => CursorIcon::NsResize,
            MediaHit::Handle(_) => CursorIcon::EwResize,
            MediaHit::Zoom | MediaHit::Align | MediaHit::Crop | MediaHit::Source | MediaHit::Play | MediaHit::Toggle | MediaHit::Mute | MediaHit::Bar => CursorIcon::Pointer,
            MediaHit::CropBox => CursorIcon::Move,
            MediaHit::Image | MediaHit::Outside => CursorIcon::Default,
        })
    }
}
