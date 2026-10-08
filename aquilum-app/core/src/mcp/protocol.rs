use super::tools;
use serde::Deserialize;
use serde_json::{json, Value};

pub type ToolCaller<'a> = &'a dyn Fn(&str, &Value) -> Result<Value, String>;

/// Handles a single JSON-RPC message (already parsed from JSON).
/// Returns `None` for notifications (no response needed), `Some(response)` for requests.
pub fn handle_message_for_stdio<F: Fn(&str, &Value) -> Result<Value, String> + Send + Sync>(
    message: &Value,
    call: &F,
) -> Option<Value> {
    match message {
        Value::Array(items) => {
            // Batch: collect all responses, skip notifications
            let responses = items
                .iter()
                .filter_map(|item| handle_single_stdio(item.clone(), call))
                .collect::<Vec<_>>();
            (!responses.is_empty()).then(|| Value::Array(responses))
        }
        single => handle_single_stdio(single.clone(), call),
    }
}

fn handle_single_stdio(message: Value, call: ToolCaller<'_>) -> Option<Value> {
    let request = match serde_json::from_value::<RpcRequest>(message) {
        Ok(request) => request,
        Err(error) => return Some(failure(Value::Null, PARSE_ERROR, &error.to_string())),
    };
    // Notifications (no id or null id) produce no response
    let id = request.id.filter(|id| !id.is_null())?;

    Some(match dispatch(&request.method, request.params, call) {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => failure(id, code, &message),
    })
}

const PROTOCOL_VERSION: &str = "2025-06-18";
const SERVER_NAME: &str = "aquilum";

const METHOD_NOT_FOUND: i32 = -32601;
const INVALID_PARAMS: i32 = -32602;
const PARSE_ERROR: i32 = -32700;

#[derive(Deserialize)]
struct RpcRequest {
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

#[derive(Deserialize)]
struct ToolCall {
    name: String,
    #[serde(default)]
    arguments: Value,
}

pub fn handle_message(body: &str, call: ToolCaller<'_>) -> Option<String> {
    let message = match serde_json::from_str::<Value>(body) {
        Ok(value) => value,
        Err(error) => {
            return Some(failure(Value::Null, PARSE_ERROR, &error.to_string()).to_string())
        }
    };

    match message {
        Value::Array(items) => {
            let responses = items
                .into_iter()
                .filter_map(|item| handle_single(item, call))
                .collect::<Vec<_>>();
            (!responses.is_empty()).then(|| Value::Array(responses).to_string())
        }
        single => handle_single(single, call).map(|value| value.to_string()),
    }
}

fn handle_single(message: Value, call: ToolCaller<'_>) -> Option<Value> {
    let request = match serde_json::from_value::<RpcRequest>(message) {
        Ok(request) => request,
        Err(error) => return Some(failure(Value::Null, PARSE_ERROR, &error.to_string())),
    };
    let id = request.id.filter(|id| !id.is_null())?;

    Some(match dispatch(&request.method, request.params, call) {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => failure(id, code, &message),
    })
}

fn dispatch(
    method: &str,
    params: Option<Value>,
    call: ToolCaller<'_>,
) -> Result<Value, (i32, String)> {
    match method {
        "initialize" => {
            let version = params
                .as_ref()
                .and_then(|value| value.get("protocolVersion"))
                .and_then(Value::as_str)
                .unwrap_or(PROTOCOL_VERSION)
                .to_owned();
            Ok(json!({
                "protocolVersion": version,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION") },
                "instructions": tools::INSTRUCTIONS,
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools::definitions() })),
        "tools/call" => {
            let request = serde_json::from_value::<ToolCall>(params.unwrap_or(Value::Null))
                .map_err(|error| (INVALID_PARAMS, error.to_string()))?;
            Ok(tool_result(call(&request.name, &request.arguments)))
        }
        _ => Err((METHOD_NOT_FOUND, format!("Неизвестный метод: {method}"))),
    }
}

fn tool_result(outcome: Result<Value, String>) -> Value {
    let (text, is_error) = match outcome {
        Ok(value) => (
            serde_json::to_string(&value).unwrap_or_else(|error| error.to_string()),
            false,
        ),
        Err(message) => (message, true),
    };
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error,
    })
}

fn failure(id: Value, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_tools(_name: &str, _arguments: &Value) -> Result<Value, String> {
        Err("не должно вызываться".to_owned())
    }

    fn handle(body: &str) -> Option<Value> {
        handle_message(body, &no_tools).map(|value| serde_json::from_str(&value).unwrap())
    }

    #[test]
    fn answers_initialize_with_protocol_version_and_tools_capability() {
        let response = handle(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26"}}"#,
        )
        .unwrap();
        assert_eq!(response["result"]["protocolVersion"], "2025-03-26");
        assert!(response["result"]["capabilities"]["tools"].is_object());
        assert_eq!(response["result"]["serverInfo"]["name"], SERVER_NAME);
    }

    #[test]
    fn keeps_silent_on_notifications() {
        assert!(handle(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
        assert!(handle(r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#).is_none());
    }

    #[test]
    fn reports_unknown_method_as_rpc_error() {
        let response = handle(r#"{"jsonrpc":"2.0","id":7,"method":"resources/list"}"#).unwrap();
        assert_eq!(response["error"]["code"], -32601);
        assert_eq!(response["id"], 7);
    }

    #[test]
    fn reports_broken_json_without_panicking() {
        let response = handle("{ не json").unwrap();
        assert_eq!(response["error"]["code"], -32700);
    }

    #[test]
    fn answers_every_request_of_a_batch_and_skips_notifications() {
        let response = handle(
            r#"[{"jsonrpc":"2.0","id":1,"method":"ping"},
                {"jsonrpc":"2.0","method":"notifications/initialized"},
                {"jsonrpc":"2.0","id":2,"method":"tools/list"}]"#,
        )
        .unwrap();
        let items = response.as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert!(items[1]["result"]["tools"].as_array().unwrap().len() > 10);
    }

    #[test]
    fn returns_tool_failure_as_result_with_error_flag() {
        let failing = |_name: &str, _arguments: &Value| Err("Заметка не найдена".to_owned());
        let raw = handle_message(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"read_note","arguments":{}}}"#,
            &failing,
        )
        .unwrap();
        let response: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(response["result"]["isError"], true);
        assert_eq!(response["result"]["content"][0]["text"], "Заметка не найдена");
        assert!(response["error"].is_null());
    }

    #[test]
    fn every_tool_declares_a_json_schema() {
        for tool in tools::definitions() {
            assert!(tool["name"].as_str().is_some_and(|name| !name.is_empty()));
            assert!(tool["description"].as_str().is_some_and(|value| !value.is_empty()));
            assert_eq!(tool["inputSchema"]["type"], "object");
        }
    }
}
