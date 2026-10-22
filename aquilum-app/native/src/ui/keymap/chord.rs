use masonry::core::keyboard::{Key, Modifiers, NamedKey};

const MAC: bool = cfg!(target_os = "macos");

pub fn action_key() -> &'static str {
    if MAC { "⌘" } else { "Ctrl" }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Name {
    Char(char),
    Named(NamedKey),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shift {
    No,
    Yes,
    Any,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    All,
    Mac,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    pub name: Name,
    pub action: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: Shift,
    pub os: Os,
}

pub const fn letter(c: char) -> Chord {
    Chord { name: Name::Char(c), action: false, ctrl: false, alt: false, shift: Shift::No, os: Os::All }
}

pub const fn named(key: NamedKey) -> Chord {
    Chord { name: Name::Named(key), action: false, ctrl: false, alt: false, shift: Shift::No, os: Os::All }
}

pub fn action(modifiers: Modifiers) -> bool {
    if MAC { modifiers.meta() } else { modifiers.ctrl() }
}

pub fn typing(modifiers: Modifiers) -> bool {
    !action(modifiers) || !MAC && modifiers.alt()
}

impl Chord {
    pub const fn action(self) -> Self {
        Chord { action: true, ..self }
    }

    pub const fn ctrl(self) -> Self {
        Chord { ctrl: true, ..self }
    }

    pub const fn alt(self) -> Self {
        Chord { alt: true, ..self }
    }

    pub const fn shift(self) -> Self {
        Chord { shift: Shift::Yes, ..self }
    }

    pub const fn any_shift(self) -> Self {
        Chord { shift: Shift::Any, ..self }
    }

    pub const fn only(self, os: Os) -> Self {
        Chord { os, ..self }
    }

    pub fn here(&self) -> bool {
        match self.os {
            Os::All => true,
            Os::Mac => MAC,
            Os::Other => !MAC,
        }
    }

    pub fn matches(&self, key: &Key, modifiers: Modifiers) -> bool {
        let name = match (self.name, key) {
            (Name::Char(c), Key::Character(text)) => text.chars().eq(c.to_lowercase()) || text.chars().eq(c.to_uppercase()),
            (Name::Named(want), Key::Named(got)) => want == *got,
            _ => false,
        };
        let (ctrl, meta) = if MAC { (self.ctrl, self.action) } else { (self.ctrl || self.action, false) };
        let shift = match self.shift {
            Shift::No => !modifiers.shift(),
            Shift::Yes => modifiers.shift(),
            Shift::Any => true,
        };
        self.here() && name && shift && modifiers.ctrl() == ctrl && (!MAC || modifiers.meta() == meta) && modifiers.alt() == self.alt
    }

    pub fn label(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.action {
            parts.push(action_key().into());
        }
        if self.ctrl && (MAC || !self.action) {
            parts.push("Ctrl".into());
        }
        if self.alt {
            parts.push(if MAC { "⌥" } else { "Alt" }.into());
        }
        if self.shift == Shift::Yes {
            parts.push("Shift".into());
        }
        parts.push(match self.name {
            Name::Char(' ') => "Space".into(),
            Name::Char(c) => c.to_uppercase().collect(),
            Name::Named(key) => key_label(key).into(),
        });
        parts.join(" + ")
    }
}

fn key_label(key: NamedKey) -> &'static str {
    match key {
        NamedKey::Enter => "↵",
        NamedKey::ArrowUp => "↑",
        NamedKey::ArrowDown => "↓",
        NamedKey::ArrowLeft => "←",
        NamedKey::ArrowRight => "→",
        NamedKey::PageUp => "PgUp",
        NamedKey::PageDown => "PgDn",
        NamedKey::Escape => "Esc",
        NamedKey::Backspace => "Backspace",
        NamedKey::Delete => "Delete",
        NamedKey::Insert => "Insert",
        NamedKey::Home => "Home",
        NamedKey::End => "End",
        NamedKey::Tab => "Tab",
        _ => "?",
    }
}
