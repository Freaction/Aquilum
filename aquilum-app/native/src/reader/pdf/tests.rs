use super::*;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub(crate) fn sample_pdf() -> Vec<u8> {
    let content = b"1 0 0 rg 0 600 600 300 re f 0 0 1 rg 0 0 600 300 re f 0 0 0 rg BT /F1 24 Tf 40 450 Td (Aquilum PDF - viewport test) Tj ET";
    let mut objects = vec!["<< /Type /Catalog /Pages 2 0 R >>".into(), "<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 >>".into()];
    for rotation in [0, 90, 0] {
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 900] /Rotate {rotation} /Resources << /Font << /F1 7 0 R >> >> /Contents 6 0 R >>"));
    }
    objects.push(format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), std::str::from_utf8(content).unwrap()));
    objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into());
    let mut data = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(data.len());
        data.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref = data.len();
    data.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
    for offset in &offsets[1..] {
        data.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    data.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", offsets.len()).as_bytes());
    data
}

fn opened(target: Target) -> (tempfile::TempDir, Session, mpsc::Receiver<()>) {
    let dir = crate::test_dir::tempdir();
    let path = dir.path().join("book.pdf");
    std::fs::write(&path, sample_pdf()).unwrap();
    let (tx, rx) = mpsc::channel();
    let mut session = Session::open(
        &path,
        target,
        Arc::new(move || {
            let _ = tx.send(());
        }),
    );
    session.ensure(Size::new(240.0, 180.0), 2.0);
    wait(&mut session, &rx, |s| s.total() == 3 && s.image().is_some());
    (dir, session, rx)
}

fn wait(s: &mut Session, rx: &mpsc::Receiver<()>, ready: impl Fn(&Session) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        s.poll();
        assert!(s.error.is_none(), "{:?}", s.error);
        if ready(s) {
            return;
        }
        assert!(Instant::now() < deadline, "PDF worker timed out");
        let _ = rx.recv_timeout(Duration::from_millis(50));
    }
}

#[test]
fn renders_only_the_viewport_and_keeps_the_last_frame_during_zoom() {
    let (_dir, mut s, rx) = opened(Target::Start);
    let frame = s.frame.as_ref().unwrap();
    assert_eq!((frame.image.image.width, frame.image.image.height), (480, 360));
    assert_eq!(&frame.image.image.data.data()[4 * (20 * 480 + 20)..][..4], &[255, 0, 0, 255]);
    assert_eq!(&frame.image.image.data.data()[4 * (300 * 480 + 20)..][..4], &[255, 255, 255, 255], "unpainted PDF paper must be opaque white");
    let anchor = Point::new(80.0, 70.0);
    let before = s.view().transform().inverse() * anchor;
    assert!(s.zoom(2.0, anchor));
    let after = s.view().transform().inverse() * anchor;
    assert!((before - after).hypot() < 1e-8, "pinch anchor moved");
    assert!(s.image().is_some(), "zoom discarded the visible frame");
    for _ in 0..20 {
        s.zoom(1.2, anchor);
    }
    wait(&mut s, &rx, |s| s.frame.as_ref().is_some_and(|f| Some(f.view) == s.requested));
    let image = &s.frame.as_ref().unwrap().image;
    assert_eq!((image.image.width, image.image.height), (480, 360), "zoom allocated a whole page");
    s.ensure(Size::new(20_000.0, 10_000.0), 4.0);
    let (w, h) = s.view().pixels();
    assert!(usize::from(w) * usize::from(h) <= 4_200_000);
}

#[test]
fn restores_page_zoom_and_offset_and_handles_rotated_pages() {
    let (_dir, mut s, _) = opened(Target::Start);
    s.turn(true);
    assert_eq!(s.sizes[1], Size::new(900.0, 600.0));
    s.zoom(4.0, Point::new(120.0, 90.0));
    s.pan(Vec2::new(-70.0, -45.0));
    let (position, fraction) = s.location().unwrap();
    assert_eq!(fraction, 2.0 / 3.0);
    let (_dir2, restored, _) = opened(Target::Cfi(position.clone()));
    assert_eq!(restored.location().unwrap().0, position);
    assert_eq!(restored.view(), s.view());
}

#[test]
fn reports_invalid_files_without_a_progress_position() {
    let dir = crate::test_dir::tempdir();
    let file = dir.path().join("invalid.pdf");
    std::fs::write(&file, b"not a PDF").unwrap();
    let (tx, rx) = mpsc::channel();
    let mut s = Session::open(
        &file,
        Target::Start,
        Arc::new(move || {
            let _ = tx.send(());
        }),
    );
    rx.recv_timeout(Duration::from_secs(10)).unwrap();
    s.poll();
    assert!(s.error.is_some());
    assert!(s.location().is_none());
}

#[test]
fn trim_releases_the_frame_and_does_not_lose_the_reading_position() {
    let (_dir, mut s, rx) = opened(Target::Start);
    s.turn(true);
    s.zoom(2.0, Point::new(120.0, 90.0));
    s.pan(Vec2::new(-30.0, -40.0));
    wait(&mut s, &rx, |s| s.frame.as_ref().is_some_and(|f| Some(f.view) == s.requested));
    let position = s.location().unwrap();
    let pixels = s.frame.as_ref().unwrap().image.image.data.data().to_vec();
    s.trim();
    assert!(s.image().is_none());
    s.ensure(Size::new(240.0, 180.0), 2.0);
    wait(&mut s, &rx, |s| s.image().is_some());
    assert_eq!(s.location().unwrap(), position);
    assert_eq!(s.frame.as_ref().unwrap().image.image.data.data(), pixels);
}

#[test]
#[ignore = "set AQUILUM_PDF_FIXTURE to a local PDF"]
fn real_pdf_viewports_have_opaque_paper_and_match_fresh_buffers() {
    let path = std::path::PathBuf::from(std::env::var_os("AQUILUM_PDF_FIXTURE").expect("PDF fixture path"));
    let (tx, rx) = mpsc::channel();
    let mut s = Session::open(
        &path,
        Target::Start,
        Arc::new(move || {
            let _ = tx.send(());
        }),
    );
    s.ensure(Size::new(800.0, 600.0), 1.0);
    wait(&mut s, &rx, |s| s.image().is_some());
    println!("pages: {}", s.total());
    for index in 0..8 {
        if index == 1 {
            s.turn(true);
        }
        s.zoom(if index % 2 == 0 { 1.4 } else { 1.0 / 1.2 }, Point::new(400.0, 300.0));
        s.pan(Vec2::new(-80.0, -150.0));
        wait(&mut s, &rx, |s| s.frame.as_ref().is_some_and(|f| Some(f.view) == s.requested));
        let frame = s.frame.as_ref().unwrap();
        let pixels = frame.image.image.data.data().to_vec();
        assert!(pixels.chunks_exact(4).all(|p| p[3] == 255), "PDF paper became transparent");
        if let Some(out) = std::env::var_os("AQ_OUT") {
            image::save_buffer(
                std::path::Path::new(&out).join(format!("pdf-real-{index}.png")),
                &pixels,
                frame.image.image.width,
                frame.image.image.height,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
        s.trim();
        s.ensure(Size::new(800.0, 600.0), 1.0);
        wait(&mut s, &rx, |s| s.image().is_some());
        assert_eq!(s.frame.as_ref().unwrap().image.image.data.data(), pixels, "reused pixels differ from a fresh render at step {index}");
    }
}
