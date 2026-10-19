mod blocks;
mod brushes;
mod decor;
mod inline;
mod mark;
mod metrics;
mod paint;
mod reveal;
mod style;

#[cfg(test)]
mod tests;

pub use blocks::{Block, Callout, Frame, State};
#[cfg(test)]
pub use blocks::classify;
pub use brushes::{ACCENT, DANGER, IDEA, INFO, LINK, MUTED, ON_ACCENT, SECONDARY, SUCCESS, SYNTAX, TEXT, WARNING, palette, tone};
pub use decor::{Boxed, Decor, Link};
pub use inline::bare_urls;
#[cfg(test)]
pub use inline::{Kind as SpanKind, hashtags, inline as inline_spans};
pub use mark::{Mark, enclosing, word_at};
pub use metrics::{CHECKBOX, Fonts, LIST_START, QUOTE_BAR};
pub use paint::{paint_backgrounds, paint_boxes, range_rects};
pub use reveal::{Part, Reveal, touches};
#[cfg(test)]
pub use paint::tag_chips;
pub use style::{Context, push, style};
