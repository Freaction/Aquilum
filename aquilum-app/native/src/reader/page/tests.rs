use masonry::parley::{FontContext, LayoutContext};

use super::{Item, dash_width, layout_blocks, text_layout};
use crate::reader::tests::{book_file, fb2, settings};
use crate::reader::{Book, flow};

#[test]
fn a_chapter_laid_out_in_parallel_matches_one_laid_out_block_by_block() {
    let (_dir, file) = book_file(fb2("Государственное благоустройство достопримечательностей неудовлетворительно характеризуется высокопоставленными представителями администрации."));
    let book = Book::open(&file).unwrap();
    let blocks = flow::blocks(book.document(0).unwrap());
    let settings = settings(18.0, false);
    let (mut fcx, mut lcx) = (FontContext::new(), LayoutContext::new());
    let items = layout_blocks(&blocks, &mut |_| None, &mut fcx, &mut lcx, &settings, 300.0, 800.0);
    let dash = dash_width(&mut fcx, &mut lcx, &settings);
    assert_eq!(items.len(), blocks.len());
    let mut hyphenated = 0;
    for (block, item) in blocks.iter().zip(&items) {
        let Item::Text { lines, breaks, x, .. } = item else { continue };
        let (layout, expected, indent) = text_layout(block, &mut fcx, &mut lcx, &settings, 300.0, dash).unwrap();
        let want: Vec<_> = layout.lines().map(|l| (l.text_range(), f64::from(l.metrics().block_min_coord), f64::from(l.metrics().block_max_coord))).collect();
        let got: Vec<_> = lines.iter().map(|l| (l.start..l.end, l.top, l.bottom)).collect();
        assert_eq!((got, breaks, x), (want, &expected, &indent));
        hyphenated += usize::from(breaks.iter().any(|b| b.1 == '-'));
    }
    assert!(hyphenated > 10, "в примере почти нет переносов в конце строки: {hyphenated}");
}
