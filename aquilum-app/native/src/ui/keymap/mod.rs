mod chord;
pub mod gesture;
pub mod pointer;
mod table;
#[cfg(test)]
mod tests;

use masonry::core::keyboard::{Key, Modifiers};

pub use chord::{action, action_key, extend, typing};
pub use gesture::Gesture;
use table::BINDINGS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Global,
    Text,
    Editor,
    Navigation,
    Caret,
    Table,
    Image,
    List,
    Find,
    Reader,
    Dialog,
    Slider,
    Choice,
    Number,
}

impl Scope {
    fn title(self) -> &'static str {
        match self {
            Scope::Global => "global",
            Scope::Text => "editing",
            Scope::Editor => "editor",
            Scope::Navigation => "navigation",
            Scope::Table => "tables",
            Scope::Image => "images",
            Scope::List => "searchWindow",
            Scope::Find => "pageSearchWindow",
            Scope::Reader => "reader",
            Scope::Dialog => "windows",
            Scope::Caret | Scope::Slider | Scope::Choice | Scope::Number => "controls",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    NewNote,
    NewTab,
    CloseTab,
    NextTab,
    PreviousTab,
    Back,
    Forward,
    ContextMenu,
    QuickOpen,
    Templates,
    FindInNote,
    FocusMode,
    WideMode,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Copy,
    Cut,
    Paste,
    PastePlain,
    Undo,
    Redo,
    SelectAll,
    Bold,
    Italic,
    Strike,
    Code,
    Indent,
    Outdent,
    LineBreak,
    WordLeft,
    WordRight,
    DocumentStart,
    DocumentEnd,
    PageUp,
    PageDown,
    ScrollLineUp,
    ScrollLineDown,
    DeleteWord,
    DeleteWordForward,
    LineDeleteBackward,
    CharLeft,
    CharRight,
    LineUp,
    LineDown,
    LineStart,
    LineEnd,
    DeleteBackward,
    DeleteForward,
    NewLine,
    NextCell,
    PreviousCell,
    CellBelow,
    ImageSource,
    ImageDelete,
    ListNext,
    ListPrevious,
    ListOpen,
    ListOpenInNewTab,
    ListCreate,
    FindNext,
    FindPrevious,
    NextPage,
    PreviousPage,
    Close,
    Confirm,
    Increase,
    Decrease,
    First,
    Last,
    #[cfg(not(feature = "installed"))]
    PaintFlashing,
    #[cfg(not(feature = "installed"))]
    LiveMetrics,
}

impl Command {
    pub fn repeats(self) -> bool {
        matches!(self, Command::ZoomIn | Command::ZoomOut | Command::NextTab | Command::PreviousTab | Command::Back | Command::Forward)
    }
}

pub fn command(scopes: &[Scope], key: &Key, modifiers: Modifiers) -> Option<Command> {
    BINDINGS
        .iter()
        .filter(|binding| scopes.contains(&binding.scope))
        .find(|binding| binding.chords.iter().chain(binding.hidden).any(|chord| chord.matches(key, modifiers)))
        .map(|binding| binding.command)
}

pub fn gesture(scopes: &[Scope], gesture: Gesture) -> Option<Command> {
    BINDINGS.iter().filter(|binding| scopes.contains(&binding.scope)).find(|binding| binding.gestures.contains(&gesture)).map(|binding| binding.command)
}

pub fn label(scope: Scope, command: Command) -> String {
    BINDINGS.iter().filter(|b| b.scope == scope && b.command == command).flat_map(|b| b.chords).filter(|c| c.here()).map(chord::Chord::label).collect::<Vec<_>>().join(" / ")
}

pub fn groups() -> Vec<(&'static str, Vec<(&'static str, String)>)> {
    let mut groups: Vec<(&'static str, Vec<(&'static str, String)>)> = Vec::new();
    for binding in BINDINGS {
        let chords: Vec<String> = binding.chords.iter().filter(|c| c.here()).map(chord::Chord::label).collect();
        if chords.is_empty() {
            continue;
        }
        let title = binding.scope.title();
        if groups.last().is_none_or(|(last, _)| *last != title) {
            groups.push((title, Vec::new()));
        }
        groups.last_mut().expect("группа").1.push((binding.label, chords.join(" / ")));
    }
    groups
}

pub fn cycle(current: usize, count: usize, forward: bool) -> usize {
    if count == 0 {
        0
    } else if forward {
        (current + 1) % count
    } else {
        (current + count - 1) % count
    }
}
