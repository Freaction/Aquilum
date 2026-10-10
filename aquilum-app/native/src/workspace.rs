//! Связь окна с ядром `aquilum-core`: каталог данных, база знаний и события ядра.
//!
//! События приходят из потоков ядра: `EventSink` кладёт их в канал и будит цикл winit,
//! хост вычитывает канал целиком в `proxy_wake_up` (пробуждения склеиваются).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

use aquilum_core::{Core, CoreEvent, EventSink};

const DEV_APP_ID: &str = "com.dmitriy.aquilum-app.native-dev";

pub const APP_ID: &str = if cfg!(debug_assertions) { DEV_APP_ID } else { PRODUCTION_APP_ID };

pub struct Workspace {
    pub core: Arc<Core>,
    pub data_dir: PathBuf,
    /// Корень базы знаний; `None` — база не выбрана.
    pub root: Option<PathBuf>,
    pub wake: Arc<dyn Fn() + Send + Sync>,
    events: Receiver<CoreEvent>,
}

impl Workspace {
    /// Открывает ядро в `data_dir` (по умолчанию — каталог нативной сборки) и выбирает базу:
    /// `vault`, иначе последнюю открытую в этом каталоге данных.
    pub fn open(data_dir: Option<&Path>, vault: Option<&Path>, wake: impl Fn() + Send + Sync + 'static) -> Result<Self, String> {
        let data_dir = data_dir_or_default(data_dir)?;
        std::fs::create_dir_all(&data_dir).map_err(|e| format!("{}: {e}", data_dir.display()))?;
        if data_dir.ends_with(PRODUCTION_APP_ID) {
            aquilum_core::migration::migrate_legacy_data(&data_dir);
        } else {
            adopt_production_settings(&data_dir);
        }

        let (sender, events) = channel();
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(wake);
        let waker = Arc::clone(&wake);
        let sink: Arc<dyn EventSink> = Arc::new(move |event: CoreEvent| {
            // Окно закрыто — событие некому показывать.
            if sender.send(event).is_ok() {
                waker();
            }
        });
        let core = Core::open(&data_dir, sink);

        let root = match vault {
            Some(vault) => Some(normalize(vault)),
            None => core.ui_state.list_workspaces().ok().and_then(|known| known.into_iter().next()).map(|w| PathBuf::from(w.path)),
        };
        if let Some(root) = &root {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or_default();
            if let Err(e) = core.ui_state.resolve_workspace(&root.to_string_lossy(), now_ms) {
                eprintln!("база {} не запомнена: {e:?}", root.display());
            }
        }
        Ok(Workspace { core, data_dir, root, wake, events })
    }

    /// Все накопившиеся события ядра.
    pub fn drain_events(&self) -> Vec<CoreEvent> {
        self.events.try_iter().collect()
    }
}

pub fn data_dir_or_default(data_dir: Option<&Path>) -> Result<PathBuf, String> {
    match data_dir {
        Some(dir) => Ok(dir.to_path_buf()),
        None => Ok(platform_data_dir().ok_or("не найден каталог данных пользователя")?.join(APP_ID)),
    }
}

pub fn normalize(path: &Path) -> PathBuf {
    let Ok(canonical) = std::fs::canonicalize(path) else { return path.to_path_buf() };
    let text = canonical.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    match text.strip_prefix(r"\\?\") {
        Some(rest) => PathBuf::from(rest),
        None => canonical.clone(),
    }
}

/// При первом запуске нативной сборки берёт настройки рабочей версии (язык, тема, шрифты),
/// чтобы интерфейс выглядел как привычный Aquilum. Файл рабочей версии только читается.
fn adopt_production_settings(data_dir: &Path) {
    let target = data_dir.join("settings.json");
    if target.exists() {
        return;
    }
    let Some(source) = platform_data_dir().map(|dir| dir.join(PRODUCTION_APP_ID).join("settings.json")) else { return };
    if source.exists()
        && let Err(e) = std::fs::copy(&source, &target)
    {
        eprintln!("настройки рабочей версии не скопированы: {e}");
    }
}

/// Идентификатор рабочей версии на Tauri: там лежат её данные.
const PRODUCTION_APP_ID: &str = "com.dmitriy.aquilum-app";

/// Тот же корень, что `app_data_dir` у Tauri: %APPDATA%, ~/Library/Application Support,
/// $XDG_DATA_HOME или ~/.local/share.
fn platform_data_dir() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
    }
}
