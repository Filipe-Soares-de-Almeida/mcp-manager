//! `mcp-manager.exe --mcp`: an MCP server on stdio (newline-delimited
//! JSON-RPC 2.0) that lets an agent list, add, edit, remove, disable and
//! enable the MCP servers of Claude Desktop.

use std::io::{BufRead, Write};

use anyhow::{Result, anyhow, bail};
use serde_json::{Map, Value, json};

use crate::launch;
use crate::manager::{Manager, Outcome, QUEUED_NOTE};
use crate::ops::{Op, ServerMap};
use crate::paths;
use crate::watcher;

const PROTOCOL_VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCTIONS: &str = "MCP Manager edits the MCP servers of Claude Desktop (claude_desktop_config.json). \
Claude Desktop rewrites that file while it runs, so a change made while it runs is queued and written as soon as \
Claude is closed: after a change, tell the user it is queued and that Claude must be restarted to load it. \
A disabled server is moved out of the config into MCP Manager's database and put back when enabled. \
Environment values are hidden unless get_server is called with reveal_env.";

/// Starts the app in the tray when it is not running, so its window is one
/// click away whenever Claude is open. `tray_with_claude` turns this off.
fn ensure_tray() {
    if std::env::var_os("MCP_MANAGER_NO_WATCHER").is_some() {
        return;
    }
    let data = paths::data_dir();
    if launch::gui_running(&data) {
        return;
    }
    if Manager::open().is_ok_and(|m| m.tray_with_claude()) {
        launch::spawn_self(&data, &["--tray"], true);
    }
}

pub fn run() -> Result<()> {
    std::thread::spawn(ensure_tray);
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(Value::Array(batch)) => {
                let replies: Vec<Value> = batch.iter().filter_map(handle).collect();
                (!replies.is_empty()).then(|| Value::Array(replies))
            }
            Ok(msg) => handle(&msg),
            Err(e) => Some(error(Value::Null, -32700, &format!("parse error: {e}"))),
        };
        if let Some(reply) = reply {
            writeln!(stdout, "{}", serde_json::to_string(&reply)?)?;
            stdout.flush()?;
        }
    }
    Ok(())
}

fn result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

pub fn handle(msg: &Value) -> Option<Value> {
    let method = msg.get("method")?.as_str()?;
    let id = msg.get("id")?.clone();
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    Some(match method {
        "initialize" => {
            let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or_default();
            let version = PROTOCOL_VERSIONS.iter().find(|v| **v == asked).copied().unwrap_or(PROTOCOL_VERSIONS[0]);
            result(
                id,
                json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "mcp-manager", "title": "MCP Manager", "version": env!("CARGO_PKG_VERSION") },
                    "instructions": INSTRUCTIONS,
                }),
            )
        }
        "ping" => result(id, json!({})),
        "tools/list" => result(id, json!({ "tools": tools() })),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            let (text, is_error) = match call(name, &args) {
                Ok(v) => (serde_json::to_string_pretty(&v).unwrap_or_default(), false),
                Err(e) => (format!("{e:#}"), true),
            };
            result(id, json!({ "content": [{ "type": "text", "text": text }], "isError": is_error }))
        }
        _ => error(id, -32601, &format!("method not found: {method}")),
    })
}

fn name_prop() -> Value {
    json!({ "type": "string", "description": "The server's name: its key under mcpServers." })
}

fn tools() -> Value {
    json!([
        {
            "name": "list_servers",
            "title": "List MCP servers",
            "description": "Lists every MCP server of Claude Desktop: the enabled ones in its config, the disabled ones kept by MCP Manager, and the queued changes still waiting for Claude to close. Environment values are hidden.",
            "inputSchema": { "type": "object", "properties": {} },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "get_server",
            "title": "Show one MCP server",
            "description": "Shows one server's full entry as it will be once the queue is applied.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": name_prop(),
                    "reveal_env": { "type": "boolean", "description": "Show environment values instead of ***. Default false." }
                },
                "required": ["name"]
            },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "add_server",
            "title": "Add an MCP server",
            "description": "Adds a server that Claude Desktop starts with a command. Queued while Claude runs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": name_prop(),
                    "command": { "type": "string", "description": "Executable to run, e.g. npx, uvx, node or a full path to an .exe." },
                    "args": list_prop("Command-line arguments."),
                    "env": map_prop("Environment variables for the server."),
                    "extra": { "type": "object", "description": "Any other fields of the entry, written as given." }
                },
                "required": ["name", "command"]
            }
        },
        {
            "name": "update_server",
            "title": "Edit an MCP server",
            "description": "Edits a server, enabled or disabled. Only the fields given change. env is merged into the existing variables unless replace_env is true; remove_env deletes variables; a null in extra deletes that field. Queued while Claude runs, except for a disabled server, which is saved at once.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": name_prop(),
                    "rename": { "type": "string", "description": "New name for the server." },
                    "command": { "type": "string" },
                    "args": list_prop("Replaces the whole argument list."),
                    "env": map_prop("Variables to set."),
                    "replace_env": { "type": "boolean", "description": "Replace all variables with env instead of merging." },
                    "remove_env": list_prop("Names of variables to delete."),
                    "extra": { "type": "object", "description": "Other fields to set; null deletes one." }
                },
                "required": ["name"]
            }
        },
        {
            "name": "remove_server",
            "title": "Remove an MCP server",
            "description": "Deletes a server from Claude Desktop, or from MCP Manager's database if it is disabled.",
            "inputSchema": { "type": "object", "properties": { "name": name_prop() }, "required": ["name"] },
            "annotations": { "destructiveHint": true }
        },
        {
            "name": "disable_server",
            "title": "Disable an MCP server",
            "description": "Takes a server out of Claude's config and keeps its entry in MCP Manager's database, so enable_server can put it back unchanged.",
            "inputSchema": { "type": "object", "properties": { "name": name_prop() }, "required": ["name"] }
        },
        {
            "name": "enable_server",
            "title": "Enable an MCP server",
            "description": "Puts a disabled server back into Claude's config.",
            "inputSchema": { "type": "object", "properties": { "name": name_prop() }, "required": ["name"] }
        },
        {
            "name": "list_changes",
            "title": "List queued and applied changes",
            "description": "Shows the changes waiting for Claude Desktop to close, and the latest applied ones.",
            "inputSchema": { "type": "object", "properties": {} },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "open_manager",
            "title": "Open the MCP Manager window",
            "description": "Shows the MCP Manager window on the user's desktop, where they can review and edit the servers themselves.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "cancel_change",
            "title": "Cancel a queued change",
            "description": "Drops a queued change by id (and later changes that depended on it), or every queued change with all=true.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer" },
                    "all": { "type": "boolean" }
                }
            }
        }
    ])
}

fn list_prop(description: &str) -> Value {
    json!({ "type": "array", "items": { "type": "string" }, "description": description })
}

fn map_prop(description: &str) -> Value {
    json!({ "type": "object", "additionalProperties": { "type": "string" }, "description": description })
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

fn need_name(args: &Value) -> Result<String> {
    str_arg(args, "name").map(str::to_string).ok_or_else(|| anyhow!("\"name\" is required"))
}

fn string_list(v: &Value, key: &str) -> Result<Vec<Value>> {
    match v {
        Value::Array(items) if items.iter().all(Value::is_string) => Ok(items.clone()),
        _ => bail!("\"{key}\" must be a list of strings"),
    }
}

fn string_map(v: &Value, key: &str) -> Result<Map<String, Value>> {
    match v {
        Value::Object(m) if m.values().all(Value::is_string) => Ok(m.clone()),
        _ => bail!("\"{key}\" must map names to string values"),
    }
}

fn masked(spec: &ServerMap, reveal: bool) -> Value {
    let mut out = spec.clone();
    if !reveal {
        if let Some(Value::Object(env)) = out.get_mut("env") {
            for v in env.values_mut() {
                *v = json!("***");
            }
        }
    }
    Value::Object(out)
}

fn outcome(mgr: &Manager, o: Outcome) -> Value {
    if o.queued_id.is_some() {
        watcher::ensure(&mgr.data);
    }
    json!({
        "status": if o.applied { "applied" } else { "queued" },
        "queued_change_id": o.queued_id,
        "claude_running": o.claude_running,
        "restart_needed": o.queued_id.is_some(),
        "message": o.message,
    })
}

fn call(tool: &str, args: &Value) -> Result<Value> {
    let mut mgr = Manager::open()?;
    match tool {
        "list_servers" => {
            let snap = mgr.snapshot()?;
            let servers: Vec<Value> = snap
                .rows
                .iter()
                .map(|r| {
                    let spec = r.spec.as_object().cloned().unwrap_or_default();
                    let env_keys: Vec<&String> =
                        spec.get("env").and_then(Value::as_object).map(|e| e.keys().collect()).unwrap_or_default();
                    let other: Vec<&String> =
                        spec.keys().filter(|k| !matches!(k.as_str(), "command" | "args" | "env")).collect();
                    json!({
                        "name": r.name,
                        "state": if r.removed { "removed when the queue is applied" } else if r.enabled { "enabled" } else { "disabled" },
                        "in_claude_config_now": r.live == Some("active"),
                        "queued": r.pending,
                        "renamed_from": r.renamed_from,
                        "command": spec.get("command"),
                        "args": spec.get("args"),
                        "env_keys": env_keys,
                        "other_fields": other,
                    })
                })
                .collect();
            let queued: Vec<Value> = snap
                .pending
                .iter()
                .map(|p| json!({ "id": p.id, "change": p.description, "source": p.source, "queued_at": p.created_at }))
                .collect();
            Ok(json!({
                "claude_running": snap.claude.running,
                "config_path": snap.config_path,
                "servers": servers,
                "queued_changes": queued,
                "note": if queued.is_empty() { Value::Null } else { json!(QUEUED_NOTE) },
            }))
        }
        "get_server" => {
            let name = need_name(args)?;
            let (spec, enabled) = mgr.effective_spec(&name)?;
            let reveal = args.get("reveal_env").and_then(Value::as_bool).unwrap_or(false);
            Ok(json!({ "name": name, "enabled": enabled, "entry": masked(&spec, reveal) }))
        }
        "add_server" => {
            let name = need_name(args)?;
            let mut spec = match args.get("extra") {
                Some(Value::Object(m)) => m.clone(),
                Some(Value::Null) | None => ServerMap::new(),
                Some(_) => bail!("\"extra\" must be an object"),
            };
            let command = str_arg(args, "command").ok_or_else(|| anyhow!("\"command\" is required"))?;
            spec.insert("command".into(), json!(command));
            if let Some(a) = args.get("args").filter(|v| !v.is_null()) {
                spec.insert("args".into(), Value::Array(string_list(a, "args")?));
            }
            if let Some(e) = args.get("env").filter(|v| !v.is_null()) {
                let env = string_map(e, "env")?;
                if !env.is_empty() {
                    spec.insert("env".into(), Value::Object(env));
                }
            }
            let o = mgr.submit("mcp", Op::Add { name, spec })?;
            Ok(outcome(&mgr, o))
        }
        "update_server" => {
            let name = need_name(args)?;
            let (mut spec, _) = mgr.effective_spec(&name)?;
            if let Some(c) = str_arg(args, "command") {
                spec.insert("command".into(), json!(c));
            }
            if let Some(a) = args.get("args").filter(|v| !v.is_null()) {
                spec.insert("args".into(), Value::Array(string_list(a, "args")?));
            }
            let replace = args.get("replace_env").and_then(Value::as_bool).unwrap_or(false);
            let mut env = if replace {
                Map::new()
            } else {
                spec.get("env").and_then(Value::as_object).cloned().unwrap_or_default()
            };
            if let Some(e) = args.get("env").filter(|v| !v.is_null()) {
                for (k, v) in string_map(e, "env")? {
                    env.insert(k, v);
                }
            }
            if let Some(r) = args.get("remove_env").filter(|v| !v.is_null()) {
                for k in string_list(r, "remove_env")? {
                    env.shift_remove(k.as_str().unwrap_or_default());
                }
            }
            if env.is_empty() {
                spec.shift_remove("env");
            } else {
                spec.insert("env".into(), Value::Object(env));
            }
            if let Some(extra) = args.get("extra").filter(|v| !v.is_null()) {
                let Value::Object(extra) = extra else { bail!("\"extra\" must be an object") };
                for (k, v) in extra {
                    if v.is_null() {
                        spec.shift_remove(k);
                    } else {
                        spec.insert(k.clone(), v.clone());
                    }
                }
            }
            let new_name = str_arg(args, "rename").map(str::to_string).filter(|n| *n != name);
            let o = mgr.submit("mcp", Op::Update { name, new_name, spec })?;
            Ok(outcome(&mgr, o))
        }
        "remove_server" => {
            let o = mgr.submit("mcp", Op::Remove { name: need_name(args)? })?;
            Ok(outcome(&mgr, o))
        }
        "disable_server" => {
            let o = mgr.submit("mcp", Op::Disable { name: need_name(args)? })?;
            Ok(outcome(&mgr, o))
        }
        "enable_server" => {
            let o = mgr.submit("mcp", Op::Enable { name: need_name(args)? })?;
            Ok(outcome(&mgr, o))
        }
        "list_changes" => {
            let queued: Vec<Value> = mgr
                .pending()?
                .iter()
                .map(|p| json!({ "id": p.id, "change": p.description, "source": p.source, "queued_at": p.created_at }))
                .collect();
            let applied: Vec<Value> = mgr
                .history(15)?
                .iter()
                .map(|h| json!({ "change": h.description, "applied_at": h.applied_at, "ok": h.ok, "error": h.message }))
                .collect();
            Ok(json!({ "queued": queued, "recently_applied": applied }))
        }
        "open_manager" => {
            launch::spawn_self(&mgr.data, &[], true);
            Ok(json!({ "message": "The MCP Manager window was opened on the user's desktop." }))
        }
        "cancel_change" => {
            if args.get("all").and_then(Value::as_bool).unwrap_or(false) {
                let n = mgr.discard_all()?;
                return Ok(json!({ "cancelled": n }));
            }
            let id = args.get("id").and_then(Value::as_i64).ok_or_else(|| anyhow!("give \"id\" or all=true"))?;
            let dropped = mgr.cancel(id)?;
            Ok(json!({ "cancelled": id, "also_dropped_because_they_depended_on_it": dropped }))
        }
        other => bail!("unknown tool: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_echoes_a_known_version_and_skips_notifications() {
        let init = json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-06-18" } });
        let reply = handle(&init).unwrap();
        assert_eq!(reply["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(reply["id"], 1);
        let note = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert!(handle(&note).is_none());
        let unknown = json!({ "jsonrpc": "2.0", "id": "x", "method": "resources/list" });
        assert_eq!(handle(&unknown).unwrap()["error"]["code"], -32601);
    }

    #[test]
    fn tool_schemas_are_objects_with_names() {
        for t in tools().as_array().unwrap() {
            assert!(t["name"].is_string());
            assert_eq!(t["inputSchema"]["type"], "object");
        }
    }
}
