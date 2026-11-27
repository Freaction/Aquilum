use mermaid_rs_renderer::parse_mermaid;

use super::canvas::Canvas;
use super::sequence;

const STARTER: &str = "Client";

enum Open {
    Call { callee: String, caller: String, result: Option<String> },
    Frame,
    Group,
}

#[derive(Default)]
struct Script {
    people: Vec<(String, bool)>,
    out: Vec<String>,
    stack: Vec<Open>,
    starter: Option<String>,
}

impl Script {
    fn person(&mut self, name: &str, actor: bool) {
        if !name.is_empty() && !self.people.iter().any(|(p, _)| p == name) {
            self.people.push((name.to_owned(), actor));
        }
    }

    fn current(&mut self) -> String {
        let caller = self.stack.iter().rev().find_map(|o| if let Open::Call { callee, .. } = o { Some(callee.clone()) } else { None });
        let caller = caller.or_else(|| self.starter.clone()).unwrap_or_else(|| STARTER.to_owned());
        if !self.people.iter().any(|(p, _)| *p == caller) {
            self.people.insert(0, (caller.clone(), true));
        }
        caller
    }

    fn close(&mut self) {
        match self.stack.pop() {
            Some(Open::Call { callee, caller, result: Some(result) }) => self.out.push(format!("{callee}-->>{caller}: {result}")),
            Some(Open::Frame) => self.out.push("end".to_owned()),
            _ => {}
        }
    }

    fn call(&mut self, text: &str, opens: bool) {
        let paren = text.find('(').unwrap_or(text.len());
        let (result, rhs) = match text[..paren].split_once('=') {
            Some((lhs, _)) => (lhs.split_whitespace().last().map(str::to_owned), text[text.find('=').unwrap_or(0) + 1..].trim()),
            None => (None, text),
        };
        let caller = self.current();
        let (callee, message) = match rhs.strip_prefix("new ") {
            Some(created) => (created[..created.find('(').unwrap_or(created.len())].trim().to_owned(), "«create»".to_owned()),
            None => match rhs[..rhs.find('(').unwrap_or(rhs.len())].rsplit_once('.') {
                Some((target, _)) => (target.trim().to_owned(), rhs[target.len() + 1..].trim().to_owned()),
                None => (caller.clone(), rhs.to_owned()),
            },
        };
        self.person(&callee, false);
        self.out.push(format!("{caller}->>{callee}: {message}"));
        if opens {
            self.stack.push(Open::Call { callee, caller, result });
        } else if let Some(result) = result {
            self.out.push(format!("{callee}-->>{caller}: {result}"));
        }
    }

    fn line(&mut self, line: &str) {
        let opens = line.ends_with('{');
        let line = line.trim_end_matches('{').trim();
        let cond = |line: &str| line.find('(').zip(line.rfind(')')).map_or("", |(a, b)| line[a + 1..b].trim()).to_owned();
        let word = line.split(|c: char| !c.is_alphanumeric() && c != '@').next().unwrap_or_default();
        let frame = match word {
            "if" => Some(format!("alt {}", cond(line))),
            "else" if line.contains("if") => Some(format!("else {}", cond(line))),
            "else" => Some("else".to_owned()),
            "while" | "for" | "forEach" | "loop" => Some(format!("loop {}", cond(line))),
            "opt" | "par" => Some(word.to_owned()),
            "try" => Some("alt try".to_owned()),
            "catch" => Some(format!("else catch {}", cond(line))),
            "finally" => Some("else finally".to_owned()),
            "critical" => Some(format!("critical {}", cond(line))),
            _ => None,
        };
        if let Some(frame) = frame {
            self.out.push(frame);
            if !frame_continues(word) {
                self.stack.push(Open::Frame);
            }
        } else if word == "group" || word == "section" {
            self.stack.push(Open::Group);
        } else if let Some(name) = line.strip_prefix("@Starter(") {
            let name = name.trim_end_matches(')').trim().to_owned();
            self.person(&name, true);
            self.starter = Some(name);
        } else if let Some(rest) = line.strip_prefix('@') {
            let words: Vec<&str> = rest.split_whitespace().filter(|w| !w.starts_with('#') && !w.starts_with("<<")).collect();
            if let (Some(kind), Some(name)) = (words.first(), words.last()) {
                self.person(name, *kind == "Actor");
            }
        } else if let Some(value) = line.strip_prefix("return ").or_else(|| line.strip_prefix("@return ")) {
            if let Some(Open::Call { callee, caller, .. }) = self.stack.iter().rev().find(|o| matches!(o, Open::Call { .. })) {
                self.out.push(format!("{callee}-->>{caller}: {}", value.trim()));
            }
        } else if let Some((from, rest)) = line.split_once("->") {
            let (to, message) = rest.split_once(':').unwrap_or((rest, ""));
            let (from, to) = (from.trim(), to.trim());
            self.person(from, false);
            self.person(to, false);
            self.out.push(format!("{from}->>{to}: {}", message.trim()));
        } else if !line.is_empty() && !line.starts_with("title") {
            self.call(line, opens);
        }
    }
}

fn frame_continues(word: &str) -> bool {
    matches!(word, "else" | "catch" | "finally")
}

pub(super) fn draw(lines: &[&str], canvas: &mut Canvas) {
    let mut script = Script::default();
    for raw in lines {
        let line = raw.trim().trim_end_matches(';');
        if line.starts_with("//") {
            continue;
        }
        let rest = match line.strip_prefix('}') {
            Some(rest) => {
                let rest = rest.trim();
                if !frame_continues(rest.split(|c: char| !c.is_alphanumeric()).next().unwrap_or_default()) {
                    script.close();
                }
                rest
            }
            None => line,
        };
        if !rest.is_empty() {
            script.line(rest);
        }
    }
    while !script.stack.is_empty() {
        script.close();
    }
    if script.out.is_empty() {
        return;
    }
    let people = script.people.iter().map(|(name, actor)| format!("{} {name}", if *actor { "actor" } else { "participant" }));
    let text = ["sequenceDiagram".to_owned()].into_iter().chain(people).chain(script.out.iter().cloned()).collect::<Vec<_>>().join("\n");
    if let Ok(parsed) = parse_mermaid(&text) {
        sequence::draw(&parsed.graph, canvas);
    }
}
