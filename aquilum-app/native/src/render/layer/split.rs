use masonry::imaging::record::Glyph;
use masonry::imaging::{BlurredRoundedRect, ClipRef, FillRef, GlyphRunRef, GroupRef, PaintSink, StrokeRef};
use masonry::peniko::BrushRef;
use vello_cpu::{PixelFormat, PixmapMut, RasterizerSettings};

use crate::render::paint::{PaintError, Painter};
use crate::render::renderer::{Frame, Renderer};

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Part {
    Below,
    Above,
}

struct Split<'a> {
    inner: &'a mut Painter,
    marker: u64,
    part: Part,
    seen: bool,
}

impl Split<'_> {
    fn drawing(&self) -> bool {
        (self.part == Part::Above) == self.seen
    }
}

impl PaintSink for Split<'_> {
    fn push_clip(&mut self, clip: ClipRef<'_>) {
        self.inner.push_clip(clip);
    }

    fn pop_clip(&mut self) {
        self.inner.pop_clip();
    }

    fn push_group(&mut self, group: GroupRef<'_>) {
        self.inner.push_group(group);
    }

    fn pop_group(&mut self) {
        self.inner.pop_group();
    }

    fn fill(&mut self, draw: FillRef<'_>) {
        if let BrushRef::Image(image) = &draw.brush
            && image.image.data.id() == self.marker
        {
            self.seen = true;
        } else if self.drawing() {
            self.inner.fill(draw);
        }
    }

    fn stroke(&mut self, draw: StrokeRef<'_>) {
        if self.drawing() {
            self.inner.stroke(draw);
        }
    }

    fn glyph_run(&mut self, draw: GlyphRunRef<'_>, glyphs: &mut dyn Iterator<Item = Glyph>) {
        if self.drawing() {
            self.inner.glyph_run(draw, glyphs);
        }
    }

    fn blurred_rounded_rect(&mut self, draw: BlurredRoundedRect) {
        if self.drawing() {
            self.inner.blurred_rounded_rect(draw);
        }
    }
}

impl Renderer {
    pub(super) fn part(&mut self, frame: &Frame<'_>, marker: u64, (x, y, w, h): (u16, u16, u16, u16), part: Part, seconds: f64) -> Result<Vec<u32>, PaintError> {
        self.painter.begin(x, y, w, h);
        self.painter.seconds = seconds;
        frame.paint_into(&mut Split { inner: &mut self.painter, marker, part, seen: false });
        self.painter.finish()?;
        let mut pixels = vec![0u32; usize::from(w) * usize::from(h)];
        let pixmap = PixmapMut::new(w, h, bytemuck::cast_slice_mut(&mut pixels)).expect("кэш слоя");
        let settings = RasterizerSettings { pixel_format: PixelFormat::Bgra8, ..Default::default() };
        self.painter.ctx.render_with(pixmap, &mut self.painter.resources, settings);
        Ok(pixels)
    }
}
