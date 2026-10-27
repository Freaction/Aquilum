use std::path::PathBuf;
use std::sync::OnceLock;

use winit::dpi::PhysicalInsets;
use winit::event_loop::EventLoop;
use winit::platform::android::EventLoopBuilderExtAndroid;
use winit::platform::android::activity::AndroidApp;

pub mod clipboard;
pub mod ime;
mod java;
mod links;

pub use links::{deliver as link, login_links};

static VAULTS: OnceLock<PathBuf> = OnceLock::new();

pub fn vaults_home() -> Option<PathBuf> {
    VAULTS.get().cloned()
}

pub fn main(app: AndroidApp) {
    java::init(&app);
    let _ = rayon::ThreadPoolBuilder::new().num_threads(crate::WORKERS).build_global();
    if let Err(err) = run(app) {
        report!("{err}");
    }
    std::process::exit(0);
}

fn run(app: AndroidApp) -> Result<(), String> {
    let data_dir = app.internal_data_path().ok_or("нет каталога данных приложения")?.join("aquilum");
    let first_run = !data_dir.exists();
    let mut args = crate::args::defaults();
    if let Some(home) = app.external_data_path() {
        let _ = VAULTS.set(home);
    }
    if first_run && let Some(vault) = vaults_home().map(|dir| dir.join("Aquilum")) {
        std::fs::create_dir_all(&vault).map_err(|e| format!("{}: {e}", vault.display()))?;
        args.vault = Some(vault);
    }
    args.data_dir = Some(data_dir);
    let event_loop = EventLoop::builder().with_android_app(app).build().map_err(|e| format!("цикл событий: {e}"))?;
    crate::host::run(args, event_loop)
}

pub fn open(target: &str) {
    if !java::send_text(jni::jni_str!("openUri"), target) {
        report!("{target} не открыт");
    }
}

pub fn safe_area() -> PhysicalInsets<u32> {
    let packed = java::read_long(jni::jni_str!("safeArea")).unwrap_or(0) as u64;
    let side = |shift: u32| ((packed >> shift) & 0xffff) as u32;
    PhysicalInsets::new(side(16), side(0), side(48), side(32))
}

pub fn leave() {
    java::run(jni::jni_str!("leave"));
}

pub fn install_update(payload: &[u8]) -> Result<(), String> {
    let path = java::read_text(jni::jni_str!("updateFile")).ok_or("нет файла для обновления")?;
    std::fs::write(&path, payload).map_err(|e| format!("{path}: {e}"))?;
    java::send_text(jni::jni_str!("installApk"), &path).then_some(()).ok_or_else(|| "установщик не открылся".into())
}

pub fn set_dark_theme(dark: bool) {
    java::send_bool(jni::jni_str!("setDarkTheme"), dark);
}

pub fn show_sync(sync: &aquilum_core::sync::SyncService) {
    let count = |n: usize| i32::try_from(n).unwrap_or(i32::MAX);
    match sync.status() {
        aquilum_core::sync::SyncStatus::Running(progress) => {
            let (stage, done, total) = crate::ui::settings::schema::sync::stage(&progress);
            java::send_progress(jni::jni_str!("syncProgress"), &stage, count(done), count(total))
        }
        _ => java::run(jni::jni_str!("syncFinished")),
    };
}
