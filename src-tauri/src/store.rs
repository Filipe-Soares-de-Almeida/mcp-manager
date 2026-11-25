//! MCP Manager's own SQLite database: the queue of changes waiting for Claude
//! to close, the disabled servers, the history of applied changes and a few
//! settings. The GUI, the MCP server and the watcher are separate processes
//! that share it, so it runs in WAL mode with a busy timeout.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::Value;

use crate::ops::{Op, ServerMap};

pub fn open(path: &Path) -> Result<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let conn = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
    conn.busy_timeout(Duration::from_secs(15))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS pending (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             created_at TEXT NOT NULL,
             source TEXT NOT NULL,
             op TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS disabled_servers (
             name TEXT PRIMARY KEY,
             spec TEXT NOT NULL,
             disabled_at TEXT NOT NULL,
             position INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS history (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             applied_at TEXT NOT NULL,
             source TEXT NOT NULL,
             description TEXT NOT NULL,
             op TEXT NOT NULL,
             ok INTEGER NOT NULL,
             message TEXT,
             backup TEXT
         );
         CREATE TABLE IF NOT EXISTS settings (
             key TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );",
    )?;
    Ok(conn)
}

pub fn now() -> String {
    chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}

#[derive(Clone, Debug, Serialize)]
pub struct Pending {
    pub id: i64,
    pub created_at: String,
    pub source: String,
    pub op: Op,
    pub description: String,
    pub kind: String,
    pub names: Vec<String>,
}

pub fn pending(conn: &Connection) -> Result<Vec<Pending>> {
    let mut stmt = conn.prepare("SELECT id, created_at, source, op FROM pending ORDER BY id")?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, created_at, source, op) = row?;
        let op: Op = serde_json::from_str(&op).with_context(|| format!("queued change #{id} is unreadable"))?;
        out.push(Pending { id, created_at, source, description: op.describe(), kind: op.kind().into(), names: op.names(), op });
    }
    Ok(out)
}

pub fn pending_count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM pending", [], |r| r.get(0))?)
}

pub fn push_pending(conn: &Connection, source: &str, op: &Op) -> Result<i64> {
    conn.execute(
        "INSERT INTO pending (created_at, source, op) VALUES (?1, ?2, ?3)",
        params![now(), source, serde_json::to_string(op)?],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_pending(conn: &Connection, id: i64) -> Result<bool> {
    Ok(conn.execute("DELETE FROM pending WHERE id = ?1", [id])? > 0)
}

pub fn delete_pending_up_to(conn: &Connection, max_id: i64) -> Result<usize> {
    Ok(conn.execute("DELETE FROM pending WHERE id <= ?1", [max_id])?)
}

pub fn disabled(conn: &Connection) -> Result<ServerMap> {
    let mut stmt = conn.prepare("SELECT name, spec FROM disabled_servers ORDER BY position, name")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut out = ServerMap::new();
    for row in rows {
        let (name, spec) = row?;
        let spec: Value =
            serde_json::from_str(&spec).with_context(|| format!("disabled server \"{name}\" is unreadable"))?;
        out.insert(name, spec);
    }
    Ok(out)
}

/// Makes the table hold exactly `servers`, keeping `disabled_at` for the ones
/// that were already there.
pub fn replace_disabled(conn: &Connection, servers: &ServerMap) -> Result<()> {
    let existing = disabled(conn)?;
    for name in existing.keys() {
        if !servers.contains_key(name) {
            conn.execute("DELETE FROM disabled_servers WHERE name = ?1", [name])?;
        }
    }
    for (pos, (name, spec)) in servers.iter().enumerate() {
        let text = serde_json::to_string(spec)?;
        if existing.contains_key(name) {
            conn.execute(
                "UPDATE disabled_servers SET spec = ?2, position = ?3 WHERE name = ?1",
                params![name, text, pos as i64],
            )?;
        } else {
            conn.execute(
                "INSERT INTO disabled_servers (name, spec, disabled_at, position) VALUES (?1, ?2, ?3, ?4)",
                params![name, text, now(), pos as i64],
            )?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub applied_at: String,
    pub source: String,
    pub description: String,
    pub kind: String,
    pub names: Vec<String>,
    pub ok: bool,
    pub message: Option<String>,
    pub backup: Option<String>,
}

pub fn add_history(
    conn: &Connection,
    source: &str,
    op: &Op,
    result: &std::result::Result<(), String>,
    backup: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO history (applied_at, source, description, op, ok, message, backup)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            now(),
            source,
            op.describe(),
            serde_json::to_string(op)?,
            result.is_ok() as i64,
            result.as_ref().err(),
            backup
        ],
    )?;
    Ok(())
}

pub fn history(conn: &Connection, limit: i64) -> Result<Vec<HistoryEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, applied_at, source, description, ok, message, backup, op FROM history ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit], |r| {
        let op = serde_json::from_str::<Op>(&r.get::<_, String>(7)?).ok();
        Ok(HistoryEntry {
            id: r.get(0)?,
            applied_at: r.get(1)?,
            source: r.get(2)?,
            description: r.get(3)?,
            kind: op.as_ref().map(|o| o.kind().to_string()).unwrap_or_default(),
            names: op.as_ref().map(Op::names).unwrap_or_default(),
            ok: r.get::<_, i64>(4)? != 0,
            message: r.get(5)?,
            backup: r.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn last_history_id(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT COALESCE(MAX(id), 0) FROM history", [], |r| r.get(0))?)
}

pub fn setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).optional()?)
}

pub fn set_setting(conn: &Connection, key: &str, value: Option<&str>) -> Result<()> {
    match value {
        Some(v) => conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, v],
        )?,
        None => conn.execute("DELETE FROM settings WHERE key = ?1", [key])?,
    };
    Ok(())
}
