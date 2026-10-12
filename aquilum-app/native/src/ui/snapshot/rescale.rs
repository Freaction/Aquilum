use masonry::app::RenderRoot;

use super::shell_with;
use crate::render::Renderer;
use crate::ui::{rescale, theme};

#[test]
fn rescale_matches_fresh_layout() {
    let _theme = theme::test_lock();
    let vault = crate::test_dir::tempdir();
    let data = crate::test_dir::tempdir();
    for note in ["Aquilum.md", "Уход от Tauri.md"] {
        std::fs::write(vault.path().join(note), "").unwrap();
    }
    let frame = |root: &mut RenderRoot| {
        let (layers, _) = root.redraw();
        let size = root.size();
        let mut renderer = Renderer::new(0);
        let scale = 1.75 * 1.05;
        renderer.render(&layers, size.width as u16, size.height as u16, scale, theme::background()).unwrap();
        renderer.png()
    };
    let (mut live, _) = shell_with(vault.path(), data.path(), 1.75, 1.75);
    let _ = live.redraw();
    rescale(&mut live, 1.75 * 1.05);
    let live = frame(&mut live);
    let (mut fresh, _) = shell_with(vault.path(), data.path(), 1.75, 1.75 * 1.05);
    let fresh = frame(&mut fresh);
    if let Some(dir) = std::env::var_os("AQUILUM_SNAPSHOTS").map(std::path::PathBuf::from) {
        std::fs::write(dir.join("rescale-live.png"), &live).unwrap();
        std::fs::write(dir.join("rescale-fresh.png"), &fresh).unwrap();
    }
    assert!(live == fresh, "кадр после смены масштаба отличается от собранного заново");
}
