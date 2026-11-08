use super::*;
use crate::ui::reader::ReaderView;

fn pdf_note(h: &Harness) -> (std::path::PathBuf, std::path::PathBuf) {
    let file = h.vault.path().join("book.pdf");
    std::fs::write(&file, crate::reader::pdf::tests::sample_pdf()).unwrap();
    let page = h.write_note("PDF.md", "---\n\u{41f}\u{443}\u{442}\u{44c} \u{43a} \u{444}\u{430}\u{439}\u{43b}\u{443}: book.pdf\npages: 0/1\n---\n");
    (file, page)
}

fn pdf_state(h: &Harness) -> Option<(u32, String, f64)> {
    let Some(Popup::Reader(s)) = &h.app.popup else { return None };
    h.root.get_widget(s.layer)?.downcast::<ReaderView>()?.inner().position()
}

pub(super) fn wait_pdf(h: &mut Harness) {
    assert!(
        h.wait_for(|h| {
            let Some(Popup::Reader(s)) = &h.app.popup else { return false };
            h.root.get_widget(s.layer).unwrap().downcast::<ReaderView>().unwrap().inner().pdf_rendered()
        }),
        "PDF did not render"
    );
    h.anim(std::time::Duration::from_secs(1));
    let _ = h.root.redraw();
    h.deliver();
}

#[test]
fn pdf_progress_survives_close_reopen_and_application_restart() {
    let mut h = Harness::new();
    let (file, page) = pdf_note(&h);
    open_book(&mut h, &file, Some(&page), "PDF");
    wait_pdf(&mut h);
    assert_eq!(pdf_state(&h).unwrap().0, 3);
    turn_pages(&mut h, 1);
    assert_eq!(pdf_state(&h).unwrap().2, 2.0 / 3.0);
    assert!(h.app.reader_zoom(&mut h.root, crate::ui::keymap::Command::ZoomIn));
    let _ = h.root.redraw();
    h.scroll((450.0, 250.0), 1.0, (-90.0, -140.0));
    h.deliver();
    let before = pdf_state(&h).unwrap();
    h.app.save_reader(&mut h.root);
    let saved = std::fs::read_to_string(&page).unwrap();
    assert!(saved.contains("pages: 2/3"), "{saved}");
    assert!(saved.contains(&before.1), "exact PDF position is missing: {saved}");
    h.escape();
    open_book(&mut h, &file, Some(&page), "PDF");
    wait_pdf(&mut h);
    assert_eq!(pdf_state(&h).unwrap(), before);
    h.escape();
    let mut h = h.restarted(1.0);
    open_book(&mut h, &file, Some(&page), "PDF");
    wait_pdf(&mut h);
    assert_eq!(pdf_state(&h).unwrap(), before);
    h.snapshot("pdf-reader.png");
}

#[test]
fn pdf_can_resume_without_a_book_note_and_zoom_does_not_scale_the_interface() {
    let mut h = Harness::new();
    let (file, _) = pdf_note(&h);
    open_book(&mut h, &file, None, "PDF");
    wait_pdf(&mut h);
    turn_pages(&mut h, 2);
    let ui_scale = h.app.scale();
    let command = crate::ui::keymap::Command::ZoomIn;
    assert!(h.app.command(&mut h.root, command).is_some());
    let before = pdf_state(&h).unwrap();
    h.escape();
    assert_eq!(h.app.scale(), ui_scale);
    open_book(&mut h, &file, None, "PDF");
    wait_pdf(&mut h);
    assert_eq!(pdf_state(&h).unwrap(), before);
}

#[test]
#[ignore = "set AQUILUM_PDF_FIXTURE to a local PDF"]
fn real_pdf_partial_redraw_matches_full_redraw_during_zoom_and_pan() {
    let mut h = Harness::new();
    let file = std::path::PathBuf::from(std::env::var_os("AQUILUM_PDF_FIXTURE").expect("PDF fixture path"));
    open_book(&mut h, &file, None, "PDF");
    wait_pdf(&mut h);
    let mut renderer = crate::render::Renderer::new(2);
    for index in 0..6 {
        let command = if index % 2 == 0 { crate::ui::keymap::Command::ZoomIn } else { crate::ui::keymap::Command::ZoomOut };
        h.app.reader_zoom(&mut h.root, command);
        h.scroll((450.0, 250.0), 1.0, (-40.0, -100.0));
        h.deliver();
        for ready in [false, true] {
            if ready {
                wait_pdf(&mut h);
            }
            let (layers, _) = h.root.redraw();
            renderer.draw_frame(&layers, W as u16, H as u16, 1.0, theme::background(), false).unwrap();
            let expected = render_full(&layers, W as u16, H as u16, 1.0);
            assert_eq!(differing_pixels(renderer.frame_pixels(), expected.frame_pixels()), 0, "PDF partial redraw differs at step {index}, ready={ready}");
            if ready {
                dump(&format!("pdf-real-ui-{index}.png"), &renderer);
            }
        }
    }
}
