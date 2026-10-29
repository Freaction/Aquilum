use super::writer::Writer;

pub(super) enum Container {
    Quote,
    List { ordered: bool, next: u32, marker: Option<String>, implicit: bool },
}

impl Writer {
    pub fn heading(&mut self, level: usize) {
        self.block();
        self.lead = format!("{} ", "#".repeat(level));
        self.rich = true;
    }

    pub fn quote(&mut self) {
        self.block();
        self.containers.push(Container::Quote);
        self.rich = true;
    }

    pub fn list(&mut self, ordered: bool, next: u32) {
        self.block();
        self.containers.push(Container::List { ordered, next, marker: None, implicit: false });
    }

    pub fn end_implicit(&mut self) {
        if matches!(self.containers.last(), Some(Container::List { implicit: true, .. })) {
            self.pop();
        }
    }

    pub fn item(&mut self) {
        if !matches!(self.containers.last(), Some(Container::List { .. })) {
            self.block();
            self.containers.push(Container::List { ordered: false, next: 1, marker: None, implicit: true });
        }
        self.block();
        for container in &mut self.containers {
            if let Container::List { marker, .. } = container {
                *marker = None;
            }
        }
        if let Some(Container::List { ordered, next, marker, .. }) = self.containers.last_mut() {
            *marker = Some(if *ordered { format!("{next}. ") } else { "- ".to_owned() });
            *next += 1;
        }
        self.rich = true;
    }

    pub fn pop(&mut self) {
        self.end_line();
        self.containers.pop();
        self.block();
    }

    pub fn rule(&mut self) {
        self.block();
        self.begin();
        self.out.push_str("---");
        self.end_line();
        self.block();
        self.rich = true;
    }

    pub fn code_block(&mut self, lang: &str, code: &str) {
        let code = code.trim_matches('\n');
        if code.is_empty() {
            return;
        }
        let fence = "`".repeat(code.split(|c| c != '`').map(str::len).max().unwrap_or(0).max(2) + 1);
        self.block();
        for line in std::iter::once(format!("{fence}{lang}")).chain(code.lines().map(str::to_owned)).chain([fence]) {
            self.begin();
            self.out.push_str(&line);
            self.end_line();
        }
        self.block();
        self.rich = true;
    }
}
