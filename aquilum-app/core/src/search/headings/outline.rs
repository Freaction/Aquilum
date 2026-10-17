use super::{heading_at, scan, Heading};

#[derive(Default)]
pub(super) struct Builder {
    found: Vec<Heading>,
    open: Vec<usize>,
}

impl Builder {
    fn push(&mut self, mut heading: Heading) {
        self.close(heading.level, heading.start, heading.start_char);
        heading.content.end = heading.content.start;
        heading.chars.end = heading.chars.start;
        self.open.push(self.found.len());
        self.found.push(heading);
    }

    fn close(&mut self, level: u8, end: usize, end_char: usize) {
        while let Some(&top) = self.open.last() {
            if self.found[top].level < level {
                break;
            }
            self.found[top].content.end = end;
            self.found[top].chars.end = end_char;
            self.open.pop();
        }
    }

    pub(super) fn finish(mut self, end: usize, end_char: usize) -> Vec<Heading> {
        self.close(0, end, end_char);
        self.found
    }
}

pub(super) fn collect(text: &str, start: usize, start_char: usize, builder: &mut Builder) -> usize {
    let (mut byte, mut count) = (0, start_char);
    let mut at = |to: usize| {
        count += text[byte..to].chars().count();
        byte = to;
        count
    };
    for heading in scan(text) {
        let start_char = at(heading.start);
        let content_char = at(heading.content.start);
        builder.push(Heading {
            start: start + heading.start,
            content: start + heading.content.start..start + heading.content.start,
            start_char,
            chars: content_char..content_char,
            ..heading
        });
    }
    at(text.len())
}

pub fn outline(pages: &mut crate::search::pages::Pages<'_>) -> std::io::Result<(Vec<Heading>, usize)> {
    let (mut builder, mut bytes, mut chars) = (Builder::default(), 0, 0);
    while let Some(page) = pages.next()? {
        chars = collect(&page.text, page.start, chars, &mut builder);
        bytes = page.start + page.text.len();
    }
    Ok((builder.finish(bytes, chars), chars))
}

pub struct Spot {
    pub page: crate::search::pages::Page,
    pub at: usize,
    pub heading: Option<String>,
}

pub fn find_spot(
    pages: &mut crate::search::pages::Pages<'_>,
    cancel: &crate::text::cancel::Cancel,
    mut found: impl FnMut(&crate::search::pages::Page) -> Option<usize>,
) -> std::io::Result<Option<Spot>> {
    let mut heading = None;
    while let Some(page) = pages.next()? {
        if cancel.is_cancelled() {
            return Ok(None);
        }
        let Some(at) = found(&page) else {
            heading = heading_at(&page.text, page.text.len()).or(heading);
            continue;
        };
        let heading = heading_at(&page.text, at).or(heading);
        return Ok(Some(Spot { page, at, heading }));
    }
    Ok(None)
}
