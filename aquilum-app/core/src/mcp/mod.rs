pub mod active;
pub mod bridge;
mod http;
mod protocol;
mod server;
mod stdio;
mod stdio_server;
mod tools;
mod vault;

pub use stdio::run_stdio_bridge;
pub use stdio_server::run_stdio_server;

use crate::settings::models::McpSettings;
use serde::Serialize;
use server::RunningServer;
use std::sync::{Arc, Mutex};
use crate::app_core::Core;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    pub running: bool,
    pub port: u16,
    pub executable: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Default)]
struct ServerState {
    running: Option<RunningServer>,
    port: u16,
    token: String,
    error: Option<String>,
}

impl ServerState {
    fn status(&self) -> McpStatus {
        McpStatus {
            running: self.running.is_some(),
            port: self.port,
            executable: std::env::current_exe()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default(),
            error: self.error.clone(),
        }
    }
}

pub struct McpServer {
    state: Mutex<ServerState>,
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(ServerState::default()),
        }
    }

    pub fn apply(&self, core: &Arc<Core>, settings: &McpSettings) -> McpStatus {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        let token = settings.token.trim().to_owned();
        let wanted = settings.enabled && !token.is_empty();
        let reconfigured = state.port != settings.port || state.token != token;

        if state.running.is_some() && (!wanted || reconfigured) {
            if let Some(running) = state.running.take() {
                running.stop();
            }
        }

        state.port = settings.port;
        state.token = token.clone();
        state.error = None;

        if !wanted {
            if settings.enabled {
                state.error = Some("Не задан токен доступа".to_owned());
            }
            return state.status();
        }

        if state.running.is_none() {
            let weak = Arc::downgrade(core);
            let caller: server::SharedToolCaller = Arc::new(move |name: &str, arguments: &serde_json::Value| {
                let core = weak.upgrade().ok_or_else(|| "Приложение завершается".to_owned())?;
                tools::call(&core, name, arguments)
            });
            match RunningServer::start(settings.port, token, caller) {
                Ok(running) => {
                    state.port = running.port();
                    state.running = Some(running);
                }
                Err(error) => {
                    state.error = Some(format!("Порт {} недоступен: {error}", settings.port))
                }
            }
        }
        state.status()
    }

    pub fn status(&self) -> McpStatus {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .status()
    }

    pub fn shutdown(&self) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(running) = state.running.take() {
            running.stop();
        }
    }
}
