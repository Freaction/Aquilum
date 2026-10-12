mod gallery;
mod rebuild;
mod rescale;

use std::path::Path;
use std::sync::Arc;

use masonry::app::{RenderRoot, RenderRootOptions, WindowSizePolicy};
use masonry::core::{NewWidget, Widget};
use masonry::dpi::PhysicalSize;

use super::file_tree::FileTree;
use super::titlebar::TabButton;
use super::{Chrome, RootParts, note, root, theme};
use crate::render::Renderer;

const W: u32 = 1000;
const H: u32 = 600;

fn parts(text: &str) -> RootParts<'_> {
    let tabs = vec![
        TabButton::new(uuid::Uuid::new_v4(), "Заметка", true, true, None).erased(),
        TabButton::new(uuid::Uuid::new_v4(), "Уход от Tauri", false, true, None).erased(),
    ];
    RootParts {
        workspace_name: "knowledge base",
        note: note::page(note::NoteView {
            file_name: "Заметка.md",
            title: "Заметка",
            body: text,
            metadata: Some(false),
            can_back: true,
            can_forward: false,
            focus_mode: false,
            hero: None,
            resolver: None,
            smart: Default::default(),
        })
        .0
        .erased(),
        sidebar_open: true,
        sidebar_from: true,
        tabs,
        graph: None,
        graph_active: false,
        empty_tab: false,
        vault_open: true,
        vault_error: None,
        focus_mode: false,
        home: true,
        panel_open: true,
        panel_from: true,
        panel_mode: crate::links::Mode::Backlinks,
        has_analysis: true,
    }
}

fn options(size_scale: f64, scale_factor: f64) -> RenderRootOptions {
    RenderRootOptions {
        default_properties: Arc::new(theme::default_properties()),
        use_system_fonts: true,
        size_policy: WindowSizePolicy::User,
        size: PhysicalSize::new((f64::from(W) * size_scale) as u32, (f64::from(H) * size_scale) as u32),
        scale_factor,
        test_font: None,
    }
}

fn open(widget: NewWidget<dyn Widget>, size_scale: f64, scale_factor: f64) -> RenderRoot {
    let mut root = RenderRoot::new(widget, |_| {}, options(size_scale, scale_factor));
    theme::register_fonts(&mut root);
    root
}

fn open_parts(vault: Option<&Path>, data: &Path, parts: RootParts<'_>) -> (RenderRoot, Chrome) {
    let mut tree = FileTree::new(vault.map(Path::to_path_buf), data);
    let (widget, chrome) = root(tree.widget(), parts);
    (open(widget.erased(), 1.0, 1.0), chrome)
}

fn shell(vault: &Path, data: &Path) -> (RenderRoot, FileTree) {
    shell_with(vault, data, 1.0, 1.0)
}

fn shell_with(vault: &Path, data: &Path, size_scale: f64, scale_factor: f64) -> (RenderRoot, FileTree) {
    let mut tree = FileTree::new(Some(vault.to_path_buf()), data);
    let (widget, _) = root(tree.widget(), parts("# Заметка\n\nТекст заметки."));
    (open(widget.erased(), size_scale, scale_factor), tree)
}

fn save(root: &mut RenderRoot, name: &str) {
    let (layers, _) = root.redraw();
    let mut renderer = Renderer::new(0);
    let size = root.size();
    let scale = f64::from(size.width) / f64::from(W);
    renderer.render(&layers, size.width as u16, size.height as u16, scale, theme::background()).expect("кадр");
    let dir = std::env::var_os("AQUILUM_SNAPSHOTS").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let path = dir.join(format!("{name}.png"));
    std::fs::write(&path, renderer.png()).expect("снимок записан");
    eprintln!("снимок: {}", path.display());
}
