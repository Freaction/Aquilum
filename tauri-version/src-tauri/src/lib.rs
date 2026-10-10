#[cfg(all(desktop, not(debug_assertions)))]
mod autostart;
mod blocking;
mod export;
mod window_state;
mod updater;

use aquilum_core::{app_core, migration};

// Each core module is re-exported under its old path, with the Tauri commands for it next to it, so
// `crate::search::...` keeps working in the shell and commands still sit beside their module.
mod documents {
    pub use aquilum_core::documents::*;
    pub mod commands;
}
mod files {
    pub use aquilum_core::files::*;
    pub mod commands;
}
mod history {
    pub use aquilum_core::history::*;
    pub mod commands;
}
mod mcp {
    pub use aquilum_core::mcp::*;
    pub mod commands;
}
mod link_title {
    pub use aquilum_core::link_title::*;
    pub mod commands;
}
mod search {
    pub use aquilum_core::search::*;
    pub mod commands;
    pub mod analysis {
        pub use aquilum_core::search::analysis::*;
        pub mod commands;
    }
    pub mod graph {
        pub mod commands;
    }
}
mod plugins {
    pub use aquilum_core::plugins::*;
    pub mod commands;
}
mod settings {
    pub use aquilum_core::settings::*;
    pub mod commands;
}
mod ui_state {
    pub use aquilum_core::ui_state::*;
    pub mod commands;
}
mod wikixiv {
    pub use aquilum_core::wikixiv::*;
    pub mod commands;
}

use std::sync::Arc;
use tauri::window::Color;
use tauri::{Emitter, Manager, Theme, WindowEvent};

pub fn run_mcp_stdio_bridge() -> i32 {
    mcp::run_stdio_bridge(env!("AQUILUM_APP_IDENTIFIER"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init());

    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_autostart::init(
        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
        None,
    ));

    #[cfg(feature = "updater")]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());

    #[cfg(debug_assertions)]
    let builder = builder.on_page_load(|webview, payload| {
        if payload.event() == tauri::webview::PageLoadEvent::Finished && webview.label() == "main" {
            if let Err(error) = webview.window().show() {
                eprintln!("[aquilum] окно не показано: {error}");
            }
        }
    });

    builder
        .on_menu_event(|app, event| {
            if event.id().as_ref() == "close-tab" {
                let _ = app.emit("close-tab", ());
            }
        })
        .setup(|app| {
            replace_close_window_item(app.handle())?;
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            migration::migrate_legacy_data(&app_data_dir);

            let handle = app.handle().clone();
            let events: Arc<dyn app_core::EventSink> = Arc::new(move |event: app_core::CoreEvent| {
                if let Err(error) = handle.emit(event.name(), &event) {
                    eprintln!("[aquilum] событие {} не отправлено: {error}", event.name());
                }
            });
            let core = app_core::Core::open(&app_data_dir, events);
            core.apply_mcp_settings();
            app.manage(core);

            let window_state_manager = window_state::WindowStateManager::new(&app_data_dir);
            if let Some(window) = app.get_webview_window("main") {
                #[cfg(target_os = "windows")]
                allow_pinch_gestures(&window);
                paint_canvas(&window, &app.state::<Arc<app_core::Core>>().settings.get_config().ui.theme);
                window_state_manager.restore(&window);
                window_state_manager.initialize(&window);
            }
            app.manage(window_state_manager);


            #[cfg(all(desktop, not(debug_assertions)))]
            autostart::repoint_to_current_exe(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            plugins::commands::plugin_vault_data_get,
            plugins::commands::plugin_vault_data_set,
            settings::commands::get_settings,
            settings::commands::update_settings,
            updater::check_for_update,
            updater::install_update,
            files::commands::read_directory,
            files::commands::existing_files,
            files::commands::resolve_attachments,
            files::commands::read_file_snapshot,
            files::commands::read_file_stat,
            files::commands::write_file_atomic,
            files::commands::create_file,
            files::commands::create_binary_file,
            files::commands::ensure_directory,
            files::commands::trash_file,
            files::commands::get_trash_state,
            files::commands::list_trash,
            files::commands::restore_deletion,
            files::commands::open_trash,
            files::commands::cleanup_trash,
            history::commands::note_history,
            history::commands::read_note_version,
            history::commands::name_note_version,
            history::commands::cleanup_history,
            files::commands::copy_file,
            files::commands::rename_file,
            documents::commands::document_open,
            documents::commands::document_push,
            documents::commands::document_pull,
            documents::commands::document_replace_text,
            documents::commands::document_read,
            documents::commands::document_import_legacy,
            documents::commands::document_revert,
            documents::commands::document_resolve_positions,
            documents::commands::document_release,
            documents::commands::document_flush_all,
            documents::commands::document_reconcile_all,
            search::commands::prepare_search_index,
            search::commands::search_knowledge_base,
            search::commands::get_search_index_status,
            search::commands::get_backlinks,
            search::commands::get_outgoing_links,
            search::commands::resolve_wiki_links,
            search::commands::suggest_notes,
            search::commands::get_note_fields,
            search::commands::run_dataview_query,
            search::analysis::commands::analyze_document,
            search::graph::commands::get_graph_snapshot,
            search::graph::commands::get_graph_paths,
            wikixiv::commands::wikixiv_search,
            link_title::commands::fetch_page_title,
            ui_state::commands::list_ui_workspaces,
            ui_state::commands::forget_ui_workspace,
            ui_state::commands::set_ui_workspace_home_page,
            ui_state::commands::resolve_ui_workspace,
            ui_state::commands::resolve_ui_document,
            ui_state::commands::open_ui_session,
            ui_state::commands::save_ui_state_batch,
            ui_state::commands::load_ui_document_view,
            ui_state::commands::rename_ui_document,
            ui_state::commands::mark_ui_document_missing,
            ui_state::commands::reset_ui_state,
            ui_state::commands::cleanup_ui_state,
            ui_state::commands::load_ui_reader_state,
            ui_state::commands::save_ui_reader_state,
            mcp::commands::get_mcp_status,
            mcp::commands::apply_mcp_settings,
            mcp::commands::set_active_note,
            export::commands::export_pdf,
            export::commands::pdf_export_is_native,
            plugins::commands::calendar_month,
            plugins::commands::calendar_open,
            // проводник.
            plugins::commands::explorer_overview,
            // Git-синхронизация.
            plugins::commands::git_status,
            plugins::commands::git_sync,
            plugins::commands::git_pull,
            plugins::commands::obsidian_detect,
            plugins::commands::obsidian_import,
            plugins::commands::table_format,
            plugins::commands::table_sort
        ])
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            let state = window.state::<window_state::WindowStateManager>();
            match event {
                WindowEvent::Resized(_) => state.observe(window),
                WindowEvent::CloseRequested { .. } => {
                    state.capture_and_persist(window);
                    window.state::<Arc<app_core::Core>>().shutdown();
                }
                _ => {}
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            tauri::RunEvent::Exit => {
                if let Some(window) = app.get_webview_window("main") {
                    app.state::<window_state::WindowStateManager>().capture_and_persist(&window.as_ref().window());
                }
                app.state::<Arc<app_core::Core>>().shutdown();
            }
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { has_visible_windows: false, .. } => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            _ => {}
        });
}

const DARK_CANVAS: Color = Color(0x09, 0x0b, 0x11, 0xff);
const LIGHT_CANVAS: Color = Color(0xff, 0xff, 0xff, 0xff);

fn paint_canvas(window: &tauri::WebviewWindow, theme: &str) {
    let dark = match theme {
        "dark" => true,
        "light" => false,
        _ => matches!(window.theme(), Ok(Theme::Dark)),
    };
    if let Err(error) = window.set_background_color(Some(if dark { DARK_CANVAS } else { LIGHT_CANVAS })) {
        eprintln!("[aquilum] фон окна не задан: {error}");
    }
}

#[cfg(target_os = "windows")]
fn allow_pinch_gestures(window: &tauri::WebviewWindow) {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings5;
    use windows::core::Interface;
    let _ = window.with_webview(|webview| {
        let enabled = unsafe {
            webview
                .controller()
                .CoreWebView2()
                .and_then(|core| core.Settings())
                .and_then(|settings| settings.cast::<ICoreWebView2Settings5>())
                .and_then(|settings| settings.SetIsPinchZoomEnabled(true))
        };
        if let Err(error) = enabled {
            eprintln!("[aquilum:input] pinch gestures stay disabled: {error}");
        }
    });
}

/// В меню macOS по умолчанию Cmd+W закрывает всё окно; здесь он закрывает активную вкладку.
fn replace_close_window_item(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, MenuItemKind};
    let menu = Menu::default(app)?;
    for item in menu.items()? {
        let MenuItemKind::Submenu(submenu) = item else { continue };
        for entry in submenu.items()? {
            let is_close = entry.as_predefined_menuitem()
                .and_then(|p| p.text().ok())
                .as_deref() == Some("Close Window");
            if is_close {
                submenu.remove(&entry)?;
                let close_tab = MenuItem::with_id(app, "close-tab", "Закрыть вкладку", true, Some("CmdOrCtrl+W"))?;
                submenu.append(&close_tab)?;
            }
        }
    }
    app.set_menu(menu)?;
    Ok(())
}
