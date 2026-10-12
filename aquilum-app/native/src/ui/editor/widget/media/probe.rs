use super::*;

impl TextEditor {
    #[cfg(feature = "memory-probe")]
    pub fn probe_stats(&self) -> String {
        self.doc.probe_stats()
    }

    #[cfg(feature = "memory-probe")]
    pub fn probe_evict(this: &mut masonry::core::WidgetMut<'_, Self>, embeds: bool) {
        let keep = this.widget.painted;
        this.widget.doc.probe_evict(keep, embeds);
    }

    #[cfg(feature = "memory-probe")]
    pub fn probe_play(this: &mut masonry::core::WidgetMut<'_, Self>) {
        let path = this.widget.doc.paragraphs().iter().filter_map(|p| p.embed.as_deref()).find_map(|e| match &e.view {
            View::Media(view) if view.video => view.path.clone(),
            _ => None,
        });
        if let Some(path) = path
            && crate::video::toggle(&path)
        {
            this.widget.videos.add(path);
            this.ctx.request_anim_frame();
        }
    }
}
