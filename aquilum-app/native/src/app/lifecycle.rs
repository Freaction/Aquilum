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
        if let Some(note) = self.note.as_mut().filter(|n| n.deadline().is_some_and(|t| t <= now)) {
            let had = note.save_error.is_some();
            if note.save(&self.workspace.core, &self.editor_text).is_err() || had {
                self.refresh_save_error(root);
            }
        }
        if let Some(text) = self.note.as_mut().and_then(crate::note::Note::take_replaced) {
            self.set_editor_text(root, text);
            self.refresh_save_error(root);
        }
    }

    pub fn scale(&self) -> f64 {
        self.interface.scale()
    }

    pub fn zoom_key(&mut self, key: &str) -> bool {
        let scale = self.interface.scale();
        match key {
            "=" | "+" => self.interface.set_scale(scale + SCALE_STEP),
            "-" => self.interface.set_scale(scale - SCALE_STEP),
            "0" => self.interface.set_scale(1.0),
            _ => false,
        }
    }

    pub fn zoom_wheel(&mut self, up: bool) -> bool {
        let step = if up { SCALE_STEP } else { -SCALE_STEP };
        self.interface.set_scale(self.interface.scale() + step)
    }

    pub fn shutdown(&mut self) {
        self.flush_view();
        if let Some(note) = &mut self.note {
            let _ = note.save(&self.workspace.core, &self.editor_text);
        }
        self.workspace.core.shutdown();
    }

    pub fn hold_due(&mut self, now: Instant) -> bool {
        self.hold.is_some_and(|until| until <= now) && self.hold.take().is_some()
    }

    pub fn holding(&mut self) -> bool {
        let loading = self.embeds.as_ref().is_some_and(embeds::Embeds::busy);
        if !loading || self.hold.is_some_and(|until| until <= Instant::now()) {
            self.hold = None;
        }
        self.hold.is_some()
    }

    pub fn editor_id(&self) -> Option<WidgetId> {
        self.note_ids.as_ref().map(|ids| ids.body.id())
    }

    pub fn restore(&mut self, root: &mut RenderRoot) -> bool {
        self.show_active(root);
        self.note.is_some()
    }

    pub fn forget_session(&mut self) {
        self.session = None;
        self.tabs = Tabs::default();
    }
}
