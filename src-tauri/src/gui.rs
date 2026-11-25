//! The desktop window and its tray icon: Tauri commands over `Manager`, and a
//! monitor thread that applies the queue when Claude Desktop closes and tells
//! the page when anything it shows has changed.
//!
//! The app lives in the notification area: closing the window hides it, the
//! tray's Sair quits. `--tray` starts it with the window hidden, which is how
//! Windows startup and the MCP server launch it.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager as _, State, WindowEvent};

use crate::backup::{self, BackupInfo};
use crate::claude;
use crate::launch;
use crate::locale;
use crate::manager::{ApplyReport, Manager, Outcome, Snapshot};
use crate::ops::{Op, ServerMap};
use crate::paths;
use crate::update::{self, Check, Feed, Release};
use crate::watcher;

const TRAY_ID: &str = "main";

struct AppState {
    mgr: Mutex<Manager>,
    restarting: Arc<AtomicBool>,
    /// The feed of the last check that found a newer version.
    feed: Mutex<Option<Feed>>,
    updating: AtomicBool,
    /// The version this start replaced, when it is the first start after an update.
    updated_from: Mutex<Option<String>>,
}

type Res<T> = Result<T, String>;

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

fn with<T>(state: &State<'_, AppState>, f: impl FnOnce(&mut Manager) -> anyhow::Result<T>) -> Res<T> {
    let mut mgr = state.mgr.lock().map_err(|_| "the manager is unavailable".to_string())?;
    f(&mut mgr).map_err(err)
}

fn submit(state: &State<'_, AppState>, op: Op) -> Res<Outcome> {
    with(state, |m| {
        let o = m.submit("app", op)?;
        if o.queued_id.is_some() {
            watcher::ensure(&m.data);
        }
        Ok(o)
    })
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[derive(Clone, Serialize)]
struct RestartDone {
    ok: bool,
    report: Option<ApplyReport>,
    error: Option<String>,
}

/// Closes Claude, applies the queue and opens Claude again; the page hears
/// every step and the result, whether the window or the tray asked.
fn restart_blocking(app: &AppHandle) -> Res<()> {
    let state = app.state::<AppState>();
    if state.restarting.swap(true, Ordering::SeqCst) {
        return Err("a restart is already in progress".into());
    }
    let result = Manager::open().and_then(|mut m| {
        m.restart_claude(&|step| {
            let _ = app.emit("restart-progress", step);
        })
    });
    state.restarting.store(false, Ordering::SeqCst);
    let done = match result {
        Ok(report) => RestartDone { ok: true, report: Some(report), error: None },
        Err(e) => RestartDone { ok: false, report: None, error: Some(err(e)) },
    };
    let _ = app.emit("restart-done", done.clone());
    let _ = app.emit("state-changed", ());
    match done.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

#[tauri::command]
async fn get_state(state: State<'_, AppState>) -> Res<Snapshot> {
    with(&state, |m| m.snapshot())
}

#[tauri::command]
async fn add_server(state: State<'_, AppState>, name: String, spec: ServerMap) -> Res<Outcome> {
    submit(&state, Op::Add { name, spec })
}

#[tauri::command]
async fn update_server(
    state: State<'_, AppState>,
    name: String,
    new_name: Option<String>,
    spec: ServerMap,
) -> Res<Outcome> {
    let new_name = new_name.filter(|n| *n != name);
    submit(&state, Op::Update { name, new_name, spec })
}

#[tauri::command]
async fn remove_server(state: State<'_, AppState>, name: String) -> Res<Outcome> {
    submit(&state, Op::Remove { name })
}

#[tauri::command]
async fn set_enabled(state: State<'_, AppState>, name: String, enabled: bool) -> Res<Outcome> {
    submit(&state, if enabled { Op::Enable { name } } else { Op::Disable { name } })
}

#[tauri::command]
async fn cancel_change(state: State<'_, AppState>, id: i64) -> Res<Vec<i64>> {
    with(&state, |m| m.cancel(id))
}

#[tauri::command]
async fn discard_changes(state: State<'_, AppState>) -> Res<usize> {
    with(&state, |m| m.discard_all())
}

#[tauri::command]
async fn apply_now(state: State<'_, AppState>) -> Res<ApplyReport> {
    with(&state, |m| m.apply_pending("app"))
}

#[tauri::command]
async fn restart_claude(app: AppHandle) -> Res<()> {
    tauri::async_runtime::spawn_blocking(move || restart_blocking(&app)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn list_backups(state: State<'_, AppState>) -> Res<Vec<BackupInfo>> {
    with(&state, |m| backup::list(&m.backups_dir()))
}

#[tauri::command]
async fn backup_servers(state: State<'_, AppState>, file: String) -> Res<ServerMap> {
    with(&state, |m| backup::read_servers(&m.backups_dir(), &file))
}

#[tauri::command]
async fn restore_backup(state: State<'_, AppState>, file: String) -> Res<Outcome> {
    let servers = with(&state, |m| backup::read_servers(&m.backups_dir(), &file))?;
    submit(&state, Op::Restore { backup: file, servers })
}

#[tauri::command]
async fn backup_now(state: State<'_, AppState>) -> Res<Option<String>> {
    with(&state, |m| m.backup_now())
}

#[tauri::command]
async fn register_self(state: State<'_, AppState>) -> Res<Outcome> {
    with(&state, |m| {
        let o = m.register_self("app")?;
        if o.queued_id.is_some() {
            watcher::ensure(&m.data);
        }
        Ok(o)
    })
}

#[tauri::command]
async fn self_spec() -> Res<Value> {
    Manager::self_spec().map(Value::Object).map_err(err)
}

#[tauri::command]
async fn set_config_override(state: State<'_, AppState>, path: Option<String>) -> Res<()> {
    with(&state, |m| m.set_config_override(path.as_deref()))
}

#[tauri::command]
async fn set_autostart(enabled: bool) -> Res<()> {
    launch::set_autostart(enabled).map_err(err)
}

#[tauri::command]
async fn set_tray_with_claude(state: State<'_, AppState>, enabled: bool) -> Res<()> {
    with(&state, |m| m.set_tray_with_claude(enabled))
}

#[tauri::command]
async fn set_locale(app: AppHandle, state: State<'_, AppState>, locale: String) -> Res<()> {
    let (lang, pending) = with(&state, |m| {
        m.set_locale(&locale)?;
        Ok((m.locale(), m.pending_count()?))
    })?;
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_menu(Some(tray_menu(&app, lang).map_err(|e| e.to_string())?));
        let _ = tray.set_tooltip(Some(locale::tray_tooltip(lang, pending)));
    }
    Ok(())
}

#[tauri::command]
async fn set_check_updates(state: State<'_, AppState>, enabled: bool) -> Res<()> {
    with(&state, |m| m.set_check_updates(enabled))
}

#[tauri::command]
async fn release_notes(lang: String) -> Res<Vec<Release>> {
    Ok(update::release_notes(&lang))
}

#[tauri::command]
async fn check_update(state: State<'_, AppState>, lang: String) -> Res<Check> {
    let (check, feed) = tauri::async_runtime::spawn_blocking(move || update::check(&lang))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)?;
    *state.feed.lock().map_err(|_| "update state unavailable")? = check.available.then_some(feed);
    Ok(check)
}

#[derive(Clone, Serialize)]
struct UpdateProgress {
    done: u64,
    total: Option<u64>,
}

/// Downloads and installs the version the last check found, then restarts
/// the app from it. Returns only on failure.
#[tauri::command]
async fn install_update(app: AppHandle, state: State<'_, AppState>) -> Res<()> {
    let feed = state
        .feed
        .lock()
        .map_err(|_| "update state unavailable")?
        .clone()
        .ok_or("no update was found to install")?;
    if state.updating.swap(true, Ordering::SeqCst) {
        return Err("an update is already being installed".into());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let progress_app = app.clone();
    let target = exe.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        update::install(&feed, &target, &|done, total| {
            let _ = progress_app.emit("update-progress", UpdateProgress { done, total });
        })
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|r| r.map_err(err));
    if let Err(e) = result {
        state.updating.store(false, Ordering::SeqCst);
        paths::log(&paths::data_dir(), &format!("update failed: {e}"));
        return Err(e);
    }
    paths::log(&paths::data_dir(), &format!("updated from {}; restarting", update::VERSION));
    launch::relaunch_after_update(&exe, update::VERSION).map_err(err)?;
    app.exit(0);
    Ok(())
}

/// The version replaced by the update that started this process, once.
#[tauri::command]
async fn take_updated_from(state: State<'_, AppState>) -> Res<Option<String>> {
    Ok(state.updated_from.lock().map_err(|_| "unavailable")?.take())
}

#[tauri::command]
async fn create_shortcut() -> Res<String> {
    launch::create_shortcut().map(|p| p.to_string_lossy().into_owned()).map_err(err)
}

#[tauri::command]
async fn open_path(path: String) -> Res<()> {
    let p = std::path::PathBuf::from(&path);
    let mut cmd = std::process::Command::new("explorer.exe");
    if p.is_file() {
        cmd.arg(format!("/select,{}", p.display()));
    } else {
        cmd.arg(&p);
    }
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

fn tray_menu(app: &AppHandle, lang: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let text = locale::tray(lang);
    let show = MenuItem::with_id(app, "show", text.open, true, None::<&str>)?;
    let restart = MenuItem::with_id(app, "restart", text.restart, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", text.quit, true, None::<&str>)?;
    Menu::with_items(app, &[&show, &restart, &quit])
}

fn build_tray(app: &AppHandle, lang: &str) -> tauri::Result<()> {
    let menu = tray_menu(app, lang)?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("MCP Manager")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "restart" => {
                let app = app.clone();
                std::thread::spawn(move || {
                    let _ = restart_blocking(&app);
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn spawn_monitor(app: AppHandle, restarting: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let Ok(mut mgr) = Manager::open() else { return };
        let _ = mgr.backup_if_changed("startup");
        let mut last = String::new();
        // The same failure repeats on every pass until something changes;
        // the page hears about it once.
        let mut last_error = String::new();
        loop {
            let running = claude::is_running();
            if !running && !restarting.load(Ordering::SeqCst) && mgr.pending_count().unwrap_or(0) > 0 {
                match mgr.apply_pending("app") {
                    Ok(r) => {
                        last_error.clear();
                        if r.applied + r.failed.len() > 0 {
                            let _ = app.emit("applied", &r);
                        }
                    }
                    Err(e) => {
                        let msg = format!("{e:#}");
                        if msg != last_error {
                            let _ = app.emit("apply-error", &msg);
                            last_error = msg;
                        }
                    }
                }
            }
            let fp = mgr.fingerprint(running);
            if fp != last {
                last = fp;
                let _ = app.emit("state-changed", ());
                if let Some(tray) = app.tray_by_id(TRAY_ID) {
                    let tip = locale::tray_tooltip(mgr.locale(), mgr.pending_count().unwrap_or(0));
                    let _ = tray.set_tooltip(Some(tip));
                }
            }
            std::thread::sleep(Duration::from_millis(1500));
        }
    });
}

pub fn run(updated_from: Option<String>) {
    let mgr = match Manager::open() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("mcp-manager: {e:#}");
            std::process::exit(1);
        }
    };
    if let Ok(exe) = std::env::current_exe() {
        update::clean_old(&exe);
    }
    let start_hidden = std::env::args().any(|a| a == "--tray");
    let gui_lock = launch::hold_gui_lock(&paths::data_dir());
    let restarting = Arc::new(AtomicBool::new(false));
    let monitor_flag = restarting.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // A second `--tray` start (Windows startup, the MCP server) only
            // makes sure the app runs; anything else asks for the window.
            if !args.iter().any(|a| a == "--tray") {
                show_main(app);
            }
        }))
        .manage(AppState {
            mgr: Mutex::new(mgr),
            restarting,
            feed: Mutex::new(None),
            updating: AtomicBool::new(false),
            updated_from: Mutex::new(updated_from),
        })
        .setup(move |app| {
            let lang = app.state::<AppState>().mgr.lock().map(|m| m.locale()).unwrap_or("en-US");
            build_tray(app.handle(), lang)?;
            if !start_hidden {
                show_main(app.handle());
            }
            spawn_monitor(app.handle().clone(), monitor_flag);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            add_server,
            update_server,
            remove_server,
            set_enabled,
            cancel_change,
            discard_changes,
            apply_now,
            restart_claude,
            list_backups,
            backup_servers,
            restore_backup,
            backup_now,
            register_self,
            self_spec,
            set_config_override,
            set_autostart,
            set_tray_with_claude,
            set_locale,
            set_check_updates,
            release_notes,
            check_update,
            install_update,
            take_updated_from,
            create_shortcut,
            open_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running MCP Manager");
    drop(gui_lock);
}
