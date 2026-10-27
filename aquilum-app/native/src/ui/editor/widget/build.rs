use super::*;

impl TextEditor {
    pub fn new(text: &str) -> Self {
        TextEditor::with(Document::new(text))
    }

    pub fn windowed(text: crop::Rope) -> Self {
        TextEditor::with(Document::windowed(text))
    }

    pub fn entire(text: crop::Rope) -> Self {
        TextEditor::with(Document::entire(text))
    }

    fn with(doc: Document) -> Self {
        let mut styles = StyleSet::new(16.0);
        styles.insert(StyleProperty::LineHeight(masonry::parley::LineHeight::FontSizeRelative(1.2)));
        TextEditor {
            doc,
            history: History::default(),
            styles,
            style: None,
            anchor: 0,
            focus: 0,
            affinity: Affinity::Downstream,
            column: None,
            compose: None,
            width: 0.0,
            past_end: 0.0,
            caret_visible: true,
            follow_caret: false,
            drag_at: None,
            painted: (0.0, 0.0),
            blink_start: std::time::Instant::now(),
            single_line: false,
            wraps: false,
            centered: false,
            scroll_x: 0.0,
            focused: false,
            table: None,
            hovered_table: None,
            embed_scrolls: std::collections::HashMap::new(),
            clips: Vec::new(),
            code_bar: None,
            copied: None,
            media: media::MediaState::default(),
            videos: crate::video::Owned::default(),
            pointer_hover: false,
            hovered_link: None,
            unit: None,
            suggest: None,
            find: find::Find::default(),
            smart: Smart::default(),
            suggest_shown: None,
            hover_terms: Vec::new(),
            external_reveal: None,
            extra_cursors: Vec::new(),
            wake: None,
        }
    }

    pub fn single_line(text: &str) -> Self {
        let text = text.replace(['\r', '\n'], " ");
        TextEditor { single_line: true, ..TextEditor::new(&text) }
    }

    pub fn wrapping(mut self) -> Self {
        self.wraps = true;
        self
    }

    pub fn centered(mut self) -> Self {
        self.centered = true;
        self
    }

    pub fn markdown(mut self) -> Self {
        let size = self.styles.inner().values().find_map(|p| if let StyleProperty::FontSize(s) = p { Some(*s) } else { None }).unwrap_or(16.0);
        self.doc.set_markdown(Some(Fonts::new(size, self.smart.callouts, self.smart.tags)));
        self
    }

    pub fn past_end(mut self, past_end: f64) -> Self {
        self.past_end = past_end;
        self
    }

    pub fn report_edits(mut self) -> Self {
        self.doc.report_edits();
        self
    }

    pub fn with_wake(mut self, wake: std::sync::Arc<dyn Fn() + Send + Sync>) -> Self {
        self.wake = Some(wake);
        self
    }

    pub fn with_resolver(mut self, resolver: Option<std::sync::Arc<dyn crate::ui::editor::resolve::Resolver>>) -> Self {
        self.doc.resolver = resolver;
        self
    }

    pub fn styled(mut self, style: fn() -> Vec<StyleProperty>) -> Self {
        for property in style() {
            self.styles.insert(property);
        }
        self.style = Some(style);
        self
    }
}
