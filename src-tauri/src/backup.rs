//! Copies of Claude's config, taken in MCP Manager's own folder before every
//! write and whenever the GUI finds the config changed since the last copy.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde::Serialize;
use serde_json::Value;

use crate::ops::ServerMap;

const KEEP: usize = 100;
const PREFIX: &str = "claude_desktop_config.";

#[derive(Clone, Debug, Serialize)]
pub struct BackupInfo {
    pub file: String,
    pub path: String,
    pub created: String,
    pub reason: String,
    pub size: u64,
    pub servers: Option<usize>,
}

/// Copies `config` into `dir` and returns the new file name, or None when
/// there is no config to copy.
pub fn take(dir: &Path, config: &Path, reason: &str) -> Result<Option<String>> {
    if !config.exists() {
        return Ok(None);
    }
    std::fs::create_dir_all(dir)?;
    let bytes = std::fs::read(config)?;
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S%.3f");
    let reason: String = reason.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    let file = format!("{PREFIX}{stamp}.{reason}.json");
    std::fs::write(dir.join(&file), bytes)?;
    prune(dir)?;
    Ok(Some(file))
}

/// Takes a copy only when the config differs from the newest one.
pub fn take_if_changed(dir: &Path, config: &Path, reason: &str) -> Result<Option<String>> {
    if !config.exists() {
        return Ok(None);
    }
    let current = std::fs::read(config)?;
    if let Some(latest) = files(dir)?.last() {
        if std::fs::read(dir.join(latest)).ok().as_deref() == Some(current.as_slice()) {
            return Ok(None);
        }
    }
    take(dir, config, reason)
}

fn files(dir: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with(PREFIX) && name.ends_with(".json") {
                names.push(name);
            }
        }
    }
    names.sort();
    Ok(names)
}

fn prune(dir: &Path) -> Result<()> {
    let names = files(dir)?;
    if names.len() > KEEP {
        for old in &names[..names.len() - KEEP] {
            let _ = std::fs::remove_file(dir.join(old));
        }
    }
    Ok(())
}

pub fn list(dir: &Path) -> Result<Vec<BackupInfo>> {
    let mut out = Vec::new();
    for file in files(dir)?.into_iter().rev() {
        let path = dir.join(&file);
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let middle = &file[PREFIX.len()..file.len() - ".json".len()];
        let (stamp, reason) = middle.split_once('.').map_or((middle, ""), |(s, rest)| {
            // The stamp carries milliseconds after a dot: "..._20-31-05.123.reason".
            match rest.split_once('.') {
                Some((_ms, r)) => (s, r),
                None => (s, rest),
            }
        });
        let (date, time) = stamp.split_once('_').unwrap_or((stamp, ""));
        let created = format!("{date} {}", time.replace('-', ":"));
        let reason = reason.to_string();
        out.push(BackupInfo {
            servers: read_servers(dir, &file).ok().map(|s| s.len()),
            path: path.to_string_lossy().into_owned(),
            file,
            created,
            reason,
            size,
        });
    }
    Ok(out)
}

fn checked(dir: &Path, file: &str) -> Result<PathBuf> {
    if file.contains(['/', '\\']) || file.contains("..") || !file.starts_with(PREFIX) {
        bail!("not a backup file name: {file}");
    }
    Ok(dir.join(file))
}

pub fn read_servers(dir: &Path, file: &str) -> Result<ServerMap> {
    let text = std::fs::read_to_string(checked(dir, file)?)?;
    let root: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))?;
    Ok(match root.get("mcpServers") {
        Some(Value::Object(m)) => m.clone(),
        _ => ServerMap::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_if_changed_skips_identical_copies() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("c.json");
        let backups = dir.path().join("b");
        std::fs::write(&cfg, r#"{"mcpServers":{"a":{"command":"x"}}}"#).unwrap();
        assert!(take_if_changed(&backups, &cfg, "startup").unwrap().is_some());
        assert!(take_if_changed(&backups, &cfg, "startup").unwrap().is_none());
        std::fs::write(&cfg, r#"{"mcpServers":{}}"#).unwrap();
        assert!(take_if_changed(&backups, &cfg, "startup").unwrap().is_some());
        let list = list(&backups).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].servers, Some(0));
        assert_eq!(list[1].servers, Some(1));
        assert_eq!(list[0].reason, "startup");
        assert!(read_servers(&backups, "../c.json").is_err());
    }
}
