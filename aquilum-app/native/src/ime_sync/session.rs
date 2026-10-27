use super::{AppState, Edit, ImeState, Message, Mirror};

#[derive(Default)]
pub struct Session {
    mirror: Mirror,
    id: i32,
    base: Option<usize>,
    applied: i32,
    inbox: Vec<Message>,
}

impl Session {
    pub fn start(&mut self) -> i32 {
        self.stop();
        (self.mirror, self.base) = (Mirror::default(), None);
        self.id
    }

    pub fn stop(&mut self) {
        self.id += 1;
    }

    pub fn base(&self) -> Option<usize> {
        self.base
    }

    pub fn received(&mut self, message: Message) {
        self.inbox.push(message);
    }

    pub fn take(&mut self) -> Vec<Edit> {
        let mut edits = Vec::new();
        for message in std::mem::take(&mut self.inbox) {
            self.applied = message.seq;
            if message.session != self.id {
                continue;
            }
            self.base = message.base;
            if message.echo {
                self.mirror.echoed(&message.state);
                continue;
            }
            let base = message.base.unwrap_or(0);
            edits.extend(self.mirror.ime_changed(&message.state).into_iter().map(|edit| match edit {
                Edit::Select { anchor, cursor } => Edit::Select { anchor: base + anchor, cursor: base + cursor },
                edit => edit,
            }));
        }
        edits
    }

    pub fn report(&self, base: usize, app: &AppState) -> Option<(i32, ImeState)> {
        (self.mirror.differs(app) || self.base != Some(base)).then(|| (self.applied, app.to_ime()))
    }
}
