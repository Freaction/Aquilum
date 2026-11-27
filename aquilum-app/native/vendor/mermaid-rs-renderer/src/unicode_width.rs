//! Unicode width classification shared between layout-time fast metrics and
//! glyph-advance text measurement.
//!
//! Both `layout::text::fallback_text_width` and
//! `text_metrics::FontFace::measure_width` need to identify CJK ideographs,
//! emoji, and emoji-cluster glue (ZWJ, variation selectors, skin-tone
//! modifiers, regional-indicator flag pairs, keycap sequences) so they can
//! agree on per-glyph widths. This module owns the codepoint tables and the
//! cluster walker so the two paths cannot drift.
//!
//! CJK wide chars are intentionally NOT collapsed into [`consume_cluster`]:
//! `measure_width` should still prefer the loaded font's real glyph advance
//! for CJK and only fall back to 1 em when the glyph is missing, which
//! requires keeping CJK as a per-char predicate.

/// Classification of a multi-codepoint cluster recognised by
/// [`consume_cluster`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cluster {
    /// Render at one em (e.g. an emoji, flag, keycap, or ZWJ family).
    Wide,
    /// Contributes no advance on its own (a stray ZWJ / VS / skin-tone
    /// modifier that did not bind to a wide cluster).
    ZeroWidth,
}

pub(crate) fn is_cjk_wide_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{1100}'..='\u{11ff}'
            | '\u{2e80}'..='\u{a4cf}'
            | '\u{a960}'..='\u{a97f}'
            | '\u{ac00}'..='\u{d7ff}'
            | '\u{f900}'..='\u{faff}'
            | '\u{fe10}'..='\u{fe1f}'
            | '\u{fe30}'..='\u{fe4f}'
            | '\u{ff01}'..='\u{ff60}'
            | '\u{ffe0}'..='\u{ffe6}'
            | '\u{20000}'..='\u{2fa1f}'
            | '\u{30000}'..='\u{323af}'
    )
}

pub(crate) fn is_emoji_wide_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{2600}'..='\u{27bf}' | '\u{1f000}'..='\u{1faff}'
    )
}

pub(crate) fn is_emoji_modifier_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{200d}' | '\u{20e3}' | '\u{fe00}'..='\u{fe0f}' | '\u{1f3fb}'..='\u{1f3ff}'
    )
}

pub(crate) fn is_regional_indicator(ch: char) -> bool {
    matches!(ch, '\u{1f1e6}'..='\u{1f1ff}')
}

pub(crate) fn is_keycap_starter(ch: char) -> bool {
    ch.is_ascii_digit() || matches!(ch, '#' | '*')
}

/// Try to consume a multi-codepoint cluster starting at `chars[idx]` and
/// return its width category along with the index just past the cluster.
///
/// Patterns are checked in priority order: keycap sequence, regional
/// indicator pair (flag), emoji-wide cluster (with optional ZWJ joins and
/// skin-tone modifiers), then standalone modifier. Returns `None` for plain
/// characters (including CJK), letting the caller decide how to measure
/// them.
pub(crate) fn consume_cluster(chars: &[char], idx: usize) -> Option<(Cluster, usize)> {
    let ch = *chars.get(idx)?;

    // Keycap: `[0-9#*]` + optional VS16 + U+20E3.
    if is_keycap_starter(ch) {
        let next = idx + 1;
        let keycap = if chars.get(next) == Some(&'\u{fe0f}') {
            next + 1
        } else {
            next
        };
        if chars.get(keycap) == Some(&'\u{20e3}') {
            return Some((Cluster::Wide, keycap + 1));
        }
    }

    // Regional indicator pair → flag.
    if is_regional_indicator(ch)
        && chars
            .get(idx + 1)
            .copied()
            .is_some_and(is_regional_indicator)
    {
        return Some((Cluster::Wide, idx + 2));
    }

    // Emoji wide base, optionally extended by ZWJ-joined emoji and/or
    // trailing modifiers.
    if is_emoji_wide_char(ch) {
        let mut end = idx + 1;
        while end < chars.len() {
            if chars[end] == '\u{200d}'
                && chars.get(end + 1).copied().is_some_and(is_emoji_wide_char)
            {
                end += 2;
            } else if is_emoji_modifier_char(chars[end]) {
                end += 1;
            } else {
                break;
            }
        }
        return Some((Cluster::Wide, end));
    }

    // Standalone modifier that did not attach to a wide base above.
    if is_emoji_modifier_char(ch) {
        return Some((Cluster::ZeroWidth, idx + 1));
    }

    None
}
