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

pub use blocks::{Block, Callout, Frame, Item, State};
#[cfg(test)]
pub use blocks::classify;
pub use brushes::{ACCENT, DANGER, IDEA, INFO, LINK, MUTED, ON_ACCENT, SECONDARY, SUCCESS, SYNTAX, TEXT, WARNING, palette, tone};
pub use decor::{Boxed, Decor, Link};
pub use inline::bare_urls;
#[cfg(test)]
pub use inline::{Kind as SpanKind, hashtags, inline as inline_spans};
pub use mark::{Mark, enclosing, word_at};
pub use metrics::{BULLET, CHECKBOX, Fonts, LIST_LEVEL, LIST_START, QUOTE_BAR, QUOTE_INSET};
pub use paint::{line_at, line_middle, paint_backgrounds, paint_boxes, paint_bullet, paint_check, range_rects, x_at};
pub use reveal::{Part, Reveal, touches};
#[cfg(test)]
pub use paint::tag_chips;
pub use style::{Context, push, style};
