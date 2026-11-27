#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mark {
    Normal,
    Reverse,
    Highlight,
    Merge,
    Pick,
}

pub(super) struct Commit {
    pub lane: usize,
    pub parents: Vec<usize>,
    pub id: Option<String>,
    pub tag: Option<String>,
    pub mark: Mark,
}

pub(super) struct Git {
    pub lanes: Vec<(String, f64)>,
    pub commits: Vec<Commit>,
}

fn attr(line: &str, key: &str) -> Option<String> {
    let at = line.find(&format!("{key}:"))? + key.len() + 1;
    let rest = line[at..].trim_start();
    let value = match rest.strip_prefix('"') {
        Some(quoted) => quoted.split('"').next()?,
        None => rest.split_whitespace().next()?,
    };
    Some(value.to_owned())
}

pub(super) fn parse(lines: &[&str]) -> Git {
    let mut git = Git { lanes: vec![("main".to_owned(), 0.0)], commits: Vec::new() };
    let mut heads: Vec<Option<usize>> = vec![None];
    let mut current = 0;
    for line in lines.iter().map(|l| l.trim()) {
        let mut words = line.split_whitespace();
        let word = words.next().unwrap_or_default();
        let name = words.next().map(|n| n.trim_matches('"').to_owned());
        let lane = |git: &Git| name.as_ref().and_then(|n| git.lanes.iter().position(|(l, _)| l == n));
        let mut parents: Vec<usize> = heads[current].into_iter().collect();
        let mark = match attr(line, "type").as_deref() {
            Some("REVERSE") => Mark::Reverse,
            Some("HIGHLIGHT") => Mark::Highlight,
            _ => Mark::Normal,
        };
        let mark = match word {
            "commit" => mark,
            "merge" => {
                let Some(other) = lane(&git) else { continue };
                parents.extend(heads[other]);
                Mark::Merge
            }
            "cherry-pick" => {
                let Some(source) = attr(line, "id").and_then(|id| git.commits.iter().position(|c| c.id.as_deref() == Some(id.as_str()))) else { continue };
                parents.push(source);
                Mark::Pick
            }
            "branch" => {
                if let Some(name) = name.clone() {
                    let order = attr(line, "order").and_then(|o| o.parse().ok()).unwrap_or(git.lanes.len() as f64);
                    git.lanes.push((name, order));
                    heads.push(heads[current]);
                    current = git.lanes.len() - 1;
                }
                continue;
            }
            "checkout" | "switch" => {
                current = lane(&git).unwrap_or(current);
                continue;
            }
            _ => continue,
        };
        let id = if mark == Mark::Pick { None } else { attr(line, "id") };
        git.commits.push(Commit { lane: current, parents, id, tag: attr(line, "tag"), mark });
        heads[current] = Some(git.commits.len() - 1);
    }
    git
}
