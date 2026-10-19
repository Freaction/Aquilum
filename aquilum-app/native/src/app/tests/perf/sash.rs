use super::*;

use crate::ui::widgets::SashAction;

fn vault_with_links(notes: usize) -> tempfile::TempDir {
    let vault = crate::test_dir::tempdir();
    std::fs::write(vault.path().join("Цель.md"), dense_body(40)).unwrap();
    for i in 0..notes {
        std::fs::write(vault.path().join(format!("Заметка со ссылкой номер {i:03}.md")), "Ссылка на [[Цель]].").unwrap();
    }
    vault
}

fn drag(b: &mut Bench, sash: masonry::core::WidgetId, step: f64) -> (std::time::Duration, std::time::Duration) {
    b.h.send(sash, SashAction::Press);
    let (mut scene, mut raster) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
    for i in 1..=30 {
        b.h.send(sash, SashAction::Drag(step * f64::from(i)));
        let (s, r) = b.timed(false);
        scene += s;
        raster += r;
    }
    b.h.send(sash, SashAction::Release);
    (scene / 30, raster / 30)
}

#[test]
#[ignore]
fn sash_drag_cost() {
    let vault = vault_with_links(60);
    let mut b = Bench::start(Some(vault.path()));
    b.h.open(vault.path().join("Цель.md"));
    wait_until(200, 20, || {
        b.h.deliver();
        b.h.app.listing.as_ref().is_some_and(|l| !l.items.is_empty())
    });
    b.h.settle();
    b.timed(true);
    aq_trace::start();
    let chrome = b.h.app.chrome.as_ref().unwrap();
    let (sidebar, panel) = (chrome.sidebar_sash, chrome.panel_parts.sash);
    let left = drag(&mut b, sidebar, 3.0);
    let right = drag(&mut b, panel, 3.0);
    eprintln!("левая панель: сцена {:?}, растр {:?}; правая панель: сцена {:?}, растр {:?}", left.0, left.1, right.0, right.1);
    if let Ok(out) = std::env::var("AQ_TRACE") {
        aq_trace::write(std::path::Path::new(&out)).unwrap();
    }
}
