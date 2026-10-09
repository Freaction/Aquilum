use serde_json::{json, Value};
use std::io::{self, BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::app_core::{Core, CoreEvent, EventSink};

/// No-op event sink for the stdio server mode — MCP tools that trigger UI navigation
/// will simply not produce a visible effect; file operations still work normally.
struct NoOpSink;

impl EventSink for NoOpSink {
    fn emit(&self, _event: CoreEvent) {}
}

/// Reads one complete JSON message from stdin.
/// MCP stdio transport sends one JSON object per line (newline-delimited).
fn read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => Ok(None),          // EOF
        Ok(_) => {
            // Strip the trailing newline
            let _ = line.pop();
            if line.ends_with('\r') {
                let _ = line.pop();
            }
            if line.is_empty() {
                // Empty line — heartbeat, skip and read next
                read_message(reader)
            } else {
                Ok(Some(line))
            }
        }
        Err(e) if e.kind() == io::ErrorKind::Interrupted => read_message(reader),
        Err(e) => Err(e),
    }
}

/// Reads a single JSON value from a line, returning an error response for malformed JSON.
fn parse_message(line: &str) -> Result<Value, Value> {
    serde_json::from_str::<Value>(&line).map_err(|error| {
        json!({
            "jsonrpc": "2.0",
            "id": null,
            "error": { "code": -32700, "message": format!("Parse error: {error}") }
        })
    })
}

/// Writes a JSON response to stdout, followed by a newline.
fn write_message<W: Write>(writer: &mut BufWriter<W>, message: &Value) -> io::Result<()> {
    writeln!(writer, "{}", message)?;
    writer.flush()
}

/// The main stdio server loop: reads JSON-RPC messages from stdin, processes them,
/// and writes responses to stdout. Returns the exit code (0 = success, 1 = error).
pub fn run_stdio_server(app_identifier: &str) -> i32 {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = io::BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());

    // Initialize Core with no-op event sink
    let app_data_dir = app_data_dir_for(app_identifier);
    let core = Core::open(&app_data_dir, Arc::new(NoOpSink));

    // Resolve workspace path and set it as active in the search service
    let workspace = resolve_workspace(&app_data_dir, &core);
    let workspace_str = workspace.to_string_lossy().into_owned();
    if let Err(e) = core.search.prepare(&workspace_str) {
        eprintln!("[aquilum-mcp] WARNING: failed to prepare search index: {e}");
    }

    let write_message_closure = &write_message::<io::StdoutLock>;

    for result in std::iter::from_fn(|| read_message(&mut reader).transpose()) {
        let line = match result {
            Ok(line) => line,
            Err(_e) => break, // Read error — stdin closed
        };

        let parsed = match parse_message(&line) {
            Ok(value) => value,
            Err(error_response) => {
                // For parse errors without an id (notifications), don't respond
                if error_response.get("id").is_none() || error_response["id"].is_null() {
                    continue;
                }
                if (write_message_closure)(&mut writer, &error_response).is_err() {
                    return 1;
                }
                continue;
            }
        };

        let response = super::protocol::handle_message_for_stdio(&parsed, {
            &{
                let core = Arc::clone(&core);
                move |name: &str, arguments: &Value| {
                    super::tools::call(&core, name, arguments)
                }
            }
        });

        // For stdio: handle_message_for_stdio returns None for notifications (no response needed),
        // Some(response) for requests that need an answer
        if let Some(ref response) = response {
            if response.get("id").is_some() && !response["id"].is_null() {
                // Only write if there's an id (not a notification)
                if write_message(&mut writer, response).is_err() {
                    return 1;
                }
            }
        }
    }

    // Clean shutdown
    core.shutdown();
    0
}

/// Resolve the workspace path for the stdio server.
/// Priority: AQUILUM_WORKSPACE env var > first known root from UI state.
fn resolve_workspace(app_data_dir: &Path, core: &Core) -> PathBuf {
    // 1. Environment variable (standard MCP stdio pattern)
    if let Ok(workspace) = std::env::var("AQUILUM_WORKSPACE") {
        let path = PathBuf::from(&workspace);
        if path.is_dir() {
            return path;
        }
        eprintln!("[aquilum-mcp] WARNING: AQUILUM_WORKSPACE points to non-existent directory: {workspace}");
    }

    // 2. Fall back to known workspaces from UI state
    let roots = core.known_roots();
    if let Some(first) = roots.first() {
        return first.clone();
    }

    // 3. Last resort: data directory itself
    app_data_dir.to_owned()
}

/// Get the app data directory for the given app identifier.
fn app_data_dir_for(app_identifier: &str) -> PathBuf {
    let base = data_directory();
    base.join(app_identifier)
}

fn data_directory() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join("Library").join("Application Support"))
            .unwrap_or_else(|| PathBuf::from("."))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(|home| PathBuf::from(home).join(".local").join("share"))
            })
            .unwrap_or_else(|| PathBuf::from("."))
    }
}
