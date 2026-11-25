use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use winit::window::Theme;

pub(super) struct SystemTheme {
    current: Arc<AtomicU8>,
    taken: Option<bool>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl SystemTheme {
    pub(super) fn new(wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        let theme = SystemTheme { current: Arc::default(), taken: None, wake };
        #[cfg(target_os = "linux")]
        portal(Arc::clone(&theme.current), Arc::clone(&theme.wake));
        theme
    }

    pub(super) fn window(&self, theme: Option<Theme>) {
        if !cfg!(target_os = "linux")
            && let Some(theme) = theme
        {
            publish(&self.current, theme == Theme::Dark, &*self.wake);
        }
    }

    fn current(&self) -> Option<bool> {
        match self.current.load(Ordering::Acquire) {
            1 => Some(false),
            2 => Some(true),
            _ => None,
        }
    }

    pub(super) fn take(&mut self) -> Option<bool> {
        let current = self.current();
        if current == self.taken {
            return None;
        }
        self.taken = current;
        current
    }
}

fn publish(current: &AtomicU8, dark: bool, wake: &dyn Fn()) {
    let value = if dark { 2 } else { 1 };
    if current.swap(value, Ordering::AcqRel) != value {
        wake();
    }
}

#[cfg(target_os = "linux")]
fn portal(current: Arc<AtomicU8>, wake: Arc<dyn Fn() + Send + Sync>) {
    use ashpd::desktop::settings::{ColorScheme, Settings};
    use futures_lite::{StreamExt, future::block_on};
    let _ = aquilum_core::threads::spawn("desktop appearance", move || {
        let result = block_on(async {
            let settings = Settings::new().await?;
            // Subscribe before the initial read so startup cannot miss a change.
            let changes = settings.receive_color_scheme_changed().await?;
            let mut changes = std::pin::pin!(changes);
            publish(&current, settings.color_scheme().await.unwrap_or_default() == ColorScheme::PreferDark, &*wake);
            while let Some(scheme) = changes.next().await {
                publish(&current, scheme == ColorScheme::PreferDark, &*wake);
            }
            Ok::<_, ashpd::Error>(())
        });
        if let Err(error) = result {
            super::report(&format!("desktop appearance: {error}"));
        }
        publish(&current, false, &*wake);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_are_coalesced_and_no_preference_is_light() {
        let wakes = Arc::new(AtomicU8::new(0));
        let counter = Arc::clone(&wakes);
        let wake = move || {
            counter.fetch_add(1, Ordering::Relaxed);
        };
        let mut theme = SystemTheme { current: Arc::default(), taken: None, wake: Arc::new(wake) };
        let set = |theme: &SystemTheme, dark| publish(&theme.current, dark, &*theme.wake);
        assert_eq!(theme.take(), None);
        set(&theme, true);
        assert_eq!(theme.current(), Some(true));
        assert_eq!(theme.take(), Some(true));
        set(&theme, true);
        assert_eq!(theme.take(), None);
        assert_eq!(wakes.load(Ordering::Relaxed), 1);
        set(&theme, false);
        set(&theme, true);
        assert_eq!(theme.take(), None, "intermediate changes must not restyle the UI");
        set(&theme, false);
        assert_eq!(theme.take(), Some(false));
        theme.window(Some(Theme::Dark));
        assert_eq!(theme.take(), (!cfg!(target_os = "linux")).then_some(true), "на Linux тему сообщает портал, а не окно");
    }
}
