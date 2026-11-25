use serde_json::Value;
use std::io::{BufRead, Write};

struct Endpoint {
    port: u16,
    token: String,
}

/// Relays MCP messages between stdin/stdout and the running app. `app_identifier` names the app
/// data folder whose `settings.json` holds the MCP port and token.
pub fn run_stdio_bridge(app_identifier: &str) -> i32 {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let message = line.trim();
        if message.is_empty() {
            continue;
        }
        if let Some(response) = forward(message, app_identifier) {
            if writeln!(stdout, "{response}").is_err() || stdout.flush().is_err() {
                return 1;
            }
        }
    }
    0
}

fn forward(message: &str, app_identifier: &str) -> Option<String> {
    let Some(endpoint) = endpoint(app_identifier) else {
        return transport_error(message, "не найдены настройки MCP");
    };
    let response = ureq::post(&format!("http://127.0.0.1:{}/mcp", endpoint.port))
        .set("Authorization", &format!("Bearer {}", endpoint.token))
        .set("Content-Type", "application/json")
        .send_string(message);

    match response {
        Ok(response) if response.status() == 202 => None,
        Ok(response) => response
            .into_string()
            .ok()
            .filter(|body| !body.trim().is_empty()),
        Err(ureq::Error::Status(status, _)) => {
            transport_error(message, &format!("сервер ответил {status}"))
        }
        Err(error) => transport_error(
            message,
            &format!("приложение Aquilum не запущено или MCP выключен ({error})"),
        ),
    }
}

fn transport_error(message: &str, reason: &str) -> Option<String> {
    let id = serde_json::from_str::<Value>(message)
        .ok()
        .and_then(|value| value.get("id").cloned())
        .filter(|id| !id.is_null())?;
    Some(
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32000, "message": format!("Aquilum MCP: {reason}") },
        })
        .to_string(),
    )
}

fn endpoint(app_identifier: &str) -> Option<Endpoint> {
    let from_env = (
        std::env::var("AQUILUM_MCP_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok()),
        std::env::var("AQUILUM_MCP_TOKEN").ok(),
    );
    if let (Some(port), Some(token)) = from_env {
        return Some(Endpoint { port, token });
    }

    let settings = std::fs::read_to_string(crate::paths::data_dir(app_identifier)?.join("settings.json")).ok()?;
    let mcp = serde_json::from_str::<Value>(&settings).ok()?.get("mcp")?.clone();
    Some(Endpoint {
        port: u16::try_from(mcp.get("port").and_then(Value::as_u64)?).ok()?,
        token: mcp.get("token").and_then(Value::as_str)?.to_owned(),
    })
}
