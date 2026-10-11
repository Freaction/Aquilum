use super::*;

pub fn new_token() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

pub(super) fn snippets(config: &AppConfig, executable: &str) -> Vec<(&'static str, String, String)> {
    let mcp = &config.mcp;
    let url = format!("http://127.0.0.1:{}/mcp", mcp.port);
    let bearer = format!("Bearer {}", mcp.token);
    let gemini = serde_json::json!({
        "mcpServers": { "aquilum": { "httpUrl": url, "headers": { "Authorization": bearer } } }
    });
    vec![
        (
            "Claude Code",
            t("settings.mcp.claudeHint"),
            format!("claude mcp add --transport http aquilum {url} --header \"Authorization: {bearer}\""),
        ),
        (
            "Codex CLI",
            t("settings.mcp.codexHint"),
            format!(
                "[mcp_servers.aquilum]\ncommand = \"{}\"\nargs = [\"--mcp-stdio\"]",
                executable.replace('\\', "\\\\")
            ),
        ),
        ("Gemini CLI", t("settings.mcp.geminiHint"), serde_json::to_string_pretty(&gemini).unwrap_or_default()),
    ]
}

pub(super) fn mcp(config: &AppConfig, ctx: &Context) -> Vec<Block> {
    let m = &config.mcp;
    let status = match &ctx.mcp {
        Some(McpStatus { error: Some(error), .. }) => Control::Status { text: error.clone(), tone: Tone::Danger },
        Some(McpStatus { running: true, port, .. }) => {
            Control::Status { text: t_with("settings.mcp.running", &[("port", &port.to_string())]), tone: Tone::Secondary }
        }
        _ if m.enabled => Control::Status { text: t("settings.mcp.starting"), tone: Tone::Muted },
        _ => Control::Status { text: t("settings.mcp.off"), tone: Tone::Muted },
    };
    let mut port = Number::new(f64::from(m.port), 1024.0, 65535.0, 1.0);
    port.commit = true;
    let mut blocks = vec![Block {
        title: t("settings.mcp.section"),
        rows: vec![
            row(t("settings.mcp.access"), Some(t("settings.mcp.accessHint")), Control::Switch(m.enabled), |c, v| {
                c.mcp.enabled = flag(v);
                if c.mcp.token.trim().is_empty() {
                    c.mcp.token = new_token();
                }
            }),
            info(t("settings.mcp.status"), None, status),
            row(t("settings.mcp.port"), None, Control::Number(port), |c, v| c.mcp.port = number(v) as u16),
            row(
                t("settings.mcp.token"),
                Some(t("settings.mcp.tokenHint")),
                Control::Text { value: m.token.clone(), commit: true },
                |c, v| c.mcp.token = text(v),
            ),
            info(
                t("settings.mcp.newToken"),
                Some(t("settings.mcp.newTokenHint")),
                Control::Buttons(vec![Button::new(t("settings.mcp.generate"), Action::NewToken)]),
            ),
            row(t("settings.mcp.allowWrite"), Some(t("settings.mcp.allowWriteHint")), Control::Switch(m.allow_write), |c, v| {
                c.mcp.allow_write = flag(v);
            }),
        ],
    }];
    if !m.enabled || m.token.is_empty() {
        return blocks;
    }
    let executable = ctx.mcp.as_ref().map_or(String::new(), |s| s.executable.clone());
    let rows = snippets(config, &executable)
        .into_iter()
        .enumerate()
        .map(|(i, (title, hint, code))| {
            let label = if ctx.copied == Some(i) { t("settings.mcp.copied") } else { t("settings.mcp.copy") };
            let mut row = info(title.to_owned(), Some(hint), Control::Buttons(vec![Button::new(label, Action::Copy(i, code.clone()))]));
            row.code = Some(code);
            row
        })
        .collect();
    blocks.push(Block { title: t("settings.mcp.connection"), rows });
    blocks
}
