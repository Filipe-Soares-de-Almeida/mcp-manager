//! Where MCP Manager keeps its own files.
//!
//! The data folder sits under %USERPROFILE%, not %APPDATA%: Claude Desktop is
//! an MSIX package, and Windows redirects AppData writes made by the processes
//! it starts (this exe in `--mcp` mode) into the package's private copy, where
//! the GUI would never see them.

use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("MCP_MANAGER_HOME") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".mcp-manager")
}

pub fn db_path(data: &std::path::Path) -> PathBuf {
    data.join("mcp-manager.db")
}

pub fn backups_dir(data: &std::path::Path) -> PathBuf {
    data.join("backups")
}

pub fn watcher_lock(data: &std::path::Path) -> PathBuf {
    data.join("watcher.lock")
}

pub fn log_file(data: &std::path::Path) -> PathBuf {
    data.join("mcp-manager.log")
}

/// Appends one line to the log. Best effort: logging never fails a change.
pub fn log(data: &std::path::Path, line: &str) {
    use std::io::Write;
    let path = log_file(data);
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > 1_000_000 {
            let _ = std::fs::rename(&path, data.join("mcp-manager.old.log"));
        }
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{} [{}] {line}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), std::process::id());
    }
}
