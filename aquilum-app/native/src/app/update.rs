use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Point;

use super::{App, Popup};
use crate::i18n::{t, t_with};
use crate::ui::update::{UpdateBar, UpdateSplash, update_bar};
use crate::ui::widgets::{Pressed, Slot};
use crate::updater::Status;

#[derive(Default)]
pub(super) struct UpdateView {
    shown: Option<Status>,
    splash: Option<WidgetId>,
    pub(super) bar: Option<(String, UpdateBar)>,
}

impl App {
    pub(super) fn on_update(&mut self, root: &mut RenderRoot) {
        let status = self.updater.status();
        if self.update.shown.as_ref() != Some(&status) {
            self.update.shown = Some(status.clone());
            let splash = match &status {
                Status::Downloading { done, total, .. } => Some((false, total.filter(|t| *t > 0).map(|t| *done as f64 * 100.0 / t as f64))),
                Status::Ready(_) | Status::Installing => Some((true, None)),
                _ => None,
            };
            match (splash, self.update.splash) {
                (Some((installing, percent)), Some(layer)) => {
                    root.edit_widget(layer, |mut w| UpdateSplash::set(&mut w.downcast(), installing, percent));
                }
                (Some((installing, percent)), None) => {
                    let widget = UpdateSplash::new(installing, percent);
                    self.update.splash = Some(widget.id());
                    root.add_layer(widget, Point::ORIGIN);
                }
                (None, Some(layer)) => {
                    root.remove_layer(layer);
                    self.update.splash = None;
                }
                (None, None) => {}
            }
            if let Status::ManualRequired { version, url } = &status {
                self.show_manual_update_dialog(root, version, url);
            }
            self.show_update_bar(root);
            self.refresh_section(root, crate::ui::settings::schema::Section::System);
        }
        if let Some((version, url, payload)) = self.updater.take_ready() {
            let _ = root.redraw();
            let mut closed = false;
            self.updater.record_attempt(&version, &url);
            match crate::updater::apply(&payload, || {
                self.shutdown();
                closed = true;
            }) {
                Ok(()) => self.updater.handed_off(),
                Err(error) => {
                    report!("обновление не установлено: {error}");
                    if closed {
                        std::process::exit(1);
                    }
                    self.updater.fail(error);
                }
            }
        }
    }

    pub(super) fn show_update_bar(&mut self, root: &mut RenderRoot) {
        let offer = match self.updater.status() {
            Status::Available(version) if !self.interface.update_dismissed(&version) => Some(version),
            _ => None,
        };
        let Some(chrome) = &self.chrome else { return };
        if offer.as_ref() == self.update.bar.as_ref().map(|(version, _)| version) {
            return;
        }
        self.update.bar = match offer {
            Some(version) => {
                let (bar, ids) = update_bar(&version);
                chrome.notice.edit(root, |mut slot| Slot::set_child(&mut slot, bar.erased()));
                Some((version, ids))
            }
            None => {
                chrome.notice.edit(root, |mut slot| Slot::clear(&mut slot));
                None
            }
        };
    }

    pub(super) fn rebuild_update_bar(&mut self, root: &mut RenderRoot) {
        self.update.bar = None;
        self.show_update_bar(root);
    }

    pub(super) fn on_update_bar(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> bool {
        let Some((version, bar)) = &self.update.bar else { return false };
        if action.downcast_ref::<Pressed>().is_none() || ![bar.install, bar.close].contains(&id) {
            return false;
        }
        if id == bar.install {
            self.updater.install();
        } else {
            let version = version.clone();
            self.interface.dismiss_update(&version);
            self.show_update_bar(root);
        }
        true
    }

    pub(in crate::app) fn show_manual_update_dialog(&mut self, root: &mut RenderRoot, version: &str, url: &str) {
        let (layer, dialog) = crate::ui::dialog::ConfirmDialog::new(&t("update.manualTitle"), &t_with("update.manualDesc", &[("version", version)]), &t("common.close"), &t("update.download"));
        root.add_layer(layer, Point::ORIGIN);
        root.focus_on(Some(dialog.layer));
        self.popup = Some(Popup::ManualUpdate(dialog, url.to_owned()));
    }
}
