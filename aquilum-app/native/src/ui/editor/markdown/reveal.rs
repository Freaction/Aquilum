use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reveal {
    Touch,
    Line,
    Block,
    Above,
    Below,
    Never,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Inline,
    Tag,
    Bullet,
    Task,
    Heading,
    Quote,
    Rule,
    CalloutName,
    Setext,
    Fence,
    Callout,
    Dataview,
    Book,
    Math,
    Mermaid,
    Image,
    Table,
}

impl Part {
    pub const fn reveal(self) -> Reveal {
        match self {
            Part::Inline | Part::Tag | Part::Bullet | Part::Task => Reveal::Touch,
            Part::Heading | Part::Quote | Part::Rule | Part::CalloutName => Reveal::Line,
            Part::Setext | Part::Fence | Part::Callout | Part::Dataview | Part::Book => Reveal::Block,
            Part::Math | Part::Mermaid => Reveal::Below,
            Part::Image => Reveal::Above,
            Part::Table => Reveal::Never,
        }
    }

    pub fn shown(self, selection: Option<&Range<usize>>, range: &Range<usize>) -> bool {
        self.reveal().opened(selection, range)
    }
}

impl Reveal {
    pub fn opened(self, selection: Option<&Range<usize>>, range: &Range<usize>) -> bool {
        match self {
            Reveal::Touch | Reveal::Block | Reveal::Below => touches(selection, range),
            Reveal::Line => selection.is_some(),
            Reveal::Above => selection.is_some_and(|s| !s.is_empty()) && touches(selection, range),
            Reveal::Never => false,
        }
    }
}

pub fn touches(selection: Option<&Range<usize>>, range: &Range<usize>) -> bool {
    selection.is_some_and(|s| s.start <= range.end && range.start <= s.end)
}
