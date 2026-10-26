use super::*;

impl App {
    pub fn title(&self) -> String {
        match &self.note {
            Some(note) => format!("{} — Aquilum", note.path.file_stem().unwrap_or_default().to_string_lossy()),
            None => "Aquilum".to_owned(),
        }
    }

    pub fn deadline(&self) -> Option<Instant> {
        [self.note.as_ref().and_then(Note::deadline), self.view_due, self.hold].into_iter().flatten().min()
    }

    pub fn tick(&mut self, root: &mut RenderRoot, now: Instant) {
        if self.view_due.is_some_and(|t| t <= now) {
            self.flush_view();
        }
        if let Some(note) = self.note.as_mut()
            && note.tick(&self.workspace.core, &self.text, now)
        {
            self.refresh_save_error(root);
        }
        if let Some(text) = self.note.as_mut().and_then(crate::note::Note::take_replaced) {
            self.set_text(root, text);
            self.refresh_save_error(root);
        }
    }

    pub fn scale(&self) -> f64 {
        self.interface.scale()
    }

    pub fn zoom(&mut self, up: bool) -> bool {
        let step = if up { SCALE_STEP } else { -SCALE_STEP };
        self.interface.set_scale(self.interface.scale() + step)
    }

    pub fn window_unfocused(&mut self) {
        self.commit_note();
    }

    pub fn persist(&mut self) {
        self.flush_view();
        match &mut self.note {
            Some(note) => note.flush(&self.workspace.core, &self.text),
            None => crate::disk::wait(),
        }
    }

    pub fn shutdown(&mut self) {
        self.persist();
        self.workspace.core.shutdown();
    }

    pub fn hold_due(&mut self, now: Instant) -> bool {
        self.hold.is_some_and(|until| until <= now) && self.hold.take().is_some()
    }

    pub fn holding(&mut self) -> bool {
        let loading = self.opening.is_some() || self.embeds.as_ref().is_some_and(embeds::Embeds::busy);
        if !loading || self.hold.is_some_and(|until| until <= Instant::now()) {
            self.hold = None;
        }
        self.hold.is_some()
    }

    pub fn editor_id(&self) -> Option<WidgetId> {
        self.note_ids.as_ref().map(|ids| ids.body.id())
    }

    pub fn restore(&mut self, root: &mut RenderRoot) {
        self.show_active(root);
    }

    pub fn forget_session(&mut self) {
        self.session = None;
        self.tabs = Tabs::default();
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.shutdown();
    }
}
