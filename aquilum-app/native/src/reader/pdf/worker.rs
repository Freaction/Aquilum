use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};

use hayro::hayro_interpret::{InterpreterSettings, util::TransformExt};
use hayro::hayro_syntax::Pdf;
use masonry::kurbo::{Affine, Size};
use masonry::peniko::ImageBrush;
use vello_cpu::{RasterizerSettings, RenderContext, Resources, TargetInit};

use super::View;

pub struct Frame {
    pub view: View,
    pub image: ImageBrush,
}

#[derive(Default)]
pub struct Inbox {
    pub opened: Option<Vec<Size>>,
    pub frame: Option<Frame>,
    pub error: Option<String>,
}

#[derive(Default)]
struct State {
    request: Option<View>,
    inbox: Inbox,
    closed: bool,
    revision: u64,
    trim: bool,
}

#[derive(Default)]
struct Shared {
    state: Mutex<State>,
    ready: Condvar,
}

pub struct Worker(Arc<Shared>);

struct Canvas<'a> {
    ctx: RenderContext,
    resources: Resources,
    surfaces: crate::render::Surfaces,
    cache: hayro::RenderCache<'a>,
    key: Option<(usize, f64)>,
}

impl Canvas<'_> {
    fn new() -> Self {
        let ctx = RenderContext::new_with(1, 1, crate::render::render_settings(0));
        Canvas { ctx, resources: Resources::new(), surfaces: crate::render::Surfaces::default(), cache: hayro::RenderCache::new(), key: None }
    }
}

impl Worker {
    pub fn new(path: PathBuf, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        let shared = Arc::new(Shared::default());
        let worker = Arc::clone(&shared);
        let _ = aquilum_core::threads::spawn("страница PDF", move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(path, &worker, &wake)));
            if let Err(error) = outcome.unwrap_or_else(|_| Err("PDF renderer panicked".into())) {
                let mut state = worker.state.lock().unwrap_or_else(|e| e.into_inner());
                state.inbox.error = Some(error);
                drop(state);
                wake();
            }
        });
        Worker(shared)
    }

    pub fn request(&self, view: View) {
        let mut state = self.0.state.lock().unwrap_or_else(|e| e.into_inner());
        state.revision += 1;
        state.request = Some(view);
        state.inbox.frame = None;
        self.0.ready.notify_one();
    }

    pub fn take(&self) -> Inbox {
        std::mem::take(&mut self.0.state.lock().unwrap_or_else(|e| e.into_inner()).inbox)
    }

    pub fn trim(&self) {
        let mut state = self.0.state.lock().unwrap_or_else(|e| e.into_inner());
        state.revision += 1;
        state.trim = true;
        state.request = None;
        state.inbox.frame = None;
        self.0.ready.notify_one();
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let mut state = self.0.state.lock().unwrap_or_else(|e| e.into_inner());
        state.closed = true;
        state.request = None;
        state.inbox = Inbox::default();
        self.0.ready.notify_one();
    }
}

fn run(path: PathBuf, shared: &Shared, wake: &Arc<dyn Fn() + Send + Sync>) -> Result<(), String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let pdf = Pdf::new(data).map_err(|e| format!("{e:?}"))?;
    let sizes: Vec<_> = pdf
        .pages()
        .iter()
        .map(|p| {
            let (w, h) = p.render_dimensions();
            Size::new(f64::from(w), f64::from(h))
        })
        .collect();
    if sizes.is_empty() || sizes.iter().any(|s| !s.width.is_finite() || !s.height.is_finite() || s.width <= 0.0 || s.height <= 0.0) {
        return Err("PDF contains no readable pages".into());
    }
    shared.state.lock().unwrap_or_else(|e| e.into_inner()).inbox.opened = Some(sizes);
    wake();
    let mut canvas = Canvas::new();
    loop {
        let (view, revision) = {
            let mut state = shared.state.lock().unwrap_or_else(|e| e.into_inner());
            while state.request.is_none() && !state.closed && !state.trim {
                state = shared.ready.wait(state).unwrap_or_else(|e| e.into_inner());
            }
            if state.closed {
                return Ok(());
            }
            if std::mem::take(&mut state.trim) { (None, state.revision) } else { (state.request.take(), state.revision) }
        };
        let Some(view) = view else {
            canvas = Canvas::new();
            crate::system::release_memory();
            continue;
        };
        let Some(page) = pdf.pages().get(view.page) else { continue };
        let key = (view.page, view.scale * view.dpi);
        if canvas.key != Some(key) {
            canvas.cache = hayro::RenderCache::new();
            canvas.key = Some(key);
        }
        let (width, height) = view.pixels();
        canvas.ctx.reset_and_resize(width, height);
        let transform = Affine::scale(view.dpi) * view.transform() * page.initial_transform(true).to_kurbo();
        hayro::render_into(page, &canvas.cache, &InterpreterSettings::default(), &hayro::RenderSettings::default(), &mut canvas.ctx, transform);
        canvas.ctx.flush();
        {
            let state = shared.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.closed {
                return Ok(());
            }
            if state.revision != revision {
                canvas.ctx.reset();
                continue;
            }
        }
        let Canvas { ctx, resources, surfaces, .. } = &mut canvas;
        let image = surfaces.owned_frame(width, height, |target| {
            let settings = RasterizerSettings { target_init: TargetInit::Clear(vello_cpu::color::palette::css::WHITE), ..Default::default() };
            ctx.render_with(target, resources, settings);
        });
        ctx.reset();
        let mut state = shared.state.lock().unwrap_or_else(|e| e.into_inner());
        if !state.closed && state.revision == revision {
            state.inbox.frame = Some(Frame { view, image });
            drop(state);
            wake();
        }
    }
}
