use masonry::core::keyboard::NamedKey as K;

use super::chord::{Chord, Os, letter, named};
use super::gesture::Gesture;
use super::{Command, Command::*, Scope};

pub struct Binding {
    pub command: Command,
    pub scope: Scope,
    pub label: &'static str,
    pub chords: &'static [Chord],
    pub hidden: &'static [Chord],
    pub gestures: &'static [Gesture],
}

const fn bind(scope: Scope, command: Command, label: &'static str, chords: &'static [Chord]) -> Binding {
    Binding { command, scope, label, chords, hidden: &[], gestures: &[] }
}

impl Binding {
    const fn hidden(self, hidden: &'static [Chord]) -> Binding {
        Binding { hidden, ..self }
    }

    const fn on(self, gestures: &'static [Gesture]) -> Binding {
        Binding { gestures, ..self }
    }
}

pub const BINDINGS: &[Binding] = &[
    bind(Scope::Global, NewNote, "newNote", &[letter('n').action()]),
    bind(Scope::Global, NewTab, "newTab", &[letter('t').action()]),
    bind(Scope::Global, CloseTab, "closeTab", &[letter('w').action()]),
    bind(Scope::Global, NextTab, "nextTab", &[named(K::Tab).ctrl(), named(K::PageDown).ctrl()]),
    bind(Scope::Global, PreviousTab, "previousTab", &[named(K::Tab).ctrl().shift(), named(K::PageUp).ctrl()]),
    bind(Scope::Global, Back, "back", &[named(K::ArrowLeft).alt().only(Os::Other), letter('[').action().only(Os::Mac)]).hidden(&[named(K::BrowserBack)]).on(&[Gesture::MouseBack]),
    bind(Scope::Global, Forward, "forward", &[named(K::ArrowRight).alt().only(Os::Other), letter(']').action().only(Os::Mac)]).hidden(&[named(K::BrowserForward)]).on(&[Gesture::MouseForward]),
    bind(Scope::Global, ContextMenu, "contextMenu", &[]).on(&[Gesture::LongPress]),
    bind(Scope::Global, QuickOpen, "globalSearch", &[letter('o').action()]),
    bind(Scope::Global, Templates, "newFromTemplate", &[letter('u').action()]),
    bind(Scope::Global, FindInNote, "pageSearch", &[letter('f').action()]),
    bind(Scope::Global, FocusMode, "focusMode", &[letter('f').action().shift()]),
    bind(Scope::Global, WideMode, "wideMode", &[letter('l').action().shift()]),
    bind(Scope::Global, ZoomIn, "zoomIn", &[letter('+').action().any_shift()]).hidden(&[letter('=').action().any_shift()]),
    #[cfg(not(feature = "installed"))]
    bind(Scope::Global, PaintFlashing, "paintFlashing", &[]).hidden(&[named(K::F12)]),
    #[cfg(not(feature = "installed"))]
    bind(Scope::Global, LiveMetrics, "liveMetrics", &[]).hidden(&[letter('p').action().shift()]),
    bind(Scope::Global, ZoomOut, "zoomOut", &[letter('-').action().any_shift()]),
    bind(Scope::Global, ZoomReset, "zoomReset", &[letter('0').action()]),
    bind(Scope::Text, Copy, "copy", &[letter('c').action(), named(K::Insert).ctrl().only(Os::Other)]),
    bind(Scope::Text, Cut, "cut", &[letter('x').action(), named(K::Delete).shift().only(Os::Other)]),
    bind(Scope::Text, Paste, "paste", &[letter('v').action(), named(K::Insert).shift().only(Os::Other)]),
    bind(Scope::Text, PastePlain, "pastePlain", &[letter('v').action().shift()]),
    bind(Scope::Text, Undo, "undo", &[letter('z').action()]),
    bind(Scope::Text, Redo, "redo", &[letter('y').action().any_shift(), letter('z').action().shift()]),
    bind(Scope::Text, SelectAll, "selectAll", &[letter('a').action()]),
    bind(Scope::Editor, Bold, "bold", &[letter('b').action()]),
    bind(Scope::Editor, Italic, "italic", &[letter('i').action()]),
    bind(Scope::Editor, Strike, "strikethrough", &[letter('s').action().shift()]),
    bind(Scope::Editor, Code, "inlineCode", &[letter('e').action()]),
    bind(Scope::Editor, Indent, "indent", &[named(K::Tab)]),
    bind(Scope::Editor, Outdent, "outdent", &[named(K::Tab).shift()]),
    bind(Scope::Editor, LineBreak, "lineBreak", &[named(K::Enter).shift()]),
    bind(Scope::Navigation, WordLeft, "wordLeft", &[named(K::ArrowLeft).alt().any_shift().only(Os::Mac), named(K::ArrowLeft).action().any_shift().only(Os::Other)]),
    bind(Scope::Navigation, WordRight, "wordRight", &[named(K::ArrowRight).alt().any_shift().only(Os::Mac), named(K::ArrowRight).action().any_shift().only(Os::Other)]),
    bind(Scope::Navigation, DocumentStart, "documentStart", &[named(K::ArrowUp).action().any_shift().only(Os::Mac), named(K::Home).action().any_shift().only(Os::Other)]).hidden(&[named(K::Home).action().any_shift().only(Os::Mac)]),
    bind(Scope::Navigation, DocumentEnd, "documentEnd", &[named(K::ArrowDown).action().any_shift().only(Os::Mac), named(K::End).action().any_shift().only(Os::Other)]).hidden(&[named(K::End).action().any_shift().only(Os::Mac)]),
    bind(Scope::Navigation, PageUp, "pageUp", &[named(K::PageUp).any_shift()]),
    bind(Scope::Navigation, PageDown, "pageDown", &[named(K::PageDown).any_shift()]),
    bind(Scope::Navigation, ScrollLineUp, "scrollLineUp", &[named(K::ArrowUp).ctrl().only(Os::Other)]),
    bind(Scope::Navigation, ScrollLineDown, "scrollLineDown", &[named(K::ArrowDown).ctrl().only(Os::Other)]),
    bind(Scope::Navigation, DeleteWord, "deleteWord", &[named(K::Backspace).alt().only(Os::Mac), named(K::Backspace).action().only(Os::Other)]).hidden(&[named(K::Backspace).ctrl().only(Os::Mac)]),
    bind(Scope::Navigation, DeleteWordForward, "deleteWordForward", &[named(K::Delete).alt().only(Os::Mac), named(K::Delete).action().only(Os::Other)]).hidden(&[named(K::Delete).ctrl().only(Os::Mac)]),
    bind(Scope::Navigation, LineDeleteBackward, "lineDeleteBackward", &[named(K::Backspace).action().only(Os::Mac)]),
    bind(Scope::Caret, CharLeft, "charLeft", &[]).hidden(&[named(K::ArrowLeft).any_shift()]),
    bind(Scope::Caret, CharRight, "charRight", &[]).hidden(&[named(K::ArrowRight).any_shift()]),
    bind(Scope::Caret, LineUp, "lineUp", &[]).hidden(&[named(K::ArrowUp).any_shift()]),
    bind(Scope::Caret, LineDown, "lineDown", &[]).hidden(&[named(K::ArrowDown).any_shift()]),
    bind(Scope::Caret, LineStart, "lineStart", &[]).hidden(&[named(K::Home).any_shift(), named(K::ArrowLeft).action().any_shift().only(Os::Mac)]),
    bind(Scope::Caret, LineEnd, "lineEnd", &[]).hidden(&[named(K::End).any_shift(), named(K::ArrowRight).action().any_shift().only(Os::Mac)]),
    bind(Scope::Caret, DeleteBackward, "deleteBackward", &[]).hidden(&[named(K::Backspace).any_shift()]),
    bind(Scope::Caret, DeleteForward, "deleteForward", &[]).hidden(&[named(K::Delete)]),
    bind(Scope::Editor, NewLine, "newLine", &[]).hidden(&[named(K::Enter)]),
    bind(Scope::Editor, Close, "cancel", &[]).hidden(&[named(K::Escape)]),
    bind(Scope::Table, NextCell, "nextCell", &[named(K::Tab)]),
    bind(Scope::Table, PreviousCell, "previousCell", &[named(K::Tab).shift()]),
    bind(Scope::Table, CellBelow, "cellBelow", &[named(K::Enter)]),
    bind(Scope::Table, Close, "leaveTable", &[named(K::Escape)]),
    bind(Scope::Image, ImageSource, "imageSource", &[named(K::Enter)]),
    bind(Scope::Image, ImageDelete, "imageDelete", &[named(K::Delete), named(K::Backspace)]),
    bind(Scope::Image, Close, "imageDeselect", &[named(K::Escape)]),
    bind(Scope::List, ListNext, "searchNext", &[named(K::ArrowDown)]),
    bind(Scope::List, ListPrevious, "searchPrevious", &[named(K::ArrowUp)]),
    bind(Scope::List, ListOpen, "searchOpen", &[named(K::Enter)]),
    bind(Scope::List, ListOpenInNewTab, "searchOpenNewPane", &[named(K::Enter).action()]),
    bind(Scope::List, ListCreate, "searchCreate", &[named(K::Enter).shift()]),
    bind(Scope::Find, FindNext, "searchNext", &[named(K::Enter)]),
    bind(Scope::Find, FindPrevious, "searchPrevious", &[named(K::Enter).shift()]),
    bind(Scope::Find, Close, "closeSearch", &[named(K::Escape)]),
    bind(Scope::Reader, NextPage, "nextPage", &[named(K::ArrowRight), named(K::PageDown), letter(' ')]),
    bind(Scope::Reader, PreviousPage, "previousPage", &[named(K::ArrowLeft), named(K::PageUp)]),
    bind(Scope::Reader, Close, "closeReader", &[named(K::Escape)]),
    bind(Scope::Dialog, Close, "closeWindow", &[named(K::Escape)]),
    bind(Scope::Dialog, Confirm, "confirm", &[named(K::Enter)]),
    bind(Scope::Slider, Increase, "increase", &[]).hidden(&[named(K::ArrowRight), named(K::ArrowUp)]),
    bind(Scope::Slider, Decrease, "decrease", &[]).hidden(&[named(K::ArrowLeft), named(K::ArrowDown)]),
    bind(Scope::Slider, First, "first", &[]).hidden(&[named(K::Home)]),
    bind(Scope::Slider, Last, "last", &[]).hidden(&[named(K::End)]),
    bind(Scope::Choice, Increase, "next", &[]).hidden(&[named(K::ArrowRight), named(K::ArrowDown)]),
    bind(Scope::Choice, Decrease, "previous", &[]).hidden(&[named(K::ArrowLeft), named(K::ArrowUp)]),
    bind(Scope::Choice, First, "first", &[]).hidden(&[named(K::Home)]),
    bind(Scope::Choice, Last, "last", &[]).hidden(&[named(K::End)]),
    bind(Scope::Number, Increase, "increase", &[]).hidden(&[named(K::ArrowUp)]),
    bind(Scope::Number, Decrease, "decrease", &[]).hidden(&[named(K::ArrowDown)]),
];
