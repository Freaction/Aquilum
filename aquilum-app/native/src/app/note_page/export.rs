use std::sync::Arc;
use std::time::{Duration, Instant};

use masonry::app::{RenderRoot, RenderRootOptions, WindowSizePolicy};
use masonry::core::{NewWidget, WindowEvent};
use masonry::dpi::PhysicalSize;
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::Flex;

use super::super::App;
use super::super::embeds::Embeds;
use crate::ui::editor::resolve::Resolver;
use crate::ui::editor::{Smart, TextEditor};
use crate::ui::tokens::size;
use crate::ui::{style, theme};

const LOADING: Duration = Duration::from_secs(15);

impl App {
    pub(in crate::app) fn export_pdf(&mut self, _root: &mut RenderRoot) {
        let Some(note) = &self.note else { return };
        let title = note.path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        let Some(dest) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).set_file_name(format!("{title}.pdf")).save_file() else { return };
        let body = self.editor_text[self.hidden.min(self.editor_text.len())..].to_owned();
        let smart = Smart::from_config(&self.workspace.core.settings.get_config());
        let embeds = self.tree.root().map(|root| Embeds::new(Arc::clone(&self.workspace.core), root.to_path_buf(), note.path.clone(), self.workspace.data_dir.join("remote"), Arc::new(|| {})));
        std::thread::spawn(move || {
            if let Err(error) = render(&title, &body, smart, embeds).and_then(|pdf| std::fs::write(&dest, pdf).map_err(|e| e.to_string())) {
                report!("{} не экспортирован в PDF: {error}", dest.display());
            }
        });
    }
}

pub(in crate::app) fn render(title: &str, body: &str, smart: Smart, embeds: Option<Embeds>) -> Result<Vec<u8>, String> {
    let width = crate::pdf::content_width().floor();
    let resolver = embeds.clone().map(|e| Arc::new(e) as Arc<dyn Resolver>);
    let heading = NewWidget::new(TextEditor::single_line(title).styled(style::editor_title)).with_class(style::EDITOR_TEXT);
    let text = NewWidget::new(TextEditor::new(body).styled(style::editor_text).markdown().with_resolver(resolver).with_smart(smart)).with_class(style::EDITOR_TEXT);
    let id = text.id();
    let column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(heading).with_fixed_spacer(masonry::layout::Length::px(size::SPACE_16)).with_fixed(text);
    let options = RenderRootOptions {
        default_properties: Arc::new(theme::default_properties()),
        use_system_fonts: true,
        size_policy: WindowSizePolicy::User,
        size: PhysicalSize::new(width as u32, crate::pdf::page_height() as u32),
        scale_factor: 1.0,
        test_font: None,
    };
    let mut root = RenderRoot::new(NewWidget::new(column).with_class(style::EDITOR).erased(), |_| {}, options);
    theme::register_fonts(&mut root);
    let _ = root.redraw();
    if let Some(embeds) = &embeds {
        let started = Instant::now();
        while started.elapsed() < LOADING {
            if embeds.take_changed() {
                root.edit_widget(id, |mut w| TextEditor::refresh_embeds(&mut w.downcast()));
                let _ = root.redraw();
            } else if embeds.busy() {
                std::thread::sleep(Duration::from_millis(20));
            } else {
                break;
            }
        }
    }
    let frame = |root: &RenderRoot| root.get_widget(id).map(|w| w.ctx().bounding_box());
    let height = frame(&root).map_or(0.0, |b| b.y1).ceil();
    root.handle_window_event(WindowEvent::Resize(PhysicalSize::new(width as u32, height as u32 + 1)));
    let _ = root.redraw();
    root.edit_widget(id, |mut w| w.ctx.request_paint_only());
    let (layers, _) = root.redraw();
    let top = frame(&root).map_or(0.0, |b| b.y0);
    let breaks: Vec<f64> = root.get_widget(id).and_then(|w| w.downcast::<TextEditor>().map(|e| e.inner().breaks())).unwrap_or_default().into_iter().map(|b| top + b).collect();
    crate::pdf::write(&layers, &crate::pdf::pages(&breaks, height), theme::current().editor_bg)
}
