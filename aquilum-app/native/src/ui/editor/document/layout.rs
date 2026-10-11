use super::*;

impl Document {
    pub fn layout(&mut self, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, width: Option<f32>, compose: Option<(usize, &str)>, caret: Option<usize>) {
        if let Some(fonts) = self.markdown.as_mut()
            && fonts.ch == 0.0
        {
            fonts.ch = text::measure_ch(fcx, lcx, fonts);
        }
        let rebreak = self.width != width;
        self.width = width;
        self.rebroke |= rebreak;
        if let Some((unresolved, stamp)) = self.resolver.as_ref().map(|r| r.unresolved())
            && stamp != self.unresolved.1
        {
            for p in self.paragraphs.iter_mut().filter(|p| p.decor().links.iter().any(|l| l.wiki)) {
                p.forget();
            }
            self.mark(0..self.paragraphs.len());
            self.unresolved = (unresolved, stamp);
        }
        let mut wanted = Vec::new();
        let mut rebreak = rebreak;
        let pinned = self.pinned();
        let mut dirty = self.dirty.take();
        loop {
            let window = self.window();
            let held = pinned.iter().chain(&self.held).map(|&i| i..i + 1);
            let mut spans: Vec<Range<usize>> = held.chain([window, self.laid.clone()]).chain(dirty.take()).collect();
            if rebreak {
                spans = vec![0..self.paragraphs.len()];
            }
            spans.sort_by_key(|r| r.start);
            let changed = self.measure(fcx, lcx, styles, width, compose, caret, rebreak, &spans, &pinned, &mut wanted) || rebreak;
            rebreak = false;
            let (from, until) = (spans.iter().map(|r| r.start).min().unwrap_or(0), spans.iter().map(|r| r.end).max().unwrap_or(0));
            self.place(from, until, fcx, lcx, styles, width, caret);
            if !changed {
                break;
            }
        }
        if let Some((at, top)) = self.anchor {
            let now = self.paragraphs[self.at_offset(at)].top;
            if (now - top).abs() > 1.0 {
                self.shift += now - top;
            }
            self.anchor = Some((at, now));
        }
        self.evict(&pinned);
        if let Some(resolver) = &self.resolver {
            resolver.sweep();
            if !wanted.is_empty() {
                resolver.resolve(wanted);
            }
        }
    }

    fn window(&self) -> Range<usize> {
        self.at_y(self.keep.0)..self.at_y(self.keep.1) + 1
    }

    #[allow(clippy::too_many_arguments)]
    fn measure(&mut self, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, width: Option<f32>, compose: Option<(usize, &str)>, caret: Option<usize>, rebreak: bool, spans: &[Range<usize>], pinned: &[usize], wanted: &mut Vec<String>) -> bool {
        let keep = self.keep;
        let ch = self.markdown.as_ref().map_or(0.0, |fonts| f64::from(fonts.ch));
        let mut line = if self.line > 0.0 { self.line } else { self.markdown.as_ref().map_or(20.0, |fonts| f64::from(fonts.size) * 1.6) };
        let mut changed = false;
        let (len, mut next) = (self.paragraphs.len(), 0);
        let indices = spans.iter().flat_map(|r| r.clone()).filter(|&i| {
            let fresh = i >= next && i < len;
            next = next.max(i + 1);
            fresh
        });
        for i in indices {
            let p = &mut self.paragraphs[i];
            let near = p.measured.is_some() && p.near(keep) || pinned.contains(&i);
            let composing = compose.filter(|(at, _)| p.range().contains(at) || *at == p.end());
            let local = self.markdown.as_ref().and_then(|_| p.caret(caret));
            let decor = p.decor.as_deref().unwrap_or(&PLAIN);
            let moved = self.markdown.is_some() && local != decor.caret;
            let wrap = width.map(|w| (w - decor.inset as f32).max(0.0));
            let pad = decor.pad.0 + decor.pad.1;
            match (&mut p.layout, composing) {
                (Some(layout), None) if rebreak && !moved => {
                    layout.break_all_lines(wrap);
                    layout.align(wrap, Alignment::Start, AlignmentOptions::default());
                    p.measured = Some((f64::from(layout.height()) + pad) as f32);
                }
                (Some(_), None) if !moved => {}
                (None, None) if !moved && p.exact && !rebreak => {}
                (None, None) if !moved && !near => {
                    if p.measured.is_none() {
                        changed = true;
                        p.estimate(&self.text, line, ch, wrap);
                    } else if rebreak {
                        p.exact = false;
                    }
                }
                (_, composing) => {
                    changed |= !p.exact;
                    p.build(&self.text, fcx, lcx, styles, width, composing, self.markdown.as_ref().map(|fonts| (fonts, local, &*self.unresolved.0)));
                    if p.block == Block::Text && p.layout().len() == 1 {
                        line = p.text_height();
                    }
                    wanted.extend(p.decor().links.iter().filter(|l| l.wiki).map(|l| l.target.trim().to_owned()));
                    if !near {
                        p.unbuild();
                    }
                }
            }
        }
        self.line = line;
        changed
    }

    #[allow(clippy::too_many_arguments)]
    fn place(&mut self, from: usize, until: usize, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, width: Option<f32>, caret: Option<usize>) {
        let mut i = if self.placed { from.min(self.paragraphs.len().saturating_sub(1)) } else { 0 };
        while i > 0 && self.paragraphs[i].fold == Fold::Member {
            i -= 1;
        }
        let mut top = i.checked_sub(1).map_or(0.0, |k| self.paragraphs[k].top + self.paragraphs[k].height);
        let keep = self.keep;
        self.placed = true;
        while i < self.paragraphs.len() {
            if i >= until && self.paragraphs[i].top == top {
                break;
            }
            if let Some((end, reveal)) = self.run_end(i) {
                let span = self.paragraphs[i].start..=self.paragraphs[end].end();
                let inside = caret.is_some_and(|c| span.contains(&c));
                let known = self.paragraphs[i].embedded.map(f64::from);
                let guess = known.unwrap_or_else(|| self.paragraphs[i..=end].iter().map(Paragraph::text_height).sum());
                let far = !inside && self.source != Some(*span.start()) && (top + guess < keep.0 || top > keep.1);
                let built = if far {
                    self.paragraphs[i].embed = None;
                    Some(guess)
                } else {
                    let built = (!(inside && reveal == embeds::Reveal::Source)).then(|| self.embed(i, end, fcx, lcx, styles, width)).flatten();
                    self.paragraphs[i].embedded = built.map(|h| h as f32);
                    built
                };
                if let Some(height) = built {
                    if reveal == embeds::Reveal::Above && self.source == Some(self.paragraphs[i].start) {
                        let p = &mut self.paragraphs[i];
                        p.fold = Fold::Above;
                        p.lift = height as f32;
                        p.top = top;
                        p.height = height + p.text_height();
                        top += p.height;
                        i += 1;
                        continue;
                    }
                    for (k, p) in self.paragraphs[i..=end].iter_mut().enumerate() {
                        p.fold = if k == 0 { Fold::Head } else { Fold::Member };
                        p.lift = 0.0;
                        p.top = if k == 0 { top } else { top + height };
                        p.height = if k == 0 { height } else { 0.0 };
                    }
                    top += height;
                    i = end + 1;
                    continue;
                }
            }
            let p = &mut self.paragraphs[i];
            p.fold = Fold::None;
            p.lift = 0.0;
            p.top = top;
            p.height = p.text_height();
            top += p.height;
            i += 1;
        }
    }

    pub fn invalidate(&mut self, offset: usize) {
        let i = self.at_offset(offset);
        self.paragraphs[i].forget();
        self.mark(i..i + 1);
    }

    pub fn invalidate_all(&mut self) {
        for p in &mut self.paragraphs {
            p.forget();
        }
        self.mark(0..self.paragraphs.len());
    }

    pub fn fonts_changed(&mut self) {
        if let Some(fonts) = self.markdown.as_mut() {
            fonts.ch = 0.0;
        }
        for p in &mut self.paragraphs {
            p.forget();
            p.embed = None;
        }
        self.mark(0..self.paragraphs.len());
    }

    pub fn measured(&self) -> bool {
        self.placed && self.dirty.is_none()
    }

    fn pinned(&self) -> Vec<usize> {
        self.pins.iter().map(|&offset| self.at_offset(offset)).collect()
    }

    fn evict(&mut self, pinned: &[usize]) {
        let keep = self.keep;
        let window = self.window();
        let old = std::mem::replace(&mut self.laid, window);
        let held = std::mem::replace(&mut self.held, pinned.to_vec());
        for i in old.chain(held) {
            if let Some(p) = self.paragraphs.get_mut(i)
                && !p.near(keep)
                && !pinned.contains(&i)
            {
                p.unbuild();
            }
        }
    }

    pub fn ensure(&mut self, shown: std::ops::RangeInclusive<usize>, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, caret: Option<usize>) {
        let pinned = self.pinned();
        let old = std::mem::replace(&mut self.shown, shown.clone());
        for i in old.filter(|i| !shown.contains(i) && !pinned.contains(i)) {
            if let Some(p) = self.paragraphs.get_mut(i) {
                p.unbuild();
            }
        }
        let width = self.width;
        for p in &mut self.paragraphs[shown] {
            if p.layout.is_none() && p.measured.is_some() {
                let local = self.markdown.as_ref().and_then(|_| p.caret(caret));
                p.build(&self.text, fcx, lcx, styles, width, None, self.markdown.as_ref().map(|fonts| (fonts, local, &*self.unresolved.0)));
            }
        }
    }
}
