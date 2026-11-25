//! The one entry point the GUI, the MCP server and the watcher all go through.
//!
//! A change is checked against the list as it will be once the queue is
//! applied, then queued. It is written to Claude's config straight away only
//! when Claude Desktop is closed; otherwise it waits for Claude to close.
//! Changes to a disabled server never touch the config, so they are saved to
//! the database at once.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use rusqlite::{Connection, TransactionBehavior};
use serde::Serialize;
use serde_json::{Value, json};

use crate::backup;
use crate::claude::{self, Install};
use crate::config::ClaudeConfig;
use crate::launch;
use crate::locale;
use crate::ops::{self, Model, Op, Row, ServerMap};
use crate::paths;
use crate::store::{self, HistoryEntry, Pending};

pub const SELF_NAME: &str = "mcp-manager";
const SETTING_EXE: &str = "claude_exe";
const SETTING_CONFIG: &str = "config_override";
const SETTING_TRAY: &str = "tray_with_claude";
const SETTING_LOCALE: &str = "locale";

pub struct Manager {
    pub data: PathBuf,
    conn: Connection,
    config_env: Option<PathBuf>,
}

#[derive(Serialize)]
pub struct ClaudeStatus {
    pub running: bool,
    pub processes: usize,
    pub install: Option<Install>,
}

#[derive(Serialize)]
pub struct Invalid {
    pub id: i64,
    pub error: String,
}

#[derive(Serialize)]
pub struct SelfEntry {
    pub name: String,
    pub up_to_date: bool,
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct Startup {
    /// Starts in the tray when Windows starts (HKCU Run key).
    pub autostart: bool,
    /// The MCP server starts the app in the tray when it is not running.
    pub tray_with_claude: bool,
    pub shortcut: bool,
}

#[derive(Serialize)]
pub struct LocaleInfo {
    /// "auto", "pt-BR" or "en-US", as chosen in Settings.
    pub setting: String,
    /// The language the interface uses.
    pub resolved: &'static str,
}

#[derive(Serialize)]
pub struct Snapshot {
    pub version: &'static str,
    pub locale: LocaleInfo,
    pub startup: Startup,
    pub claude: ClaudeStatus,
    pub config_path: String,
    pub config_source: &'static str,
    pub config_exists: bool,
    pub config_candidates: Vec<String>,
    pub config_error: Option<String>,
    pub data_dir: String,
    pub db_path: String,
    pub rows: Vec<Row>,
    pub pending: Vec<Pending>,
    pub invalid: Vec<Invalid>,
    pub history: Vec<HistoryEntry>,
    pub self_exe: String,
    pub self_entry: Option<SelfEntry>,
}

#[derive(Serialize, Debug)]
pub struct Outcome {
    /// Written to the config (or to the database, for a disabled server).
    pub applied: bool,
    /// Waiting for Claude Desktop to close.
    pub queued_id: Option<i64>,
    pub claude_running: bool,
    pub message: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct Failed {
    pub id: i64,
    pub description: String,
    pub error: String,
}

#[derive(Serialize, Debug, Default, Clone)]
pub struct ApplyReport {
    pub applied: usize,
    pub failed: Vec<Failed>,
    pub backup: Option<String>,
}

pub const QUEUED_NOTE: &str = "Claude Desktop is running, and it rewrites its config while it runs, so the change \
is queued and will be written as soon as Claude is closed. Restart Claude to load it: quit Claude completely \
(including from the tray), or use \"Restart Claude\" in the MCP Manager app, which closes Claude, applies the \
queue and opens Claude again.";

impl Manager {
    pub fn open() -> Result<Self> {
        let data = paths::data_dir();
        let config_env = std::env::var_os("MCP_MANAGER_CONFIG").map(PathBuf::from);
        Self::open_at(&data, config_env)
    }

    pub fn open_at(data: &Path, config_env: Option<PathBuf>) -> Result<Self> {
        std::fs::create_dir_all(paths::backups_dir(data))
            .with_context(|| format!("creating {}", data.display()))?;
        let conn = store::open(&paths::db_path(data))?;
        Ok(Self { data: data.to_path_buf(), conn, config_env })
    }

    pub fn backups_dir(&self) -> PathBuf {
        paths::backups_dir(&self.data)
    }

    /// Claude Desktop, found from its executable. A running or newly found
    /// executable is remembered, so the config can still be found while
    /// Claude is closed.
    pub fn install(&self) -> Option<Install> {
        let remembered = store::setting(&self.conn, SETTING_EXE).ok().flatten().map(PathBuf::from);
        let found = claude::detect(remembered.as_deref());
        if let Some(i) = &found {
            if remembered.as_deref() != Some(i.exe.as_path()) {
                let _ = store::set_setting(&self.conn, SETTING_EXE, Some(&i.exe.to_string_lossy()));
            }
        }
        found
    }

    fn config_path_for(&self, install: Option<&Install>) -> (PathBuf, &'static str) {
        if let Some(p) = &self.config_env {
            return (p.clone(), "env");
        }
        if let Ok(Some(p)) = store::setting(&self.conn, SETTING_CONFIG) {
            return (PathBuf::from(p), "override");
        }
        (claude::config_path(install), "detected")
    }

    pub fn config_path(&self) -> PathBuf {
        self.config_path_for(self.install().as_ref()).0
    }

    pub fn locale_setting(&self) -> String {
        store::setting(&self.conn, SETTING_LOCALE).ok().flatten().unwrap_or_else(|| "auto".into())
    }

    pub fn locale(&self) -> &'static str {
        locale::resolve(&self.locale_setting())
    }

    pub fn set_locale(&self, setting: &str) -> Result<()> {
        if setting != "auto" && !locale::SUPPORTED.contains(&setting) {
            bail!("unsupported language: {setting}");
        }
        store::set_setting(&self.conn, SETTING_LOCALE, Some(setting))
    }

    pub fn tray_with_claude(&self) -> bool {
        !matches!(store::setting(&self.conn, SETTING_TRAY), Ok(Some(v)) if v == "0")
    }

    pub fn set_tray_with_claude(&self, enabled: bool) -> Result<()> {
        store::set_setting(&self.conn, SETTING_TRAY, Some(if enabled { "1" } else { "0" }))
    }

    pub fn set_config_override(&self, path: Option<&str>) -> Result<()> {
        store::set_setting(&self.conn, SETTING_CONFIG, path.filter(|p| !p.trim().is_empty()))
    }

    fn real_model(&self, cfg_path: &Path) -> Result<Model> {
        let cfg = ClaudeConfig::read(cfg_path)?;
        Ok(Model { active: cfg.servers(), disabled: store::disabled(&self.conn)? })
    }

    pub fn pending(&self) -> Result<Vec<Pending>> {
        store::pending(&self.conn)
    }

    pub fn pending_count(&self) -> Result<i64> {
        store::pending_count(&self.conn)
    }

    pub fn history(&self, limit: i64) -> Result<Vec<HistoryEntry>> {
        store::history(&self.conn, limit)
    }

    /// Changes whenever anything the GUI shows could have changed.
    pub fn fingerprint(&self, running: bool) -> String {
        let cfg = self.config_path();
        let meta = std::fs::metadata(&cfg).ok();
        let stamp = meta.as_ref().and_then(|m| m.modified().ok()).map(|t| format!("{t:?}")).unwrap_or_default();
        let pending: i64 = self
            .conn
            .query_row("SELECT COALESCE(MAX(id),0) * 100000 + COUNT(*) FROM pending", [], |r| r.get(0))
            .unwrap_or(-1);
        let disabled: String = self
            .conn
            .query_row("SELECT COALESCE(group_concat(name || length(spec)), '') FROM disabled_servers", [], |r| {
                r.get(0)
            })
            .unwrap_or_default();
        let hist = store::last_history_id(&self.conn).unwrap_or(-1);
        format!("{running}|{}|{stamp}|{pending}|{disabled}|{hist}", cfg.display())
    }

    pub fn snapshot(&self) -> Result<Snapshot> {
        let install = self.install();
        let running = claude::running();
        let (cfg_path, source) = self.config_path_for(install.as_ref());
        let pending = store::pending(&self.conn)?;
        let (real, config_error) = match self.real_model(&cfg_path) {
            Ok(m) => (m, None),
            Err(e) => (Model { active: ServerMap::new(), disabled: store::disabled(&self.conn)? }, Some(format!("{e:#}"))),
        };
        let ops: Vec<(i64, Op)> = pending.iter().map(|p| (p.id, p.op.clone())).collect();
        let (rows, invalid) = ops::effective(&real, &ops);
        let self_exe = std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let self_entry = find_self(&rows, &self_exe);
        let locale_setting = self.locale_setting();
        Ok(Snapshot {
            version: env!("CARGO_PKG_VERSION"),
            locale: LocaleInfo { resolved: locale::resolve(&locale_setting), setting: locale_setting },
            startup: Startup {
                autostart: launch::autostart_enabled(),
                tray_with_claude: self.tray_with_claude(),
                shortcut: launch::shortcut_exists(),
            },
            claude: ClaudeStatus { running: !running.is_empty(), processes: running.len(), install: install.clone() },
            config_exists: cfg_path.exists(),
            config_candidates: claude::config_candidates(install.as_ref())
                .into_iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
            config_path: cfg_path.to_string_lossy().into_owned(),
            config_source: source,
            config_error,
            data_dir: self.data.to_string_lossy().into_owned(),
            db_path: paths::db_path(&self.data).to_string_lossy().into_owned(),
            rows,
            pending,
            invalid: invalid.into_iter().map(|(id, error)| Invalid { id, error }).collect(),
            history: store::history(&self.conn, 40)?,
            self_exe,
            self_entry,
        })
    }

    /// A copy of the config when it changed since the newest copy.
    pub fn backup_if_changed(&self, reason: &str) -> Result<Option<String>> {
        backup::take_if_changed(&self.backups_dir(), &self.config_path(), reason)
    }

    pub fn backup_now(&self) -> Result<Option<String>> {
        backup::take(&self.backups_dir(), &self.config_path(), "manual")
    }

    pub fn submit(&mut self, source: &str, op: Op) -> Result<Outcome> {
        let cfg_path = self.config_path();
        let real = self.real_model(&cfg_path)?;
        let pending = store::pending(&self.conn)?;
        let mut sim = real.clone();
        for p in &pending {
            let _ = sim.apply(&p.op);
        }
        sim.apply(&op).map_err(|e| anyhow!(e))?;

        let names = op.names();
        let referenced = pending.iter().any(|p| p.op.names().iter().any(|n| names.contains(n)));
        let only_db = match &op {
            Op::Update { name, .. } | Op::Remove { name } => {
                real.disabled.contains_key(name) && !real.active.contains_key(name)
            }
            _ => false,
        };
        let running = claude::is_running();

        if only_db && !referenced {
            let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let mut model = Model { active: ServerMap::new(), disabled: store::disabled(&tx)? };
            model.apply(&op).map_err(|e| anyhow!(e))?;
            store::replace_disabled(&tx, &model.disabled)?;
            store::add_history(&tx, source, &op, &Ok(()), None)?;
            tx.commit()?;
            return Ok(Outcome {
                applied: true,
                queued_id: None,
                claude_running: running,
                message: format!(
                    "{}: saved. The server is disabled, so it lives in MCP Manager's database and Claude's \
                     config was not touched; no restart is needed.",
                    op.describe()
                ),
            });
        }

        let id = store::push_pending(&self.conn, source, &op)?;
        if !running {
            let report = self.apply_pending(source)?;
            if let Some(f) = report.failed.iter().find(|f| f.id == id) {
                bail!("{}: {}", f.description, f.error);
            }
            return Ok(Outcome {
                applied: true,
                queued_id: None,
                claude_running: false,
                message: format!(
                    "{}: written to {}. Claude Desktop is closed; it loads the change when it starts.",
                    op.describe(),
                    cfg_path.display()
                ),
            });
        }
        Ok(Outcome {
            applied: false,
            queued_id: Some(id),
            claude_running: true,
            message: format!("{}: queued as change #{id}. {QUEUED_NOTE}", op.describe()),
        })
    }

    /// Writes every queued change to Claude's config in one transaction,
    /// after copying the config to the backups folder.
    pub fn apply_pending(&mut self, source: &str) -> Result<ApplyReport> {
        let cfg_path = self.config_path();
        if claude::is_running() {
            bail!("Claude Desktop is running: its config can only be written once it is closed");
        }
        let backups = self.backups_dir();
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let pending = store::pending(&tx)?;
        let Some(last) = pending.last().map(|p| p.id) else {
            return Ok(ApplyReport::default());
        };
        let mut cfg = ClaudeConfig::read(&cfg_path)?;
        let backup = backup::take(&backups, &cfg_path, "before-apply")?;
        let before = serde_json::to_string(&cfg.servers())?;
        let mut model = Model { active: cfg.servers(), disabled: store::disabled(&tx)? };
        let mut report = ApplyReport { backup: backup.clone(), ..Default::default() };
        for p in &pending {
            let result = model.apply(&p.op);
            store::add_history(&tx, &p.source, &p.op, &result, backup.as_deref())?;
            match result {
                Ok(()) => report.applied += 1,
                Err(error) => report.failed.push(Failed { id: p.id, description: p.description.clone(), error }),
            }
        }
        if serde_json::to_string(&model.active)? != before {
            cfg.set_servers(model.active.clone());
            cfg.write()?;
        }
        store::replace_disabled(&tx, &model.disabled)?;
        store::delete_pending_up_to(&tx, last)?;
        tx.commit()?;
        paths::log(
            &self.data,
            &format!(
                "applied {} change(s) via {source}, {} failed, backup {:?}, config {}",
                report.applied,
                report.failed.len(),
                report.backup,
                cfg_path.display()
            ),
        );
        Ok(report)
    }

    /// Drops one queued change, then any later change that no longer applies
    /// without it. Returns the ids of those later ones.
    pub fn cancel(&mut self, id: i64) -> Result<Vec<i64>> {
        let real = self.real_model(&self.config_path())?;
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !store::delete_pending(&tx, id)? {
            bail!("there is no queued change #{id}");
        }
        let rest: Vec<(i64, Op)> = store::pending(&tx)?.into_iter().map(|p| (p.id, p.op)).collect();
        let (_, invalid) = ops::effective(&real, &rest);
        let mut dropped = Vec::new();
        for (bad, _) in invalid {
            store::delete_pending(&tx, bad)?;
            dropped.push(bad);
        }
        tx.commit()?;
        Ok(dropped)
    }

    pub fn discard_all(&mut self) -> Result<usize> {
        Ok(self.conn.execute("DELETE FROM pending", [])?)
    }

    /// Closes Claude Desktop, applies the queue and starts Claude again.
    pub fn restart_claude(&mut self, progress: &dyn Fn(&str)) -> Result<ApplyReport> {
        let install = self.install().ok_or_else(|| anyhow!("Claude Desktop was not found on this computer"))?;
        progress("closing");
        claude::close(Duration::from_secs(6))?;
        std::thread::sleep(Duration::from_millis(1200));
        progress("applying");
        let report = self.apply_pending("restart")?;
        progress("starting");
        claude::launch(&install)?;
        Ok(report)
    }

    pub fn self_spec() -> Result<ServerMap> {
        let exe = std::env::current_exe()?;
        Ok(json!({ "command": exe.to_string_lossy(), "args": ["--mcp"] }).as_object().cloned().unwrap_or_default())
    }

    /// Adds MCP Manager's own `--mcp` entry, or points the existing one at
    /// this executable.
    pub fn register_self(&mut self, source: &str) -> Result<Outcome> {
        let snap = self.snapshot()?;
        let spec = Self::self_spec()?;
        let existing = snap
            .self_entry
            .map(|e| e.name)
            .or_else(|| snap.rows.iter().find(|r| r.name == SELF_NAME && !r.removed).map(|r| r.name.clone()));
        let op = match existing {
            Some(name) => {
                let mut merged = snap
                    .rows
                    .iter()
                    .find(|r| r.name == name)
                    .and_then(|r| r.spec.as_object().cloned())
                    .unwrap_or_default();
                for (k, v) in spec {
                    merged.insert(k, v);
                }
                Op::Update { name, new_name: None, spec: merged }
            }
            None => Op::Add { name: SELF_NAME.into(), spec },
        };
        self.submit(source, op)
    }

    /// The spec a server will have once the queue is applied.
    pub fn effective_spec(&self, name: &str) -> Result<(ServerMap, bool)> {
        let snap = self.snapshot()?;
        let row = snap
            .rows
            .into_iter()
            .find(|r| r.name == name && !r.removed)
            .ok_or_else(|| anyhow!("no server named \"{name}\""))?;
        Ok((row.spec.as_object().cloned().unwrap_or_default(), row.enabled))
    }
}

fn same_path(a: &str, b: &str) -> bool {
    let norm = |s: &str| s.trim().trim_matches('"').replace('/', "\\").to_lowercase();
    norm(a) == norm(b)
}

fn find_self(rows: &[Row], exe: &str) -> Option<SelfEntry> {
    let is_mcp = |r: &Row| {
        r.spec.get("args").and_then(Value::as_array).is_some_and(|a| a.iter().any(|v| v == "--mcp"))
    };
    let command = |r: &Row| r.spec.get("command").and_then(Value::as_str).unwrap_or_default().to_string();
    let live: Vec<&Row> = rows.iter().filter(|r| !r.removed).collect();
    if let Some(r) = live.iter().find(|r| is_mcp(r) && same_path(&command(r), exe)) {
        return Some(SelfEntry { name: r.name.clone(), up_to_date: true, enabled: r.enabled });
    }
    live.iter()
        .find(|r| r.name == SELF_NAME || (is_mcp(r) && command(r).to_lowercase().contains("mcp-manager")))
        .map(|r| SelfEntry { name: r.name.clone(), up_to_date: false, enabled: r.enabled })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(config: &str) -> (tempfile::TempDir, Manager) {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("claude_desktop_config.json");
        std::fs::write(&cfg, config).unwrap();
        let mgr = Manager::open_at(&dir.path().join("data"), Some(cfg)).unwrap();
        (dir, mgr)
    }

    fn spec(cmd: &str) -> ServerMap {
        json!({ "command": cmd }).as_object().unwrap().clone()
    }

    fn read_cfg(dir: &tempfile::TempDir) -> Value {
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("claude_desktop_config.json")).unwrap()).unwrap()
    }

    // Claude Desktop may be open while the tests run, so they queue changes
    // directly and apply them with the process probe faked to "closed".
    impl Manager {
        fn queue(&mut self, op: Op) -> i64 {
            store::push_pending(&self.conn, "test", &op).unwrap()
        }
    }

    #[test]
    fn queue_disable_enable_round_trip_through_the_database() {
        let (dir, mut mgr) = setup(r#"{"mcpServers":{"a":{"command":"a.exe","env":{"K":"v"}}},"preferences":{"x":1}}"#);
        mgr.queue(Op::Disable { name: "a".into() });
        mgr.queue(Op::Add { name: "b".into(), spec: spec("b.exe") });
        let report = apply_without_probe(&mut mgr);
        assert_eq!(report.applied, 2);
        assert!(report.backup.is_some());
        let cfg = read_cfg(&dir);
        assert!(cfg["mcpServers"].get("a").is_none());
        assert_eq!(cfg["mcpServers"]["b"]["command"], "b.exe");
        assert_eq!(cfg["preferences"]["x"], 1);
        assert_eq!(store::disabled(&mgr.conn).unwrap()["a"]["env"]["K"], "v");

        mgr.queue(Op::Enable { name: "a".into() });
        apply_without_probe(&mut mgr);
        let cfg = read_cfg(&dir);
        assert_eq!(cfg["mcpServers"]["a"]["env"]["K"], "v");
        assert!(store::disabled(&mgr.conn).unwrap().is_empty());
        assert_eq!(mgr.pending_count().unwrap(), 0);
        assert_eq!(mgr.history(10).unwrap().len(), 3);
    }

    #[test]
    fn a_failing_change_is_recorded_and_the_rest_still_apply() {
        let (dir, mut mgr) = setup(r#"{"mcpServers":{}}"#);
        mgr.queue(Op::Remove { name: "ghost".into() });
        mgr.queue(Op::Add { name: "c".into(), spec: spec("c.exe") });
        let report = apply_without_probe(&mut mgr);
        assert_eq!(report.applied, 1);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(read_cfg(&dir)["mcpServers"]["c"]["command"], "c.exe");
        assert!(!mgr.history(10).unwrap()[1].ok);
    }

    #[test]
    fn cancel_drops_the_changes_that_depended_on_it() {
        let (_dir, mut mgr) = setup(r#"{"mcpServers":{}}"#);
        let add = mgr.queue(Op::Add { name: "d".into(), spec: spec("d.exe") });
        let edit = mgr.queue(Op::Update { name: "d".into(), new_name: None, spec: spec("d2.exe") });
        let other = mgr.queue(Op::Add { name: "e".into(), spec: spec("e.exe") });
        assert_eq!(mgr.cancel(add).unwrap(), vec![edit]);
        let left: Vec<i64> = mgr.pending().unwrap().iter().map(|p| p.id).collect();
        assert_eq!(left, vec![other]);
    }

    #[test]
    fn finds_its_own_entry() {
        let exe = r"C:\Apps\MCP Manager\mcp-manager.exe";
        let rows = vec![Row {
            name: "mm".into(),
            spec: json!({ "command": exe.to_uppercase(), "args": ["--mcp"] }),
            enabled: true,
            removed: false,
            pending: vec![],
            live: Some("active"),
            renamed_from: None,
        }];
        let found = find_self(&rows, exe).unwrap();
        assert!(found.up_to_date);
        assert_eq!(found.name, "mm");
        let moved = find_self(&rows, r"D:\elsewhere\mcp-manager.exe").unwrap();
        assert!(!moved.up_to_date);
    }

    /// apply_pending with Claude Desktop reported as closed.
    fn apply_without_probe(mgr: &mut Manager) -> ApplyReport {
        // SAFETY: every test sets the same value, and nothing in this crate
        // reads the environment from another thread while they run.
        unsafe { std::env::set_var("MCP_MANAGER_FAKE_CLAUDE", "0") };
        mgr.apply_pending("test").unwrap()
    }
}
