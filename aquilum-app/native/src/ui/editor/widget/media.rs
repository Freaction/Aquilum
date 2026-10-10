use masonry::core::keyboard::KeyboardEvent;
use masonry::parley::Alignment;

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
}

impl TextEditor {
    fn media_view(&self, start: usize) -> Option<&MediaView> {
        let p = self.doc.paragraph(self.doc.at_offset(start));
        match (p.start == start, p.embed.as_deref().map(|e| &e.view)) {
            (true, Some(View::Media(view))) => Some(view),
            _ => None,
        }
    }

    fn media_under(&self, point: Point) -> Option<(usize, MediaHit)> {
        if self.doc.markdown.is_none() || !self.laid_out() {
            return None;
        }
        let p = self.doc.paragraph(self.doc.at_y(point.y.max(0.0)));
        let Some(View::Media(view)) = p.embed.as_deref().map(|e| &e.view).filter(|_| matches!(p.fold, Fold::Head | Fold::Above)) else { return None };
        let local = Point::new(point.x - self.shift().x, point.y - p.top);
        if local.y >= view.height {
            return None;
        }
        let overlay = MediaOverlay { hovered: true, ..self.media_overlay(p.start) };
        Some((p.start, view.hit(local, &overlay))).filter(|(_, hit)| *hit != MediaHit::Outside)
    }

    pub(super) fn paint_videos(&self, painter: &mut masonry::imaging::Painter<'_>) {
        let (first, last) = (self.doc.at_y(self.painted.0), self.doc.at_y(self.painted.1));
        for p in &self.doc.paragraphs()[first..=last] {
            if let (Fold::Head | Fold::Above, Some(View::Media(view))) = (p.fold, p.embed.as_deref().map(|e| &e.view)) {
                let overlay = self.media_overlay(p.start);
                if overlay.video.is_some() {
                    view.paint(painter, self.shift().x, p.top, &overlay);
                }
            }
        }
    }

    pub(super) fn media_overlay(&self, start: usize) -> MediaOverlay {
        let m = &self.media;
        let crop = m.crop.filter(|(s, _)| *s == start).map(|(_, c)| c);
        let hover = m.hover.filter(|(s, _)| *s == start).map(|(_, h)| h);
        let video = self.media_view(start).filter(|v| v.video).and_then(|v| v.path.as_deref()).filter(|p| self.videos.has(p)).and_then(crate::video::state);
        MediaOverlay { hovered: hover.is_some() || crop.is_some(), selected: m.selected == Some(start), hover, crop, video }
    }

    fn rewrite(&mut self, start: usize, line: &EmbedLine) {
        let range = self.doc.paragraph(self.doc.at_offset(start)).range();
        let text = media::format(line);
        if self.doc.text()[range.clone()] == text {
            return;
        }
        let keep = (self.anchor, self.focus);
        self.replace(range, &text, Kind::Table);
        (self.anchor, self.focus) = (keep.0.min(self.doc.len()), keep.1.min(self.doc.len()));
    }

    fn end_crop(&mut self, apply: bool) {
        let Some((start, crop)) = self.media.crop.take() else { return };
        self.doc.uncropped = None;
        if apply && let Some(mut line) = self.media_view(start).map(|v| v.line.clone()) {
            line.params.crop = (crop != FULL).then_some(crop);
            self.rewrite(start, &line);
        }
    }

    fn media_changed(&mut self, ctx: &mut EventCtx<'_>) {
        ctx.submit_action::<TextAction>(TextAction::Changed(self.doc.text().to_owned()));
        ctx.request_layout();
        ctx.request_render();
    }

    fn show_source(&mut self, ctx: &mut EventCtx<'_>, start: usize) {
        self.end_crop(false);
        self.media.selected = None;
        let at = (start + 2).min(self.doc.len());
        self.anchor = at;
        self.focus = at;
        ctx.request_focus();
        ctx.request_layout();
    }

    fn request(&self, request: Request) {
        if let Some(resolver) = &self.doc.resolver {
            resolver.request(request);
        }
    }

    pub(super) fn media_press(&mut self, ctx: &mut EventCtx<'_>, point: Point, count: u8) -> bool {
        let under = self.media_under(point);
        if self.media.crop.is_some_and(|(s, _)| under.is_none_or(|(u, _)| u != s)) {
            self.end_crop(false);
            ctx.request_layout();
        }
        let Some((start, hit)) = under else {
            if self.media.selected.take().is_some() {
                ctx.request_render();
            }
            return false;
        };
        let Some(view) = self.media_view(start) else { return false };
        let (line, frame, video, image, path, crop_now) = (view.line.clone(), view.frame, view.video, view.image.clone(), view.path.clone(), view.crop());
        match hit {
            MediaHit::Zoom => {
                if let Some(image) = image {
                    self.request(Request::View(image));
                }
            }
            MediaHit::Align => {
                let mut line = line;
                line.params.align = match line.params.align {
                    Alignment::Start | Alignment::Left => Alignment::Center,
                    Alignment::End | Alignment::Right => Alignment::Start,
                    _ => Alignment::End,
                };
                self.rewrite(start, &line);
                self.media_changed(ctx);
            }
            MediaHit::Crop if self.media.crop.is_some() => {
                self.end_crop(true);
                self.media_changed(ctx);
            }
            MediaHit::Crop => {
                self.media.crop = Some((start, line.params.crop.unwrap_or(FULL)));
                self.doc.uncropped = Some(start);
                ctx.request_layout();
            }
            MediaHit::Source => self.show_source(ctx, start),
            MediaHit::Grip => {
                let step = if matches!(line.params.align, Alignment::Center) { 2.0 } else { 1.0 };
                self.media.drag = Some((start, Drag::Resize { x0: point.x, w0: frame.width(), step }));
                ctx.capture_pointer();
            }
            MediaHit::Play | MediaHit::Toggle => {
                if let Some(path) = path {
                    if crate::video::toggle(&path) {
                        self.videos.add(path);
                        ctx.request_anim_frame();
                    } else {
                        self.request(Request::Open { target: path.to_string_lossy().into_owned(), wiki: false, new_tab: false });
                    }
                }
            }
            MediaHit::Mute => {
                if let Some(path) = &path {
                    crate::video::mute(path);
                }
            }
            MediaHit::Bar => {
                if let Some(path) = &path {
                    let top = self.doc.paragraph(self.doc.at_offset(start)).top;
                    let local = Point::new(point.x - self.shift().x, point.y - top);
                    if let Some(view) = self.media_view(start) {
                        crate::video::seek(path, view.seek_fraction(local));
                    }
                    self.media.drag = Some((start, Drag::Seek));
                    ctx.capture_pointer();
                    ctx.request_anim_frame();
                }
            }
            MediaHit::Handle(i) => {
                let from = self.media.crop.map_or(crop_now, |(_, c)| c);
                self.media.drag = Some((start, Drag::Crop { handle: Some(i), at: point, from }));
                ctx.capture_pointer();
            }
            MediaHit::CropBox => {
                let from = self.media.crop.map_or(crop_now, |(_, c)| c);
                self.media.drag = Some((start, Drag::Crop { handle: None, at: point, from }));
                ctx.capture_pointer();
            }
            MediaHit::Image => {
                self.media.selected = Some(start);
                if count >= 2
                    && !video
                    && let Some(image) = image
                {
                    self.request(Request::View(image));
                }
            }
            MediaHit::Outside => return false,
        }
        ctx.request_focus();
        ctx.request_render();
        ctx.set_handled();
        true
    }

    pub(super) fn media_move(&mut self, ctx: &mut EventCtx<'_>, point: Point) -> bool {
        if let Some((start, drag)) = self.media.drag {
            let Some(view) = self.media_view(start) else { return false };
            match drag {
                Drag::Resize { x0, w0, step } => {
                    let width = (w0 + (point.x - x0) * step).clamp(view.min_width().min(view.room), view.room).round();
                    if self.doc.resized != Some((start, width)) {
                        self.doc.resized = Some((start, width));
                        ctx.request_layout();
                        ctx.request_render();
                    }
                }
                Drag::Seek => {
                    let top = self.doc.paragraph(self.doc.at_offset(start)).top;
                    let local = Point::new(point.x - self.shift().x, point.y - top);
                    if let Some(path) = &view.path {
                        crate::video::seek(path, view.seek_fraction(local));
                    }
                    ctx.request_anim_frame();
                }
                Drag::Crop { handle, at, from } => {
                    let crop = view.drag_crop(from, handle, point.x - at.x, point.y - at.y);
                    self.media.crop = Some((start, crop));
                    ctx.request_render();
                }
            }
            return true;
        }
        let hover = self.media_under(point);
        if hover != self.media.hover {
            self.media.hover = hover;
            ctx.request_render();
        }
        false
    }

    pub(super) fn media_leave(&mut self, ctx: &mut EventCtx<'_>) {
        if self.media.drag.is_none() && self.media.hover.take().is_some() {
            ctx.request_render();
        }
    }

    pub(super) fn media_release(&mut self, ctx: &mut EventCtx<'_>) -> bool {
        let Some((start, drag)) = self.media.drag.take() else { return false };
        if let (Drag::Resize { .. }, Some((_, width))) = (drag, self.doc.resized.take())
            && let Some(mut line) = self.media_view(start).map(|v| v.line.clone())
        {
            line.params.width = Some(width);
            self.rewrite(start, &line);
            self.media_changed(ctx);
        }
        true
    }

    pub(super) fn media_key(&mut self, ctx: &mut EventCtx<'_>, event: &KeyboardEvent) -> bool {
        let target = self.media.crop.map(|(s, _)| s).or(self.media.selected);
        let Some(start) = target else { return false };
        if event.state != KeyState::Down {
            return true;
        }
        let cropping = self.media.crop.is_some();
        match &event.key {
            Key::Named(NamedKey::Enter) if cropping => {
                self.end_crop(true);
                self.media_changed(ctx);
            }
            Key::Named(NamedKey::Escape) if cropping => {
                self.end_crop(false);
                ctx.request_layout();
            }
            Key::Named(NamedKey::Enter) => self.show_source(ctx, start),
            Key::Named(NamedKey::Escape) => self.media.selected = None,
            Key::Named(NamedKey::Backspace | NamedKey::Delete) => {
                let p = self.doc.paragraph(self.doc.at_offset(start));
                let range = p.start..(p.end() + 1).min(self.doc.len());
                self.media.selected = None;
                self.media.hover = None;
                self.replace(range, "", Kind::Other);
                self.anchor = start.min(self.doc.len());
                self.focus = self.anchor;
                self.media_changed(ctx);
            }
            _ => {
                self.media.selected = None;
                ctx.request_render();
                return false;
            }
        }
        ctx.set_handled();
        ctx.request_render();
        true
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
