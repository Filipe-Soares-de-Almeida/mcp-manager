//! Starting this exe in another mode, the Windows startup entry, the Start
//! Menu shortcut, and the lock that tells whether the GUI is running.
//!
//! A process started from `--mcp` would inherit Claude's job object (ended
//! when Claude quits) and its MSIX package context (AppData writes redirected
//! into the package). WMI's Win32_Process.Create starts the new process from
//! the WMI service instead, outside both.

use std::fs::{File, OpenOptions, TryLockError};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::claude::CREATE_NO_WINDOW;
use crate::paths;

const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "MCP Manager";

fn exe() -> Result<PathBuf> {
    std::env::current_exe().context("finding this executable")
}

fn via_wmi(exe: &Path, args: &[&str]) -> Result<()> {
    let mut line = format!("\"{}\"", exe.display());
    for a in args {
        line.push(' ');
        line.push_str(a);
    }
    let script = format!(
        "$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{{CommandLine='{}'}}; exit $r.ReturnValue",
        line.replace('\'', "''")
    );
    let status = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status()?;
    if !status.success() {
        bail!("Win32_Process.Create returned {:?}", status.code());
    }
    Ok(())
}

fn via_breakaway(exe: &Path, args: &[&str]) -> Result<()> {
    Command::new(exe)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB)
        .spawn()?;
    Ok(())
}

/// Starts this exe with `args`, detached from the current process. With
/// `wmi_first`, WMI is tried before a breakaway spawn; otherwise after it.
pub fn spawn_self(data: &Path, args: &[&str], wmi_first: bool) {
    let Ok(exe) = exe() else { return };
    let attempts: [fn(&Path, &[&str]) -> Result<()>; 2] =
        if wmi_first { [via_wmi, via_breakaway] } else { [via_breakaway, via_wmi] };
    for (i, attempt) in attempts.iter().enumerate() {
        match attempt(&exe, args) {
            Ok(()) => {
                paths::log(data, &format!("started {args:?}"));
                return;
            }
            Err(e) => paths::log(data, &format!("start {args:?}, attempt {}: {e:#}", i + 1)),
        }
    }
}

/// Blocks until process `pid` exits, or `timeout` passes.
pub fn wait_for_exit(pid: u32, timeout: std::time::Duration) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
    // SAFETY: the handle is checked for null and closed once waited on.
    unsafe {
        let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            return;
        }
        WaitForSingleObject(handle, timeout.as_millis().min(u32::MAX as u128) as u32);
        CloseHandle(handle);
    }
}

/// Starts this app again from `exe` once the current process exits.
pub fn relaunch_after_update(exe: &Path, from_version: &str) -> Result<()> {
    Command::new(exe)
        .args(["--wait-pid", &std::process::id().to_string(), "--updated-from", from_version])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("starting {}", exe.display()))?;
    Ok(())
}

// ---- GUI lock -----------------------------------------------------------

fn gui_lock_path(data: &Path) -> PathBuf {
    data.join("gui.lock")
}

/// Held by the GUI for as long as it runs.
pub fn hold_gui_lock(data: &Path) -> Option<File> {
    let f = OpenOptions::new().create(true).truncate(false).write(true).open(gui_lock_path(data)).ok()?;
    f.try_lock().ok().map(|()| f)
}

pub fn gui_running(data: &Path) -> bool {
    let Ok(f) = OpenOptions::new().create(true).truncate(false).write(true).open(gui_lock_path(data)) else {
        return false;
    };
    match f.try_lock() {
        Ok(()) => {
            let _ = f.unlock();
            false
        }
        Err(TryLockError::WouldBlock) => true,
        Err(_) => false,
    }
}

// ---- start with Windows -------------------------------------------------

pub fn autostart_enabled() -> bool {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY)
        .and_then(|k| k.get_value::<String, _>(RUN_VALUE))
        .is_ok()
}

pub fn set_autostart(enabled: bool) -> Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN_KEY)?;
    if enabled {
        key.set_value(RUN_VALUE, &format!("\"{}\" --tray", exe()?.display()))?;
    } else {
        match key.delete_value(RUN_VALUE) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

// ---- Start Menu shortcut ------------------------------------------------

pub fn shortcut_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|p| p.join(r"Microsoft\Windows\Start Menu\Programs\MCP Manager.lnk"))
}

pub fn shortcut_exists() -> bool {
    shortcut_path().is_some_and(|p| p.exists())
}

pub fn create_shortcut() -> Result<PathBuf> {
    let exe = exe()?;
    let lnk = shortcut_path().context("APPDATA is not set")?;
    let dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    let q = |p: &Path| p.display().to_string().replace('\'', "''");
    let script = format!(
        "$s = (New-Object -ComObject WScript.Shell).CreateShortcut('{}'); $s.TargetPath = '{}'; $s.WorkingDirectory = '{}'; $s.IconLocation = '{},0'; $s.Description = 'MCP Manager'; $s.Save()",
        q(&lnk),
        q(&exe),
        q(&dir),
        q(&exe)
    );
    let out = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;
    if !out.status.success() || !lnk.exists() {
        bail!("could not create the shortcut: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(lnk)
}
