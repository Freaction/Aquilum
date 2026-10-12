use masonry::parley::Alignment;

use super::*;
use crate::ui::editor::embed::MediaHit;
use crate::ui::editor::resolve::Request;

impl TextEditor {
    pub(in crate::ui::editor::widget) fn media_press(&mut self, ctx: &mut EventCtx<'_>, point: Point, count: u8) -> bool {
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
                    self.request(Request::View(image, path));
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
                self.changed(ctx);
            }
            MediaHit::Crop if self.media.crop.is_some() => {
                self.end_crop(true);
                self.changed(ctx);
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
                    self.request(Request::View(image, path));
                }
            }
            MediaHit::Outside => return false,
        }
        ctx.request_focus();
        ctx.request_render();
        ctx.set_handled();
        true
    }

    pub(in crate::ui::editor::widget) fn media_move(&mut self, ctx: &mut EventCtx<'_>, point: Point) -> bool {
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
            let start = hover.map(|(s, _)| s);
            if start != self.media.bar.as_ref().map(|(s, _)| *s) {
                self.media.bar = start.map(|s| (s, crate::ui::widgets::Intro::new(crate::ui::widgets::FAST)));
                ctx.request_anim_frame();
            }
            ctx.request_render();
        }
        false
    }

    pub(in crate::ui::editor::widget) fn media_release(&mut self, ctx: &mut EventCtx<'_>) {
        let Some((start, drag)) = self.media.drag.take() else { return };
        if let (Drag::Resize { .. }, Some((_, width))) = (drag, self.doc.resized.take())
            && let Some(mut line) = self.media_view(start).map(|v| v.line.clone())
        {
            line.params.width = Some(width);
            self.rewrite(start, &line);
            self.changed(ctx);
        }
    }
}
