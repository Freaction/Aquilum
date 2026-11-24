use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use aquilum_core::Core;
use masonry::app::{RenderRoot, RenderRootOptions, WindowSizePolicy};
use masonry::core::{NewWidget, WindowEvent};
use masonry::dpi::PhysicalSize;
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::Flex;

use super::super::book::Picked;
use super::super::{App, jobs};
use super::super::embeds::Embeds;
use crate::native_dialog::Destination;
use crate::ui::editor::resolve::Resolver;
use crate::ui::editor::{Smart, TextEditor};
use crate::ui::tokens::size;
use crate::ui::{style, theme};

const LOADING: Duration = Duration::from_secs(15);

pub(in crate::app) type Loading = (Embeds, Receiver<()>);

pub(in crate::app) fn loading(core: &Arc<Core>, root: &Path, note: PathBuf, data_dir: &Path) -> Loading {
    let (changed, changes) = channel();
    let wake = move || {
        let _ = changed.send(());
    };
    (Embeds::new(Arc::clone(core), root.to_path_buf(), note, data_dir.join("remote"), Arc::new(wake)), changes)
}

fn stem(note: &Path) -> String {
    note.file_stem().unwrap_or_default().to_string_lossy().into_owned()
}

impl App {
    pub(in crate::app) fn export_pdf(&mut self, _root: &mut RenderRoot) {
        let Some(note) = self.note.as_ref().map(|n| n.path.clone()) else { return };
        let name = format!("{}.pdf", stem(&note));
        self.pick(move || crate::native_dialog::save_file("PDF", &["pdf"], &name).map(|dest| Picked::Pdf(note, dest)));
    }

    pub(in crate::app) fn write_pdf(&mut self, dest: Destination) {
        let Some(note) = self.note.as_ref().map(|n| n.path.clone()) else { return };
        let (body, smart) = (self.body(), Smart::from_config(&self.workspace.core.settings.get_config()));
        let embeds = self.tree.root().map(|root| loading(&self.workspace.core, root, note.clone(), &self.workspace.data_dir));
        self.own(jobs::detach("экспорт PDF", move || {
            if let Err(error) = render(&stem(&note), body, smart, embeds).and_then(|pdf| dest.write(&pdf)) {
                report!("{dest} не экспортирован в PDF: {error}");
            }
        }));
    }
}

pub(in crate::app) fn render(title: &str, body: crop::Rope, smart: Smart, embeds: Option<Loading>) -> Result<Vec<u8>, String> {
    theme::printing(|| paper(title, body, smart, embeds))
}

fn paper(title: &str, body: crop::Rope, smart: Smart, embeds: Option<Loading>) -> Result<Vec<u8>, String> {
    let width = crate::pdf::content_width().floor();
    let resolver = embeds.as_ref().map(|(e, _)| Arc::new(e.clone()) as Arc<dyn Resolver>);
    let heading = NewWidget::new(TextEditor::single_line(title).styled(style::editor_title)).with_class(style::EDITOR_TEXT);
    let text = NewWidget::new(TextEditor::entire(body).printed().styled(style::editor_text).markdown().with_resolver(resolver).with_smart(smart)).with_class(style::EDITOR_TEXT);
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
    if let Some((embeds, changes)) = &embeds {
        let until = Instant::now() + LOADING;
        while let Some(left) = until.checked_duration_since(Instant::now()) {
            if embeds.take_changed() {
                root.edit_widget(id, |mut w| TextEditor::refresh_embeds(&mut w.downcast()));
                let _ = root.redraw();
            } else if !embeds.busy() || changes.recv_timeout(left).is_err() {
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
