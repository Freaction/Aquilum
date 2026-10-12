mod blocks;
mod brushes;
mod decor;
mod inline;
mod mark;
mod metrics;
mod paint;
mod style;

#[cfg(test)]
mod tests;

pub use blocks::{Block, Callout, State};
#[cfg(test)]
pub use blocks::classify;
pub use brushes::{ACCENT, DANGER, LINK, MUTED, ON_ACCENT, SECONDARY, SUCCESS, SYNTAX, TEXT, palette, tone};
pub use decor::{Decor, Link};
pub use inline::bare_urls;
#[cfg(test)]
pub use inline::{Kind as SpanKind, hashtags, inline as inline_spans};
pub use mark::{Mark, enclosing};
pub use metrics::{CHECKBOX, Fonts, LIST_LEVEL, QUOTE_BAR};
pub use paint::{paint_inline, range_rects};
pub use style::{push, style};
