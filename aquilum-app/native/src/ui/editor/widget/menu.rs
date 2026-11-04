use super::*;
use crate::ui::editor::ContextMenu;
use crate::ui::editor::table::Outcome;
use crate::ui::keymap;
use crate::ui::keymap::gesture::MENU_SETTLE;
use masonry::core::PointerType;
use std::time::Instant;

#[derive(Clone, Copy, Default)]
pub(super) enum Menu {
    #[default]
    Closed,
    Pressed(u8),
    Shown(Option<Rect>, ContextMenu, Option<Instant>),
}

impl TextEditor {
    fn open_menu(&mut self, transform: Affine, at: Point, submit: impl FnOnce(TextAction)) {
        let bounds = self.selection_bounds();
        let mut local = bounds.unwrap_or(Rect::from_points(at, at));
        if bounds.is_some() && self.grips_wanted() {
            local.y1 += size::SELECTION_HANDLE_SIZE;
        }
        let inserts = !self.single_line && self.doc.markdown.is_some() && !self.table_active();
        let menu = ContextMenu { around: transform.transform_rect_bbox(local), selected: bounds.is_some(), inserts };
        self.menu = Menu::Shown(Some(local), menu, None);
        submit(TextAction::Menu(Some(menu)));
    }

    pub(super) fn follow_menu(&mut self, transform: Affine, submit: impl FnOnce(TextAction)) -> Option<Instant> {
        let Menu::Shown(Some(local), menu, moving) = self.menu else { return None };
        let around = transform.transform_rect_bbox(local);
        if around == menu.around {
            return None;
        }
        if moving.is_none() {
            submit(TextAction::Menu(None));
        }
        let settle = Instant::now() + MENU_SETTLE;
        self.menu = Menu::Shown(Some(local), ContextMenu { around, ..menu }, Some(settle));
        Some(settle)
    }

    pub(super) fn settle_menu(&mut self, submit: impl FnOnce(TextAction)) -> Option<Instant> {
        let Menu::Shown(local, menu, Some(settle)) = self.menu else { return None };
        if Instant::now() < settle {
            return Some(settle);
        }
        self.menu = Menu::Shown(local, menu, None);
        submit(TextAction::Menu(Some(menu)));
        None
    }

    pub(super) fn close_menu(&mut self, submit: impl FnOnce(TextAction)) {
        if matches!(std::mem::take(&mut self.menu), Menu::Shown(..)) {
            submit(TextAction::Menu(None));
        }
    }

    pub fn menu_closed(this: &mut WidgetMut<'_, Self>) {
        this.widget.close_menu(|_| {});
    }

    pub(super) fn release_menu(&mut self, ctx: &mut EventCtx<'_>, up: Option<(Point, PointerType)>) {
        let Menu::Pressed(count) = self.menu else { return };
        self.menu = Menu::Closed;
        if let Some((at, pointer)) = up
            && keymap::pointer::selection_menu(pointer, count, self.selection_bounds().is_some())
        {
            self.open_menu(ctx.window_transform(), at, |shown| ctx.submit_action::<TextAction>(shown));
        }
    }

    pub(super) fn context_press(&mut self, ctx: &mut EventCtx<'_>, point: Point, reach: f64) {
        if self.table_context(ctx, point) {
            return;
        }
        if self.table_active() {
            self.leave_table();
        }
        self.media.selected = self.media_under(point, reach).map(|(start, _)| start);
        let (offset, affinity) = self.hit(point);
        let range = self.selection();
        if offset < range.start || offset > range.end {
            self.move_to((offset, affinity), false);
        }
        ctx.request_focus();
        self.open_menu(ctx.window_transform(), point, |shown| ctx.submit_action::<TextAction>(shown));
        self.refresh(ctx);
    }

    pub(super) fn menu_text(&mut self, ctx: &mut EventCtx<'_>, event: &TextEvent) -> bool {
        let quiet = match event {
            TextEvent::Keyboard(key) => key.state != KeyState::Down,
            TextEvent::Ime(Ime::Preedit(text, _)) => text.is_empty(),
            TextEvent::Ime(ime) => matches!(ime, Ime::Enabled | Ime::Disabled),
            TextEvent::WindowFocusChange(focused) => *focused,
            TextEvent::ClipboardPaste(_) => false,
        };
        if quiet {
            return false;
        }
        if self.grips.off() {
            ctx.request_compose();
        }
        if !matches!(self.menu, Menu::Shown(..)) {
            return false;
        }
        self.close_menu(|closed| ctx.submit_action::<TextAction>(closed));
        let escape = matches!(event, TextEvent::Keyboard(key) if keymap::closes(&key.key, key.modifiers));
        if escape {
            ctx.set_handled();
        }
        escape
    }

    fn selection_bounds(&self) -> Option<Rect> {
        if let Some(start) = self.media.selected {
            return self.media_view(start).map(|view| view.frame + self.embed_origin(self.doc.at_offset(start)));
        }
        if let Some((_, rects)) = self.table_caret() {
            return rects.into_iter().reduce(|a, b| a.union(b));
        }
        if let Some(session) = &self.table
            && let Some(cells) = session.range
            && let Some((range, view)) = self.doc.table_at(session.start)
        {
            return Some(view.range_rect(cells, session.scroll_x) + self.embed_origin(self.doc.at_offset(range.start)));
        }
        let [start, end] = self.ends();
        (!self.selection().is_empty()).then(|| start.union(end))
    }

    pub(super) fn clip(&mut self, cut: bool) -> Option<String> {
        if let Some(start) = self.media.selected {
            let text = self.doc.copy(self.media_source(start));
            if cut {
                self.remove_media(start);
            }
            return Some(text);
        }
        if self.table_active() {
            let (session, view) = self.table_parts()?;
            let text = session.selected(view).or_else(|| session.range_text(view))?;
            if cut && let Outcome::Write(model) = if session.active.is_some() { session.insert(view, "") } else { session.clear_range(view) } {
                self.write_table(&model);
            }
            return Some(text);
        }
        let text = self.selected_text()?;
        if cut {
            self.insert("", Kind::Other);
        }
        Some(text)
    }

    pub(super) fn select_all(&mut self) {
        self.media.selected = None;
        if self.table_active()
            && let Some((session, view)) = self.table_parts()
        {
            return session.select_all(view);
        }
        (self.anchor, self.focus) = (0, self.doc.len());
    }

    pub fn to_clipboard(this: &mut WidgetMut<'_, Self>, cut: bool) {
        let Some(text) = this.widget.clip(cut) else { return };
        this.ctx.set_clipboard(text);
        if cut {
            this.widget.follow_caret = true;
            Self::edited(this);
        }
    }

    pub fn select_everything(this: &mut WidgetMut<'_, Self>) {
        let editor = &mut *this.widget;
        editor.select_all();
        if let Menu::Shown(_, menu, _) = editor.menu {
            let menu = ContextMenu { selected: editor.selection_bounds().is_some(), ..menu };
            editor.menu = Menu::Shown(None, menu, None);
            this.ctx.submit_action::<TextAction>(TextAction::Menu(Some(menu)));
        }
        Self::repaint(this);
    }
}
