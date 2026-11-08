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

impl Worker {
    pub fn new(path: PathBuf, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        let shared = Arc::new(Shared::default());
        let worker = Arc::clone(&shared);
        std::thread::spawn(move || {
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
        // A completed obsolete frame must not keep its buffer alive.
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
    let mut ctx = RenderContext::new_with(1, 1, crate::render::render_settings(0));
    let mut resources = Resources::new();
    let mut surfaces = crate::render::Surfaces::default();
    let mut cache = hayro::RenderCache::new();
    let mut cache_key = None;
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
            ctx = RenderContext::new_with(1, 1, crate::render::render_settings(0));
            resources = Resources::new();
            surfaces = crate::render::Surfaces::default();
            cache = hayro::RenderCache::new();
            cache_key = None;
            crate::system::release_memory();
            continue;
        };
        let Some(page) = pdf.pages().get(view.page) else { continue };
        // Keep font/image caches for the current page and raster scale only.
        let key = (view.page, view.scale * view.dpi);
        if cache_key != Some(key) {
            cache = hayro::RenderCache::new();
            cache_key = Some(key);
        }
        let (width, height) = view.pixels();
        ctx.reset_and_resize(width, height);
        let transform = Affine::scale(view.dpi) * view.transform() * page.initial_transform(true).to_kurbo();
        hayro::render_into(page, &cache, &InterpreterSettings::default(), &hayro::RenderSettings::default(), &mut ctx, transform);
        ctx.flush();
        {
            let state = shared.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.closed {
                return Ok(());
            }
            if state.revision != revision {
                ctx.reset();
                continue;
            }
        }
        let image = surfaces.owned_frame(width, height, |target| {
            let settings = RasterizerSettings { target_init: TargetInit::Clear(vello_cpu::color::palette::css::WHITE), ..Default::default() };
            ctx.render_with(target, &mut resources, settings);
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
