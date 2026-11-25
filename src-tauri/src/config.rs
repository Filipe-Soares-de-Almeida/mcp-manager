//! Reading and writing `claude_desktop_config.json`.
//!
//! Only `mcpServers` is ever changed. Every other key (`preferences`,
//! `coworkUserFilesPath`, ...) and the key order are written back as read.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};

use crate::ops::ServerMap;

pub struct ClaudeConfig {
    pub path: PathBuf,
    pub root: Map<String, Value>,
    trailing_newline: bool,
}

impl ClaudeConfig {
    pub fn read(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self { path: path.to_path_buf(), root: Map::new(), trailing_newline: false });
        }
        let raw = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let raw_trimmed = raw.trim_start_matches('\u{feff}');
        let root = if raw_trimmed.trim().is_empty() {
            Map::new()
        } else {
            match serde_json::from_str::<Value>(raw_trimmed)
                .with_context(|| format!("{} is not valid JSON", path.display()))?
            {
                Value::Object(m) => m,
                _ => bail!("{} does not hold a JSON object", path.display()),
            }
        };
        Ok(Self { path: path.to_path_buf(), root, trailing_newline: raw.ends_with('\n') })
    }

    pub fn servers(&self) -> ServerMap {
        match self.root.get("mcpServers") {
            Some(Value::Object(m)) => m.clone(),
            _ => Map::new(),
        }
    }

    pub fn set_servers(&mut self, servers: ServerMap) {
        self.root.insert("mcpServers".into(), Value::Object(servers));
    }

    /// Writes through a temporary file in the same folder and a rename, so a
    /// crash midway leaves either the old file or the new one.
    pub fn write(&self) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut text = serde_json::to_string_pretty(&Value::Object(self.root.clone()))?;
        if self.trailing_newline {
            text.push('\n');
        }
        let tmp = self.path.with_extension("json.mcp-manager.tmp");
        std::fs::write(&tmp, text.as_bytes()).with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, &self.path).with_context(|| format!("replacing {}", self.path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn write_keeps_other_keys_order_and_numbers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("claude_desktop_config.json");
        let original = "{\n  \"mcpServers\": {\n    \"a\": {\n      \"command\": \"a.exe\"\n    }\n  },\n  \"coworkUserFilesPath\": \"C:\\\\x\",\n  \"preferences\": {\n    \"zoom\": 1.0,\n    \"Key\": 1,\n    \"key\": 2\n  }\n}";
        std::fs::write(&path, original).unwrap();

        let mut cfg = ClaudeConfig::read(&path).unwrap();
        let mut servers = cfg.servers();
        servers.insert("b".into(), json!({ "command": "b.exe" }));
        cfg.set_servers(servers);
        cfg.write().unwrap();

        let back = std::fs::read_to_string(&path).unwrap();
        let keys: Vec<String> = serde_json::from_str::<Map<String, Value>>(&back).unwrap().keys().cloned().collect();
        assert_eq!(keys, ["mcpServers", "coworkUserFilesPath", "preferences"]);
        assert!(back.contains("\"zoom\": 1.0"), "number text kept: {back}");
        assert!(back.contains("\"Key\": 1") && back.contains("\"key\": 2"));
        assert!(back.contains("\"b\""));
        assert!(!back.ends_with('\n'));
    }

    #[test]
    fn missing_file_reads_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = ClaudeConfig::read(&dir.path().join("none.json")).unwrap();
        assert!(cfg.servers().is_empty());
    }
}
