//! Changes to the server list, and how they play out on a model of it.
//!
//! The same `Model::apply` runs twice for every change: once on a copy, when
//! the change is submitted, to reject what cannot work and to show the list as
//! it will be; and once for real, when Claude is closed and the queue is
//! written to the config.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub type ServerMap = Map<String, Value>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Op {
    Add {
        name: String,
        spec: ServerMap,
    },
    Update {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        new_name: Option<String>,
        spec: ServerMap,
    },
    Remove {
        name: String,
    },
    Disable {
        name: String,
    },
    Enable {
        name: String,
    },
    Restore {
        backup: String,
        servers: ServerMap,
    },
}

impl Op {
    pub fn kind(&self) -> &'static str {
        match self {
            Op::Add { .. } => "add",
            Op::Update { new_name: Some(n), name, .. } if n != name => "rename",
            Op::Update { .. } => "update",
            Op::Remove { .. } => "remove",
            Op::Disable { .. } => "disable",
            Op::Enable { .. } => "enable",
            Op::Restore { .. } => "restore",
        }
    }

    /// Every server name the change reads or writes.
    pub fn names(&self) -> Vec<String> {
        match self {
            Op::Add { name, .. } | Op::Remove { name } | Op::Disable { name } | Op::Enable { name } => {
                vec![name.clone()]
            }
            Op::Update { name, new_name, .. } => {
                let mut v = vec![name.clone()];
                if let Some(n) = new_name {
                    if n != name {
                        v.push(n.clone());
                    }
                }
                v
            }
            Op::Restore { servers, .. } => servers.keys().cloned().collect(),
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Op::Add { name, .. } => format!("Add \"{name}\""),
            Op::Update { name, new_name: Some(n), .. } if n != name => {
                format!("Edit \"{name}\" and rename it to \"{n}\"")
            }
            Op::Update { name, .. } => format!("Edit \"{name}\""),
            Op::Remove { name } => format!("Remove \"{name}\""),
            Op::Disable { name } => format!("Disable \"{name}\""),
            Op::Enable { name } => format!("Enable \"{name}\""),
            Op::Restore { backup, servers } => {
                format!("Restore {} servers from backup {backup}", servers.len())
            }
        }
    }
}

/// The servers Claude has in its config (`active`) and the ones parked in
/// MCP Manager's database while disabled.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Model {
    pub active: ServerMap,
    pub disabled: ServerMap,
}

impl Model {
    pub fn contains(&self, name: &str) -> bool {
        self.active.contains_key(name) || self.disabled.contains_key(name)
    }

    pub fn apply(&mut self, op: &Op) -> Result<(), String> {
        match op {
            Op::Add { name, spec } => {
                validate_name(name)?;
                validate_spec(spec)?;
                if self.contains(name) {
                    return Err(format!("a server named \"{name}\" already exists"));
                }
                self.active.insert(name.clone(), Value::Object(spec.clone()));
                Ok(())
            }
            Op::Update { name, new_name, spec } => {
                validate_spec(spec)?;
                let target = new_name.clone().unwrap_or_else(|| name.clone());
                if target != *name {
                    validate_name(&target)?;
                    if self.contains(&target) {
                        return Err(format!("a server named \"{target}\" already exists"));
                    }
                }
                let value = Value::Object(spec.clone());
                if self.active.contains_key(name) {
                    replace_in_place(&mut self.active, name, &target, value);
                } else if self.disabled.contains_key(name) {
                    replace_in_place(&mut self.disabled, name, &target, value);
                } else {
                    return Err(format!("no server named \"{name}\""));
                }
                Ok(())
            }
            Op::Remove { name } => {
                if self.active.shift_remove(name).is_none() && self.disabled.shift_remove(name).is_none() {
                    return Err(format!("no server named \"{name}\""));
                }
                Ok(())
            }
            Op::Disable { name } => match self.active.shift_remove(name) {
                Some(spec) => {
                    self.disabled.insert(name.clone(), spec);
                    Ok(())
                }
                None if self.disabled.contains_key(name) => Err(format!("\"{name}\" is already disabled")),
                None => Err(format!("no server named \"{name}\"")),
            },
            Op::Enable { name } => match self.disabled.shift_remove(name) {
                Some(spec) => {
                    self.active.insert(name.clone(), spec);
                    Ok(())
                }
                None if self.active.contains_key(name) => Err(format!("\"{name}\" is already enabled")),
                None => Err(format!("no disabled server named \"{name}\"")),
            },
            Op::Restore { servers, .. } => {
                for (name, spec) in servers {
                    if !spec.is_object() {
                        return Err(format!("backup entry \"{name}\" is not an object"));
                    }
                }
                for name in servers.keys() {
                    self.disabled.shift_remove(name);
                }
                self.active = servers.clone();
                Ok(())
            }
        }
    }
}

fn replace_in_place(map: &mut ServerMap, old: &str, new: &str, value: Value) {
    if old == new {
        map.insert(old.to_string(), value);
        return;
    }
    let entries = std::mem::take(map);
    for (k, v) in entries {
        if k == old {
            map.insert(new.to_string(), value.clone());
        } else {
            map.insert(k, v);
        }
    }
}

pub fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("the server name is empty".into());
    }
    if name != name.trim() {
        return Err("the server name starts or ends with a space".into());
    }
    if name.chars().count() > 64 {
        return Err("the server name is longer than 64 characters".into());
    }
    if let Some(c) = name.chars().find(|c| !(c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ' '))) {
        return Err(format!("the server name has a character Claude cannot use in tool names: '{c}'"));
    }
    Ok(())
}

pub fn validate_spec(spec: &ServerMap) -> Result<(), String> {
    match spec.get("command") {
        Some(Value::String(s)) if !s.trim().is_empty() => {}
        Some(Value::String(_)) | None => {
            return Err("\"command\" is required: Claude Desktop only starts servers that have a command".into());
        }
        Some(_) => return Err("\"command\" must be a string".into()),
    }
    if let Some(args) = spec.get("args") {
        match args {
            Value::Array(items) if items.iter().all(Value::is_string) => {}
            _ => return Err("\"args\" must be a list of strings".into()),
        }
    }
    if let Some(env) = spec.get("env") {
        match env {
            Value::Object(vars) if vars.values().all(Value::is_string) => {}
            _ => return Err("\"env\" must map names to string values".into()),
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Row {
    pub name: String,
    pub spec: Value,
    /// Enabled once the queue is applied.
    pub enabled: bool,
    /// Removed once the queue is applied.
    pub removed: bool,
    /// Kinds of the queued changes that touch this server, in order.
    pub pending: Vec<String>,
    /// Where the server is right now: "active" (in Claude's config),
    /// "disabled" (in the database), or None when only the queue has it.
    pub live: Option<&'static str>,
    pub renamed_from: Option<String>,
}

/// The server list as it will be once `ops` are applied to `real`, and the ids
/// of the ops that no longer apply.
pub fn effective(real: &Model, ops: &[(i64, Op)]) -> (Vec<Row>, Vec<(i64, String)>) {
    let mut sim = real.clone();
    let mut touched: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut renamed: BTreeMap<String, String> = BTreeMap::new();
    let mut invalid = Vec::new();

    for (id, op) in ops {
        let before = sim.clone();
        match sim.apply(op) {
            Ok(()) => {
                let kind = op.kind().to_string();
                match op {
                    Op::Restore { .. } => {
                        let all: BTreeSet<&String> = before
                            .active
                            .keys()
                            .chain(before.disabled.keys())
                            .chain(sim.active.keys())
                            .collect();
                        for name in all {
                            let changed = before.active.get(name) != sim.active.get(name)
                                || before.disabled.get(name) != sim.disabled.get(name);
                            if changed {
                                touched.entry(name.clone()).or_default().push(kind.clone());
                            }
                        }
                    }
                    Op::Update { name, new_name: Some(n), .. } if n != name => {
                        let origin = renamed.remove(name).unwrap_or_else(|| name.clone());
                        let mut kinds = touched.remove(name).unwrap_or_default();
                        kinds.push(kind);
                        touched.insert(n.clone(), kinds);
                        renamed.insert(n.clone(), origin);
                    }
                    _ => {
                        for name in op.names() {
                            touched.entry(name).or_default().push(kind.clone());
                        }
                    }
                }
            }
            Err(e) => invalid.push((*id, e)),
        }
    }

    let live = |name: &str| -> Option<&'static str> {
        if real.active.contains_key(name) {
            Some("active")
        } else if real.disabled.contains_key(name) {
            Some("disabled")
        } else {
            None
        }
    };

    let mut rows = Vec::new();
    for (map, enabled) in [(&sim.active, true), (&sim.disabled, false)] {
        for (name, spec) in map {
            let from = renamed.get(name).cloned();
            rows.push(Row {
                name: name.clone(),
                spec: spec.clone(),
                enabled,
                removed: false,
                pending: touched.get(name).cloned().unwrap_or_default(),
                live: from.as_deref().map_or_else(|| live(name), live),
                renamed_from: from,
            });
        }
    }
    let renamed_away: BTreeSet<&String> = renamed.values().collect();
    for (map, enabled) in [(&real.active, true), (&real.disabled, false)] {
        for (name, spec) in map {
            if sim.contains(name) || renamed_away.contains(name) {
                continue;
            }
            rows.push(Row {
                name: name.clone(),
                spec: spec.clone(),
                enabled,
                removed: true,
                pending: touched.get(name).cloned().unwrap_or_else(|| vec!["remove".into()]),
                live: live(name),
                renamed_from: None,
            });
        }
    }
    rows.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.name.cmp(&b.name)));
    (rows, invalid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec(cmd: &str) -> ServerMap {
        json!({ "command": cmd, "args": ["--x"] }).as_object().unwrap().clone()
    }

    fn model() -> Model {
        let mut m = Model::default();
        m.active.insert("alpha".into(), Value::Object(spec("a.exe")));
        m.active.insert("beta".into(), Value::Object(spec("b.exe")));
        m
    }

    #[test]
    fn add_rejects_duplicate_and_missing_command() {
        let mut m = model();
        assert!(m.apply(&Op::Add { name: "alpha".into(), spec: spec("x") }).is_err());
        let no_cmd = json!({ "args": [] }).as_object().unwrap().clone();
        assert!(m.apply(&Op::Add { name: "gamma".into(), spec: no_cmd }).is_err());
        assert!(m.apply(&Op::Add { name: "gamma".into(), spec: spec("g.exe") }).is_ok());
        assert_eq!(m.active.keys().collect::<Vec<_>>(), ["alpha", "beta", "gamma"]);
    }

    #[test]
    fn disable_then_enable_round_trips_the_spec() {
        let mut m = model();
        let original = m.active["alpha"].clone();
        m.apply(&Op::Disable { name: "alpha".into() }).unwrap();
        assert!(!m.active.contains_key("alpha"));
        assert_eq!(m.disabled["alpha"], original);
        assert!(m.apply(&Op::Disable { name: "alpha".into() }).is_err());
        m.apply(&Op::Enable { name: "alpha".into() }).unwrap();
        assert_eq!(m.active["alpha"], original);
        assert!(m.disabled.is_empty());
    }

    #[test]
    fn rename_keeps_the_position() {
        let mut m = model();
        m.apply(&Op::Update { name: "alpha".into(), new_name: Some("zeta".into()), spec: spec("z.exe") })
            .unwrap();
        assert_eq!(m.active.keys().collect::<Vec<_>>(), ["zeta", "beta"]);
        assert!(m
            .apply(&Op::Update { name: "zeta".into(), new_name: Some("beta".into()), spec: spec("z") })
            .is_err());
    }

    #[test]
    fn update_and_remove_work_on_disabled_servers() {
        let mut m = model();
        m.apply(&Op::Disable { name: "beta".into() }).unwrap();
        m.apply(&Op::Update { name: "beta".into(), new_name: None, spec: spec("b2.exe") }).unwrap();
        assert_eq!(m.disabled["beta"]["command"], "b2.exe");
        m.apply(&Op::Remove { name: "beta".into() }).unwrap();
        assert!(!m.contains("beta"));
    }

    #[test]
    fn effective_marks_pending_and_reports_invalid_ops() {
        let real = model();
        let ops = vec![
            (1, Op::Add { name: "gamma".into(), spec: spec("g") }),
            (2, Op::Disable { name: "alpha".into() }),
            (3, Op::Remove { name: "beta".into() }),
            (4, Op::Enable { name: "nope".into() }),
        ];
        let (rows, invalid) = effective(&real, &ops);
        assert_eq!(invalid.len(), 1);
        assert_eq!(invalid[0].0, 4);
        let by = |n: &str| rows.iter().find(|r| r.name == n).unwrap().clone();
        assert!(!by("alpha").enabled);
        assert_eq!(by("alpha").pending, ["disable"]);
        assert_eq!(by("alpha").live, Some("active"));
        assert!(by("beta").removed);
        assert_eq!(by("gamma").live, None);
        assert_eq!(by("gamma").pending, ["add"]);
    }

    #[test]
    fn effective_shows_a_rename_as_one_row() {
        let real = model();
        let ops = vec![(1, Op::Update { name: "alpha".into(), new_name: Some("omega".into()), spec: spec("o") })];
        let (rows, _) = effective(&real, &ops);
        assert_eq!(rows.len(), 2);
        let omega = rows.iter().find(|r| r.name == "omega").unwrap();
        assert_eq!(omega.renamed_from.as_deref(), Some("alpha"));
        assert_eq!(omega.live, Some("active"));
        assert_eq!(omega.pending, ["rename"]);
    }

    #[test]
    fn names_with_symbols_are_rejected() {
        assert!(validate_name("good-name_1.x").is_ok());
        assert!(validate_name("bad/name").is_err());
        assert!(validate_name(" padded").is_err());
        assert!(validate_name("").is_err());
    }
}
