use std::marker::PhantomData;

use masonry::app::RenderRoot;
use masonry::core::{FromDynWidget, NewWidget, Widget, WidgetId, WidgetMut, WidgetRef};

pub struct Handle<W: ?Sized> {
    id: WidgetId,
    widget: PhantomData<fn() -> Box<W>>,
}

impl<W: ?Sized> Clone for Handle<W> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<W: ?Sized> Copy for Handle<W> {}

impl<W: Widget + FromDynWidget + ?Sized> Handle<W> {
    pub fn of(widget: &NewWidget<W>) -> Self {
        Handle { id: widget.id(), widget: PhantomData }
    }

    pub fn id(self) -> WidgetId {
        self.id
    }

    pub fn edit<R>(self, root: &mut RenderRoot, f: impl FnOnce(WidgetMut<'_, W>) -> R) -> Option<R> {
        root.has_widget(self.id).then(|| root.edit_widget(self.id, |mut widget| f(widget.downcast())))
    }

    pub fn get(self, root: &RenderRoot) -> Option<WidgetRef<'_, W>> {
        root.get_widget(self.id)?.downcast::<W>()
    }
}
