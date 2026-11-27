use std::collections::HashMap;

use chrono::{Datelike, NaiveDate};
use masonry::kurbo::{Line, Point, Rect, RoundedRect};
use mermaid_rs_renderer::{GanttStatus, Graph};

use super::super::super::markdown::{MUTED, SECONDARY, TEXT};
use super::super::panel::Tone as Hue;
use super::super::tone::Tone;
use super::canvas::{BOLD, Canvas, FONT, LINE, Label, Pen, width};
use super::shape::polygon;
use crate::ui::tokens::size;

const ROW: f64 = 40.0;
const BAR: f64 = 0.5;
const CHART: f64 = FONT * 26.0;
const TICKS: usize = 4;
const DAYS: f64 = 3.0;

fn look(status: Option<GanttStatus>) -> (Tone, Option<Tone>) {
    match status {
        Some(GanttStatus::Done) => (Tone::Panel(Hue::Success), Some(Tone::Ink(Hue::Success))),
        Some(GanttStatus::Active) => (Tone::Accent, None),
        Some(GanttStatus::Crit) => (Tone::Panel(Hue::Danger), Some(Tone::Ink(Hue::Danger))),
        Some(GanttStatus::Milestone) => (Tone::Ink(Hue::Warning), None),
        None => (Tone::Panel(Hue::Accent), Some(Tone::Ink(Hue::Accent))),
    }
}

fn day(value: &str) -> Option<f64> {
    NaiveDate::parse_from_str(&value.trim().replace(['/', '.'], "-"), "%Y-%m-%d").ok().map(|d| f64::from(d.num_days_from_ce()))
}

fn date(day: f64) -> String {
    NaiveDate::from_num_days_from_ce_opt(day.round() as i32).map_or_else(String::new, |d| d.format("%Y-%m-%d").to_string())
}

fn length(value: &str) -> Option<f64> {
    let digits: String = value.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
    let scale = match value.chars().rev().find(|c| c.is_alphabetic()).map(|c| c.to_ascii_lowercase()) {
        Some('w') => 7.0,
        Some('h') => 1.0 / 24.0,
        Some('m') => 30.0,
        Some('y') => 365.0,
        _ => 1.0,
    };
    Some(digits.parse::<f64>().ok()? * scale)
}

pub(super) fn draw(graph: &Graph, canvas: &mut Canvas) {
    let tasks = &graph.gantt_tasks;
    if tasks.is_empty() {
        return;
    }
    let origin = tasks.iter().filter_map(|t| t.start.as_deref().and_then(day)).reduce(f64::min);
    let (mut ends, mut cursor) = (HashMap::new(), 0.0f64);
    let spans: Vec<(f64, f64)> = tasks
        .iter()
        .map(|t| {
            let long = t.duration.as_deref().and_then(length).unwrap_or(DAYS).max(0.1);
            let after = t.after.as_deref().and_then(|a| ends.get(a).copied());
            let start = t.start.as_deref().and_then(day).or(after).unwrap_or(origin.unwrap_or(0.0) + cursor);
            ends.insert(t.id.as_str(), start + long);
            cursor = cursor.max(start + long + 0.5);
            (start, long)
        })
        .collect();
    let first = spans.iter().map(|s| s.0).fold(f64::MAX, f64::min);
    let last = spans.iter().map(|s| s.0 + s.1).fold(f64::MIN, f64::max).max(first + 1.0);
    let scale = CHART / (last - first);
    let compact = graph.gantt_display_mode.as_deref().is_some_and(|m| m.eq_ignore_ascii_case("compact"));
    let (mut rows, mut lanes, mut sections) = (Vec::new(), Vec::<(usize, f64)>::new(), Vec::<(&str, usize, usize)>::new());
    let (mut count, mut current) = (0, None);
    for (t, &(start, long)) in tasks.iter().zip(&spans) {
        if t.section.as_deref() != current {
            current = t.section.as_deref();
            lanes.clear();
            sections.extend(current.map(|name| (name, count, count)));
        }
        let row = match lanes.iter_mut().find(|(_, end)| compact && start >= *end) {
            Some(lane) => {
                lane.1 = start + long;
                lane.0
            }
            None => {
                lanes.push((count, start + long));
                count += 1;
                count - 1
            }
        };
        if let Some(s) = sections.last_mut().filter(|_| current.is_some()) {
            s.2 = s.2.max(row);
        }
        rows.push(row);
    }
    let named = sections.iter().map(|s| width(s.0, FONT - 1.0) * BOLD).fold(0.0, f64::max);
    let task_x = size::GAP_XL + if named > 0.0 { named + size::GAP_XL } else { 0.0 };
    let left = task_x + tasks.iter().map(|t| width(&t.label, FONT - 1.0)).fold(FONT * 6.5, f64::max) + size::GAP_XL;
    let (right, bottom) = (left + CHART, count as f64 * ROW);
    let x = |t: f64| left + (t - first) * scale;
    for (i, &(name, a, b)) in sections.iter().enumerate() {
        let band = Rect::new(0.0, a as f64 * ROW, right, (b + 1) as f64 * ROW);
        if i % 2 == 0 {
            canvas.shape(RoundedRect::from_rect(band, size::RADIUS_LG), Some(Tone::Editor), None);
        }
        canvas.label(Label::new(Point::new(size::GAP_XL, band.y0 + ROW / 2.0), name, FONT - 1.0, SECONDARY).bold().start(0.0));
    }
    for k in 0..=TICKS {
        let t = first + (last - first) * k as f64 / TICKS as f64;
        canvas.shape(Line::new((x(t), 0.0), (x(t), bottom)), None, Some(Pen::border(Tone::Border).dashed()));
        let text = if origin.is_some() { date(t) } else { format!("{:.0}", t - first) };
        canvas.label(Label::new(Point::new(x(t), bottom + FONT), text, FONT - 3.0, MUTED));
    }
    canvas.shape(Line::new((left, bottom), (right, bottom)), None, Some(Pen::border(Tone::Border)));
    if let Some(title) = &graph.gantt_title {
        canvas.label(Label::new(Point::new((left + right) / 2.0, -LINE), title.as_str(), FONT + 1.0, TEXT).bold());
    }
    for ((t, &(start, long)), &row) in tasks.iter().zip(&spans).zip(&rows) {
        let (middle, tall) = (row as f64 * ROW + ROW / 2.0, ROW * BAR);
        let (fill, stroke) = look(t.status);
        let x0 = x(start);
        let end = if t.status == Some(GanttStatus::Milestone) {
            let r = tall * 0.6;
            canvas.shape(polygon(&[Point::new(x0, middle - r), Point::new(x0 + r, middle), Point::new(x0, middle + r), Point::new(x0 - r, middle)], true), Some(fill), None);
            x0 + r
        } else {
            let bar = Rect::new(x0, middle - tall / 2.0, x(start + long).max(x0 + tall), middle + tall / 2.0);
            canvas.shape(RoundedRect::from_rect(bar, tall / 2.0), Some(fill), stroke.map(Pen::border));
            bar.x1
        };
        let at = if compact { end + size::GAP_LG } else { task_x };
        canvas.label(Label::new(Point::new(at, middle), t.label.as_str(), FONT - 1.0, TEXT).start(0.0));
    }
}
