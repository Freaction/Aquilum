pub const SCHEMA: &str = "CREATE TABLE workspaces (
         id TEXT PRIMARY KEY,
         path TEXT NOT NULL UNIQUE,
         last_seen_ms INTEGER NOT NULL,
         home_page TEXT NOT NULL DEFAULT ''
     );
     CREATE TABLE documents (
         id TEXT PRIMARY KEY,
         workspace_id TEXT NOT NULL,
         relative_path TEXT NOT NULL,
         status TEXT NOT NULL DEFAULT 'active',
         missing_since_ms INTEGER,
         UNIQUE(workspace_id, id),
         FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
     );
     CREATE UNIQUE INDEX active_document_path
         ON documents(workspace_id, relative_path) WHERE status = 'active';
     CREATE TABLE sessions (
         workspace_id TEXT NOT NULL,
         window_id TEXT NOT NULL,
         epoch TEXT NOT NULL,
         last_sequence INTEGER NOT NULL DEFAULT -1,
         active_tab_id TEXT,
         graph_center_x REAL,
         graph_center_y REAL,
         graph_scale REAL,
         updated_at_ms INTEGER NOT NULL,
         PRIMARY KEY(workspace_id, window_id),
         FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
     );
     CREATE TABLE tabs (
         workspace_id TEXT NOT NULL,
         window_id TEXT NOT NULL,
         tab_id TEXT NOT NULL,
         document_id TEXT,
         kind TEXT NOT NULL,
         position INTEGER NOT NULL,
         relative_path TEXT,
         PRIMARY KEY(workspace_id, window_id, tab_id),
         FOREIGN KEY(workspace_id, window_id)
             REFERENCES sessions(workspace_id, window_id) ON DELETE CASCADE,
         FOREIGN KEY(workspace_id, document_id)
             REFERENCES documents(workspace_id, id) ON DELETE CASCADE
     );
     CREATE TABLE view_states (
         workspace_id TEXT NOT NULL,
         window_id TEXT NOT NULL,
         document_id TEXT NOT NULL,
         pane_id TEXT NOT NULL,
         cursor_anchor BLOB NOT NULL,
         cursor_head BLOB NOT NULL,
         fallback_anchor INTEGER NOT NULL,
         fallback_head INTEGER NOT NULL,
         scroll_anchor BLOB NOT NULL,
         fallback_scroll_anchor INTEGER NOT NULL DEFAULT 0,
         scroll_offset_px REAL NOT NULL,
         focused_surface TEXT NOT NULL,
         updated_at_ms INTEGER NOT NULL,
         PRIMARY KEY(workspace_id, window_id, document_id, pane_id),
         FOREIGN KEY(workspace_id, document_id)
             REFERENCES documents(workspace_id, id) ON DELETE CASCADE
     );
     CREATE TABLE base_view_states (
         workspace_id TEXT NOT NULL,
         relative_path TEXT NOT NULL,
         selected_view INTEGER NOT NULL CHECK(selected_view >= 0),
         PRIMARY KEY(workspace_id, relative_path),
         FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
     );
     CREATE TABLE reader_states (
         workspace_id TEXT NOT NULL,
         book_file TEXT NOT NULL,
         current INTEGER NOT NULL,
         cfi TEXT,
         updated_at_ms INTEGER NOT NULL,
         PRIMARY KEY(workspace_id, book_file),
         FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
     );";
