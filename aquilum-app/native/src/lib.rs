macro_rules! report {
    ($($arg:tt)*) => {
        $crate::host::report(&format!($($arg)*))
    };
}

mod a11y;
#[cfg(target_os = "android")]
mod android;
mod args;
mod autostart;
mod app;
mod clipboard;
mod cover;
mod poster;
mod graph;
mod reader;
mod video;
mod dates;
mod disk;
mod file_ops;
mod fling;
mod frontmatter;
mod i18n;
mod images;
mod instance;
mod interface;
mod links;
mod history;
#[cfg(any(target_os = "android", test))]
mod ime_sync;
mod host;
mod metrics;
mod native_dialog;
#[cfg(feature = "memory-probe")]
mod memory_probe;

#[cfg(feature = "memory-probe")]
#[global_allocator]
static ALLOCATOR: memory_probe::Counting = memory_probe::Counting;
mod navigation;
mod note;
mod output;
mod paths;
mod pdf;
mod rank;
mod rope;
mod stats;
mod render;
#[cfg(not(feature = "installed"))]
mod replay;
mod session;
mod system;
mod tabs;
#[cfg(target_os = "windows")]
mod touchpad;
#[cfg(not(target_os = "windows"))]
mod touchpad {
    pub struct Touchpad;

    impl Touchpad {
        pub fn attach(_: &dyn winit::window::Window) -> Option<Self> {
            None
        }

        pub fn active(&self) -> bool {
            false
        }

        pub fn update(&self, _: std::time::Instant, _: f64) -> (f32, f32, f32) {
            (0.0, 0.0, 1.0)
        }
    }
}
#[cfg(test)]
mod test_dir;
mod vault_files;
mod vsync;
mod ui;
mod updater;
mod window_state;
mod workspace;

#[cfg(target_os = "android")]
pub use android::{documents::deliver as android_document, ime::received as android_ime_state, link as android_link, main as android_main};
#[cfg(target_os = "android")]
pub use ime_sync::{ImeState, Message as ImeMessage};
#[cfg(target_os = "android")]
pub use winit::platform::android::activity::AndroidApp;
pub use args::Args;

const WORKERS: usize = 4;

#[cfg(not(target_os = "android"))]
pub fn desktop_main() {
    #[cfg(target_os = "windows")]
    unsafe {
        #[link(name = "winmm")]
        unsafe extern "system" {
            fn timeBeginPeriod(uPeriod: u32) -> u32;
        }
        let _ = timeBeginPeriod(1);
    }
    if std::env::args().any(|argument| argument == "--mcp-stdio") {
        std::process::exit(aquilum_core::mcp::run_stdio_bridge(workspace::APP_ID));
    }
    let _ = rayon::ThreadPoolBuilder::new().num_threads(WORKERS).build_global();
    let result = args::parse().and_then(|args| host::run(args, winit::event_loop::EventLoop::new().map_err(|e| format!("цикл событий: {e}"))?));
    if let Err(err) = &result {
        report!("{err}");
        native_dialog::alert(err);
    }
    std::process::exit(i32::from(result.is_err()));
}
