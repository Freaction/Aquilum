use aquilum_core::markdown::fence::{Fence, INDENT_LIMIT};
use aquilum_core::markdown::indent::Indent;

use super::super::super::highlight::Lang;
use super::list::Levels;
use super::{Block, Frame, State};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct Open {
    fence: Fence,
    saved: Levels,
    frame: Frame,
    indent: usize,
    lang: Option<Lang>,
}

impl Open {
    fn frame(&self, line: &str) -> Option<Frame> {
        let frame = self.frame.follow(line)?;
        Some(Frame { lead: frame.lead + Indent::of(&line[frame.lead..]).bytes.min(self.indent), ..frame })
    }

    fn continues(&self, next: Option<&str>) -> bool {
        next.is_some_and(|next| self.frame.follow(next).is_some())
    }
}

impl State {
    pub(super) fn fenced(&mut self, line: &str, next: Option<&str>) -> Option<(Frame, Block)> {
        let open = self.open?;
        let frame = open.frame(line);
        let close = frame.is_none_or(|frame| open.fence.closes(&line[frame.lead..]));
        if close {
            self.open = None;
            self.levels = open.saved;
        }
        frame.map(|frame| (frame, Block::Code { first: false, last: close || !open.continues(next), fence: close, lang: open.lang }))
    }

    pub(super) fn fence(&mut self, line: &str, frame: Frame, next: Option<&str>) -> Option<(Frame, Block)> {
        let rest = &line[frame.lead..];
        if self.math || self.commenting || !rest.trim_start_matches([' ', '\t']).starts_with(['`', '~']) {
            return None;
        }
        let fence = Fence::open(rest, if frame.quote > 0 || self.levels.is_empty() { INDENT_LIMIT } else { usize::MAX })?;
        let indent = Indent::of(rest).bytes;
        let open = Open { fence, saved: self.levels, frame, indent, lang: Lang::from_info(fence.info(rest)) };
        self.open = Some(open);
        Some((Frame { lead: frame.lead + indent, ..frame }, Block::Code { first: true, last: !open.continues(next), fence: true, lang: open.lang }))
    }

    pub(super) fn math_line(&mut self, rest: &str, next: Option<&str>) -> Option<Block> {
        let marker = rest.trim() == "$$";
        if self.commenting || !self.math && !marker {
            return None;
        }
        let (first, close) = (!self.math, self.math && marker);
        self.math = !close;
        Some(Block::Code { first, last: close || next.is_none(), fence: first || close, lang: Some(Lang::Math) })
    }
}
