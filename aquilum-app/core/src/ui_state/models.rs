use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn parse_uuid_column(value: String, index: usize) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownWorkspace {
    pub id: Uuid,
    pub path: String,
    pub last_seen_ms: i64,
    pub home_page: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSessionInput {
    pub workspace_id: Uuid,
    pub window_id: String,
    pub epoch: Uuid,
    pub now_ms: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TabKind {
    Document,
    Base,
    Empty,
    Graph,
}

impl TabKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Base => "base",
            Self::Empty => "empty",
            Self::Graph => "graph",
        }
    }

    pub fn parse(value: &str, index: usize) -> rusqlite::Result<Self> {
        match value {
            "document" => Ok(Self::Document),
            "base" => Ok(Self::Base),
            "empty" => Ok(Self::Empty),
            "graph" => Ok(Self::Graph),
            value => Err(rusqlite::Error::FromSqlConversionFailure(
                index,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unknown tab kind: {value}"),
                )),
            )),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabState {
    pub tab_id: Uuid,
    pub document_id: Option<Uuid>,
    pub kind: TabKind,
    pub position: i64,
    #[serde(default)]
    pub relative_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedTabState {
    pub tab_id: Uuid,
    pub document_id: Option<Uuid>,
    pub kind: TabKind,
    pub position: i64,
    pub relative_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStateInput {
    pub active_tab_id: Option<Uuid>,
    pub tabs: Option<Vec<TabState>>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphCameraState {
    pub center_x: f64,
    pub center_y: f64,
    pub scale: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewStateInput {
    #[serde(default, skip_serializing)]
    pub path: Option<String>,
    pub document_id: Uuid,
    pub pane_id: String,
    pub cursor_anchor: Vec<u8>,
    pub cursor_head: Vec<u8>,
    pub fallback_anchor: i64,
    pub fallback_head: i64,
    pub scroll_anchor: Vec<u8>,
    pub fallback_scroll_anchor: i64,
    pub scroll_offset_px: f64,
    pub focused_surface: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedViewState {
    pub document_id: Uuid,
    pub pane_id: String,
    pub cursor_anchor: Vec<u8>,
    pub cursor_head: Vec<u8>,
    pub fallback_anchor: i64,
    pub fallback_head: i64,
    pub scroll_anchor: Vec<u8>,
    pub fallback_scroll_anchor: i64,
    pub scroll_offset_px: f64,
    pub focused_surface: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveStateBatchInput {
    pub workspace_id: Uuid,
    pub window_id: String,
    pub epoch: Uuid,
    pub sequence: i64,
    pub now_ms: i64,
    pub session: Option<SessionStateInput>,
    pub views: Vec<ViewStateInput>,
    #[serde(default)]
    pub graph_camera: Option<GraphCameraState>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedSession {
    pub active_tab_id: Option<Uuid>,
    pub tabs: Vec<LoadedTabState>,
    pub views: Vec<LoadedViewState>,
    pub graph_camera: Option<GraphCameraState>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedReaderState {
    pub current: i64,
    pub cfi: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveReaderStateInput {
    pub workspace_id: Uuid,
    pub book_file: String,
    pub current: i64,
    pub cfi: Option<String>,
    pub now_ms: i64,
}
