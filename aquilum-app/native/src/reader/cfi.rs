use super::dom::{Dom, Kind, NodeId, utf16_len};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Part {
    pub index: u32,
    pub id: Option<String>,
    pub offset: Option<u32>,
    pub temporal: Option<f64>,
    pub spatial: Vec<f64>,
    pub text: Vec<String>,
    pub side: Option<String>,
}

pub type Path = Vec<Part>;
pub type Steps = Vec<Path>;

#[derive(Clone, Debug, PartialEq)]
pub enum Cfi {
    Point(Steps),
    Range { parent: Steps, start: Steps, end: Steps },
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Bang,
    Comma,
    Slash(u32),
    Colon(u32),
    Tilde(f64),
    At(f64),
    Bracket(String),
    Param(String, String),
}

fn int(value: &str) -> u32 {
    value.parse().unwrap_or(0)
}

fn float(value: &str) -> f64 {
    value.parse().unwrap_or(0.0)
}

fn tokenize(source: &str) -> Vec<Tok> {
    let mut tokens = Vec::new();
    let mut state: Option<String> = None;
    let mut escape = false;
    let mut value = String::new();
    let chars: Vec<Option<char>> = source.trim().chars().map(Some).chain([None]).collect();
    for ch in chars {
        if ch == Some('^') && !escape {
            escape = true;
            continue;
        }
        let digit = ch.is_some_and(|c| c.is_ascii_digit());
        let decimal = digit || ch == Some('.');
        match state.clone().as_deref() {
            Some("!") => {
                tokens.push(Tok::Bang);
                state = None;
                value.clear();
            }
            Some(",") => {
                tokens.push(Tok::Comma);
                state = None;
                value.clear();
            }
            Some(kind @ ("/" | ":")) => {
                if digit {
                    value.extend(ch);
                    escape = false;
                    continue;
                }
                tokens.push(if kind == "/" { Tok::Slash(int(&value)) } else { Tok::Colon(int(&value)) });
                state = None;
                value.clear();
            }
            Some("~") => {
                if decimal {
                    value.extend(ch);
                    escape = false;
                    continue;
                }
                tokens.push(Tok::Tilde(float(&value)));
                state = None;
                value.clear();
            }
            Some("@") => {
                if ch == Some(':') {
                    tokens.push(Tok::At(float(&value)));
                    value.clear();
                    state = Some("@".into());
                    continue;
                }
                if decimal {
                    value.extend(ch);
                    escape = false;
                    continue;
                }
                tokens.push(Tok::At(float(&value)));
                state = None;
                value.clear();
            }
            Some("[") => {
                match ch {
                    Some(';') if !escape => {
                        tokens.push(Tok::Bracket(std::mem::take(&mut value)));
                        state = Some(";".into());
                    }
                    Some(',') if !escape => {
                        tokens.push(Tok::Bracket(std::mem::take(&mut value)));
                        state = Some("[".into());
                    }
                    Some(']') if !escape => {
                        tokens.push(Tok::Bracket(std::mem::take(&mut value)));
                        state = None;
                    }
                    _ => {
                        value.extend(ch);
                        escape = false;
                    }
                }
                continue;
            }
            Some(param) if param.starts_with(';') => {
                match ch {
                    Some('=') if !escape => {
                        state = Some(format!(";{value}"));
                        value.clear();
                    }
                    Some(';') if !escape => {
                        tokens.push(Tok::Param(param.to_owned(), std::mem::take(&mut value)));
                        state = Some(";".into());
                    }
                    Some(']') if !escape => {
                        tokens.push(Tok::Param(param.to_owned(), std::mem::take(&mut value)));
                        state = None;
                    }
                    _ => {
                        value.extend(ch);
                        escape = false;
                    }
                }
                continue;
            }
            _ => {}
        }
        if let Some(c @ ('/' | ':' | '~' | '@' | '[' | '!' | ',')) = ch {
            state = Some(c.to_string());
        }
    }
    tokens
}

fn parts(tokens: &[Tok]) -> Path {
    let mut parts: Path = Vec::new();
    let mut after_slash = false;
    for token in tokens {
        if let Tok::Slash(index) = token {
            parts.push(Part { index: *index, ..Part::default() });
            after_slash = true;
            continue;
        }
        let Some(last) = parts.last_mut() else { continue };
        match token {
            Tok::Colon(offset) => last.offset = Some(*offset),
            Tok::Tilde(t) => last.temporal = Some(*t),
            Tok::At(v) => last.spatial.push(*v),
            Tok::Param(key, value) if key == ";s" => last.side = Some(value.clone()),
            Tok::Bracket(value) => {
                if after_slash && !value.is_empty() {
                    last.id = Some(value.clone());
                } else {
                    last.text.push(value.clone());
                    continue;
                }
            }
            _ => {}
        }
        after_slash = false;
    }
    parts
}

fn split<'a>(tokens: &'a [Tok], at: &Tok) -> Vec<&'a [Tok]> {
    tokens.split(|t| t == at).collect()
}

fn steps(tokens: &[Tok]) -> Steps {
    split(tokens, &Tok::Bang).into_iter().map(parts).collect()
}

fn unwrap(source: &str) -> &str {
    source.trim().strip_prefix("epubcfi(").and_then(|s| s.strip_suffix(')')).unwrap_or(source)
}

pub fn parse(source: &str) -> Cfi {
    let tokens = tokenize(unwrap(source));
    let groups = split(&tokens, &Tok::Comma);
    match groups.as_slice() {
        [parent, start, end] => Cfi::Range { parent: steps(parent), start: steps(start), end: steps(end) },
        _ => Cfi::Point(steps(&tokens)),
    }
}

fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        if matches!(c, '^' | '[' | ']' | '(' | ')' | ',' | ';' | '=') {
            out.push('^');
        }
        out.push(c);
    }
    out
}

fn part_string(part: &Part) -> String {
    let param = part.side.as_ref().map_or(String::new(), |s| format!(";s={s}"));
    let mut out = format!("/{}", part.index);
    if let Some(id) = &part.id {
        out.push_str(&format!("[{}{param}]", escape(id)));
    }
    if let Some(offset) = part.offset.filter(|_| part.index % 2 == 1) {
        out.push_str(&format!(":{offset}"));
    }
    if let Some(t) = part.temporal.filter(|t| *t != 0.0) {
        out.push_str(&format!("~{t}"));
    }
    if !part.spatial.is_empty() {
        out.push_str(&format!("@{}", part.spatial.iter().map(f64::to_string).collect::<Vec<_>>().join(":")));
    }
    if !part.text.is_empty() || (part.id.is_none() && part.side.is_some()) {
        out.push_str(&format!("[{}{param}]", part.text.iter().map(|t| escape(t)).collect::<Vec<_>>().join(",")));
    }
    out
}

fn steps_string(steps: &Steps) -> String {
    steps.iter().map(|path| path.iter().map(part_string).collect::<String>()).collect::<Vec<_>>().join("!")
}

impl std::fmt::Display for Cfi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = match self {
            Cfi::Point(steps) => steps_string(steps),
            Cfi::Range { parent, start, end } => [parent, start, end].map(steps_string).join(","),
        };
        write!(f, "epubcfi({inner})")
    }
}

fn concat(a: &Steps, b: &Steps) -> Steps {
    let Some((last, head)) = a.split_last() else { return b.clone() };
    let mut out = head.to_vec();
    let mut joined = last.clone();
    joined.extend(b.first().cloned().unwrap_or_default());
    out.push(joined);
    out.extend(b.iter().skip(1).cloned());
    out
}

impl Cfi {
    pub fn collapse(&self, to_end: bool) -> Steps {
        match self {
            Cfi::Point(steps) => steps.clone(),
            Cfi::Range { parent, start, end } => concat(parent, if to_end { end } else { start }),
        }
    }

    pub fn range(from: &Steps, to: &Steps) -> Cfi {
        let (local_from, local_to) = (from.last().cloned().unwrap_or_default(), to.last().cloned().unwrap_or_default());
        let (mut parent, mut start, mut end) = (Vec::new(), Vec::new(), Vec::new());
        let mut shared = true;
        for i in 0..local_from.len().max(local_to.len()) {
            let (a, b) = (local_from.get(i), local_to.get(i));
            let empty = |p: Option<&Part>| p.and_then(|p| p.offset).unwrap_or(0) == 0;
            shared = shared && a.map(|p| p.index) == b.map(|p| p.index) && empty(a) && empty(b);
            if shared {
                parent.extend(a.cloned());
            } else {
                start.extend(a.cloned());
                end.extend(b.cloned());
            }
        }
        let mut steps = from[..from.len().saturating_sub(1)].to_vec();
        steps.push(parent);
        Cfi::Range { parent: steps, start: vec![start], end: vec![end] }
    }

    pub fn section(&self) -> Option<usize> {
        let first = match self {
            Cfi::Point(steps) => steps.first()?,
            Cfi::Range { parent, .. } => parent.first()?,
        };
        let index = first.last()?.index;
        (index >= 2).then(|| (index / 2 - 1) as usize)
    }

    pub fn local(&self) -> Cfi {
        match self {
            Cfi::Point(steps) => Cfi::Point(steps[1.min(steps.len())..].to_vec()),
            Cfi::Range { parent, start, end } => Cfi::Range { parent: parent[1.min(parent.len())..].to_vec(), start: start.clone(), end: end.clone() },
        }
    }
}

pub fn join(base: &str, local: &str) -> String {
    format!("epubcfi({}!{})", unwrap(base), unwrap(local))
}

#[derive(Clone, Debug, PartialEq)]
enum Slot {
    Before,
    After,
    First,
    Last,
    Gap,
    Element(NodeId),
    Chunk(Vec<NodeId>),
}

fn is_text(dom: &Dom, id: NodeId) -> bool {
    matches!(dom.node(id).kind, Kind::Text(_))
}

fn index_children(dom: &Dom, node: NodeId) -> Vec<Slot> {
    let mut slots: Vec<Slot> = Vec::new();
    for &child in &dom.node(node).children {
        let text = is_text(dom, child);
        match slots.last_mut() {
            None => slots.push(if text { Slot::Chunk(vec![child]) } else { Slot::Element(child) }),
            Some(Slot::Chunk(chunk)) if text => chunk.push(child),
            Some(_) if text => slots.push(Slot::Chunk(vec![child])),
            Some(Slot::Element(_)) => {
                slots.push(Slot::Gap);
                slots.push(Slot::Element(child));
            }
            Some(_) => slots.push(Slot::Element(child)),
        }
    }
    if matches!(slots.first(), Some(Slot::Element(_))) {
        slots.insert(0, Slot::First);
    }
    if matches!(slots.last(), Some(Slot::Element(_))) {
        slots.push(Slot::Last);
    }
    slots.insert(0, Slot::Before);
    slots.push(Slot::After);
    slots
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Point {
    At(NodeId, u32),
    Before(NodeId),
    After(NodeId),
}

fn locate(dom: &Dom, root: NodeId, path: &Path) -> Option<Point> {
    let last = path.last()?;
    if let Some(element) = last.id.as_deref().and_then(|id| dom.by_id(id)) {
        return Some(Point::At(element, 0));
    }
    let mut node = root;
    let mut chunk: Option<Vec<NodeId>> = None;
    for part in path {
        if chunk.is_some() {
            return None;
        }
        let slots = index_children(dom, node);
        match slots.get(part.index as usize)? {
            Slot::First => return Some(Point::At(dom.node(node).children.first().copied().unwrap_or(node), 0)),
            Slot::Last => return Some(Point::At(dom.node(node).children.last().copied().unwrap_or(node), 0)),
            Slot::Before => return Some(Point::Before(node)),
            Slot::After => return Some(Point::After(node)),
            Slot::Gap => return None,
            Slot::Element(element) => node = *element,
            Slot::Chunk(texts) => chunk = Some(texts.clone()),
        }
    }
    let offset = last.offset.unwrap_or(0);
    let Some(texts) = chunk else { return Some(Point::At(node, offset)) };
    let mut sum = 0;
    for text in texts {
        let length = utf16_len(dom.text_of(text).unwrap_or_default());
        if sum + length >= offset {
            return Some(Point::At(text, offset - sum));
        }
        sum += length;
    }
    None
}

pub fn resolve(dom: &Dom, local: &Cfi) -> Option<(Point, Point)> {
    let start = local.collapse(false);
    let end = local.collapse(true);
    Some((locate(dom, Dom::ROOT, start.first()?)?, locate(dom, Dom::ROOT, end.first()?)?))
}

fn path_to(dom: &Dom, node: NodeId, offset: Option<u32>) -> Path {
    let Some(parent) = dom.node(node).parent else { return Vec::new() };
    let slots = index_children(dom, parent);
    let found = slots.iter().position(|slot| match slot {
        Slot::Element(e) => *e == node,
        Slot::Chunk(texts) => texts.contains(&node),
        _ => false,
    });
    let mut offset = offset;
    if let Some(Slot::Chunk(texts)) = found.map(|i| &slots[i]) {
        let before: u32 = texts.iter().take_while(|&&t| t != node).map(|&t| utf16_len(dom.text_of(t).unwrap_or_default())).sum();
        offset = offset.map(|o| o + before);
    }
    let mut path = if parent != Dom::ROOT { path_to(dom, parent, None) } else { Vec::new() };
    if let Some(index) = found {
        path.push(Part { index: index as u32, id: dom.id(node).map(str::to_owned), offset, ..Part::default() });
    }
    path
}

pub fn from_range(dom: &Dom, start: (NodeId, u32), end: (NodeId, u32)) -> Cfi {
    let from = vec![path_to(dom, start.0, Some(start.1))];
    if start == end {
        return Cfi::Point(from);
    }
    Cfi::range(&from, &vec![path_to(dom, end.0, Some(end.1))])
}

pub fn element_path(dom: &Dom, node: NodeId) -> Path {
    path_to(dom, node, None)
}

pub fn order(dom: &Dom) -> (Vec<u32>, Vec<u32>) {
    let mut enter = vec![0; dom.len()];
    let mut exit = vec![0; dom.len()];
    let mut counter = 0;
    let mut stack = vec![(Dom::ROOT, false)];
    while let Some((node, done)) = stack.pop() {
        if done {
            exit[node as usize] = counter;
            continue;
        }
        enter[node as usize] = counter;
        counter += 1;
        stack.push((node, true));
        for &child in dom.node(node).children.iter().rev() {
            stack.push((child, false));
        }
    }
    (enter, exit)
}

#[cfg_attr(not(test), allow(dead_code))]
fn boundary(dom: &Dom, enter: &[u32], exit: &[u32], point: Point) -> (u32, u32) {
    match point {
        Point::Before(node) => (enter[node as usize], 0),
        Point::After(node) => (exit[node as usize], 0),
        Point::At(node, offset) if is_text(dom, node) => (enter[node as usize], offset),
        Point::At(node, offset) => match dom.node(node).children.get(offset as usize) {
            Some(&child) => (enter[child as usize], 0),
            None => (exit[node as usize], 0),
        },
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn text(dom: &Dom, start: Point, end: Point) -> String {
    let (enter, exit) = order(dom);
    let (from, to) = (boundary(dom, &enter, &exit, start), boundary(dom, &enter, &exit, end));
    let mut texts: Vec<NodeId> = (0..dom.len() as NodeId).filter(|&n| is_text(dom, n)).collect();
    texts.sort_by_key(|&n| enter[n as usize]);
    let mut out: Vec<u16> = Vec::new();
    for node in texts {
        let at = enter[node as usize];
        if at < from.0 || at > to.0 {
            continue;
        }
        let units: Vec<u16> = dom.text_of(node).unwrap_or_default().encode_utf16().collect();
        let begin = if at == from.0 { (from.1 as usize).min(units.len()) } else { 0 };
        let finish = if at == to.0 { (to.1 as usize).min(units.len()) } else { units.len() };
        if at == to.0 && to.1 == 0 && !matches!(end, Point::At(n, _) if n == node) {
            continue;
        }
        out.extend_from_slice(&units[begin..finish.max(begin)]);
    }
    String::from_utf16_lossy(&out)
}
