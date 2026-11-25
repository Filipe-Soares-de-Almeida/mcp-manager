//! The headless watcher (`mcp-manager.exe --watch`): waits for Claude
//! Desktop to close, applies the queue, and exits once nothing is queued.
//!
//! The MCP server starts it whenever it queues a change. It has to outlive
//! Claude, which ends the processes it started when it quits, so it is
//! started outside Claude's job object (see `launch::spawn_self`).

use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;
use std::time::Duration;

use anyhow::Result;

use crate::claude;
use crate::launch;
use crate::manager::Manager;
use crate::paths;

fn lock_file(data: &Path) -> std::io::Result<File> {
    OpenOptions::new().create(true).truncate(false).write(true).open(paths::watcher_lock(data))
}

/// True when a watcher holds the lock right now.
pub fn is_running(data: &Path) -> bool {
    match lock_file(data) {
        Ok(f) => match f.try_lock() {
            Ok(()) => {
                let _ = f.unlock();
                false
            }
            Err(TryLockError::WouldBlock) => true,
            Err(_) => false,
        },
        Err(_) => false,
    }
}

/// Starts a watcher unless one is already running.
pub fn ensure(data: &Path) {
    if std::env::var_os("MCP_MANAGER_NO_WATCHER").is_some() || is_running(data) {
        return;
    }
    launch::spawn_self(data, &["--watch"], false);
}

pub fn run() -> Result<()> {
    let data = paths::data_dir();
    std::fs::create_dir_all(&data)?;
    let lock = lock_file(&data)?;
    if lock.try_lock().is_err() {
        return Ok(());
    }
    paths::log(&data, "watcher waiting for Claude Desktop to close");
    let mut failures = 0;
    let mut mgr = Manager::open()?;
    loop {
        if mgr.pending_count()? == 0 {
            break;
        }
        if claude::is_running() {
            std::thread::sleep(Duration::from_secs(1));
            continue;
        }
        // Claude writes its config on the way out; let the last process finish.
        std::thread::sleep(Duration::from_millis(1500));
        if claude::is_running() {
            continue;
        }
        match mgr.apply_pending("watcher") {
            Ok(_) => failures = 0,
            Err(e) => {
                failures += 1;
                paths::log(&data, &format!("watcher could not apply the queue: {e:#}"));
                if failures >= 20 {
                    break;
                }
                std::thread::sleep(Duration::from_secs(10));
            }
        }
    }
    paths::log(&data, "watcher done");
    drop(lock);
    Ok(())
}
