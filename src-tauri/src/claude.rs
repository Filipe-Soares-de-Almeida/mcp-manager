//! Finding Claude Desktop: its executable, its config file, whether it runs,
//! and closing and starting it.
//!
//! The executable decides the config path. The MSIX build lives in
//! `WindowsApps\<Name>_<Version>_<Arch>__<PublisherId>\app\Claude.exe` and keeps
//! its data in `%LOCALAPPDATA%\Claude-Data\<lowercase package family>`; older
//! MSIX builds used the package's `LocalCache\Roaming\Claude`, and the Squirrel
//! installer uses `%APPDATA%\Claude`.
//!
//! The Claude Code CLI is also named `claude.exe`, both on PATH and bundled
//! under `Claude-Data\...\claude-code`, so processes are told apart by path.

use std::os::windows::ffi::OsStringExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use serde::Serialize;

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DEFAULT_FAMILY: &str = "Claude_pzs8sxrjxfjjc";
const CONFIG_FILE: &str = "claude_desktop_config.json";

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InstallKind {
    Msix { family: String, app_id: String, package_dir: PathBuf, version: String },
    Squirrel,
    Unknown,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Install {
    pub exe: PathBuf,
    #[serde(flatten)]
    pub kind: InstallKind,
    /// How the executable was found: "running", "remembered", "registry", "squirrel".
    pub found_by: &'static str,
}

pub fn is_desktop_exe(path: &Path) -> bool {
    let p = path.to_string_lossy().to_lowercase().replace('/', "\\");
    if !p.ends_with("\\claude.exe") || p.contains("\\claude-code") || p.contains("\\.local\\bin\\") {
        return false;
    }
    (p.contains("\\windowsapps\\") && p.ends_with("\\app\\claude.exe")) || p.contains("\\anthropicclaude\\")
}

/// Splits a package folder name such as `Claude_6.28899.6.0_x64__pzs8sxrjxfjjc`
/// into (family name, version).
pub fn parse_package_folder(folder: &str) -> Option<(String, String)> {
    let (left, publisher) = folder.split_once("__")?;
    let mut parts = left.split('_');
    let name = parts.next()?;
    let version = parts.next().unwrap_or_default();
    if name.is_empty() || publisher.is_empty() {
        return None;
    }
    Some((format!("{name}_{publisher}"), version.to_string()))
}

fn manifest_app_id(package_dir: &Path) -> Option<String> {
    let xml = std::fs::read_to_string(package_dir.join("AppxManifest.xml")).ok()?;
    for tag in xml.split("<Application ").skip(1) {
        let head = tag.split('>').next().unwrap_or_default();
        let attr = |key: &str| {
            let needle = format!("{key}=\"");
            head.find(&needle).map(|i| {
                let rest = &head[i + needle.len()..];
                rest[..rest.find('"').unwrap_or(rest.len())].to_string()
            })
        };
        if attr("Executable").is_some_and(|e| e.to_lowercase().ends_with("claude.exe")) {
            return attr("Id");
        }
    }
    None
}

pub fn install_from_exe(exe: &Path, found_by: &'static str) -> Install {
    let lower = exe.to_string_lossy().to_lowercase();
    let kind = if lower.contains("\\windowsapps\\") {
        let package_dir = exe.parent().and_then(Path::parent).map(Path::to_path_buf);
        match package_dir.as_ref().and_then(|d| {
            let folder = d.file_name()?.to_string_lossy().into_owned();
            parse_package_folder(&folder).map(|(f, v)| (d.clone(), f, v))
        }) {
            Some((dir, family, version)) => InstallKind::Msix {
                app_id: manifest_app_id(&dir).unwrap_or_else(|| "Claude".into()),
                family,
                package_dir: dir,
                version,
            },
            None => InstallKind::Unknown,
        }
    } else if lower.contains("\\anthropicclaude\\") {
        InstallKind::Squirrel
    } else {
        InstallKind::Unknown
    };
    Install { exe: exe.to_path_buf(), kind, found_by }
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key).map(PathBuf::from)
}

/// Every place this install may keep its config, most likely first.
pub fn config_candidates(install: Option<&Install>) -> Vec<PathBuf> {
    let local = env_path("LOCALAPPDATA");
    let roaming = env_path("APPDATA");
    let mut out = Vec::new();
    let msix = |family: &str, out: &mut Vec<PathBuf>| {
        if let Some(l) = &local {
            out.push(l.join("Claude-Data").join(family.to_lowercase()).join(CONFIG_FILE));
            out.push(l.join("Packages").join(family).join("LocalCache").join("Roaming").join("Claude").join(CONFIG_FILE));
        }
    };
    match install.map(|i| &i.kind) {
        Some(InstallKind::Msix { family, .. }) => msix(family, &mut out),
        Some(InstallKind::Squirrel) => {}
        _ => msix(DEFAULT_FAMILY, &mut out),
    }
    if let Some(r) = &roaming {
        out.push(r.join("Claude").join(CONFIG_FILE));
    }
    out
}

/// The config this install reads: the first candidate that exists, else the first.
pub fn config_path(install: Option<&Install>) -> PathBuf {
    let candidates = config_candidates(install);
    candidates
        .iter()
        .find(|p| p.exists())
        .or(candidates.first())
        .cloned()
        .unwrap_or_else(|| PathBuf::from(CONFIG_FILE))
}

// ---- processes ---------------------------------------------------------

fn process_path(pid: u32) -> Option<PathBuf> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return None;
        }
        let mut buf = vec![0u16; 2048];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len);
        CloseHandle(h);
        (ok != 0).then(|| PathBuf::from(std::ffi::OsString::from_wide(&buf[..len as usize])))
    }
}

/// Processes whose image is `exe_name`, with their full path where Windows gives it.
pub fn processes_named(exe_name: &str) -> Vec<(u32, Option<PathBuf>)> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
    };
    let mut out = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return out;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
                if name.eq_ignore_ascii_case(exe_name) {
                    out.push((entry.th32ProcessID, process_path(entry.th32ProcessID)));
                }
                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
    out
}

/// The running Claude Desktop processes (pid, exe).
pub fn running() -> Vec<(u32, PathBuf)> {
    if let Ok(v) = std::env::var("MCP_MANAGER_FAKE_CLAUDE") {
        return if v == "1" { vec![(0, PathBuf::from("fake"))] } else { Vec::new() };
    }
    processes_named("claude.exe")
        .into_iter()
        .filter_map(|(pid, path)| path.filter(|p| is_desktop_exe(p)).map(|p| (pid, p)))
        .collect()
}

pub fn is_running() -> bool {
    !running().is_empty()
}

// ---- finding the install -----------------------------------------------

fn version_key(v: &str) -> Vec<u64> {
    v.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

fn registry_msix() -> Option<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;
    let packages = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages")
        .ok()?;
    let mut found: Vec<(Vec<u64>, PathBuf)> = Vec::new();
    for name in packages.enum_keys().flatten() {
        let lower = name.to_lowercase();
        if !(lower.starts_with("claude_") || lower.starts_with("anthropicpbc.claude_")) {
            continue;
        }
        let Some((_, version)) = parse_package_folder(&name) else { continue };
        let Ok(key) = packages.open_subkey(&name) else { continue };
        let Ok(root) = key.get_value::<String, _>("PackageRootFolder") else { continue };
        let exe = PathBuf::from(root).join("app").join("Claude.exe");
        if exe.exists() {
            found.push((version_key(&version), exe));
        }
    }
    found.sort();
    found.pop().map(|(_, exe)| exe)
}

/// Finds Claude Desktop from its executable: the running process first (the
/// version actually in use), then the one seen last time, then the newest
/// MSIX package registered for this user, then a Squirrel install.
pub fn detect(remembered: Option<&Path>) -> Option<Install> {
    if let Some((_, exe)) = running().into_iter().find(|(_, p)| p.exists()) {
        return Some(install_from_exe(&exe, "running"));
    }
    if let Some(exe) = remembered.filter(|p| p.exists() && is_desktop_exe(p)) {
        return Some(install_from_exe(exe, "remembered"));
    }
    if let Some(exe) = registry_msix() {
        return Some(install_from_exe(&exe, "registry"));
    }
    let squirrel = env_path("LOCALAPPDATA")?.join("AnthropicClaude").join("claude.exe");
    squirrel.exists().then(|| install_from_exe(&squirrel, "squirrel"))
}

// ---- closing and starting ----------------------------------------------

fn wait_closed(timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if !is_running() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    !is_running()
}

fn taskkill(pid: u32, force: bool) {
    let mut cmd = Command::new("taskkill");
    if force {
        cmd.args(["/F", "/T"]);
    }
    let _ = cmd.args(["/PID", &pid.to_string()]).creation_flags(CREATE_NO_WINDOW).output();
}

/// Asks every Claude Desktop process to close, then ends the ones still
/// running after `grace`. Returns once none is left.
pub fn close(grace: Duration) -> Result<()> {
    let pids: Vec<u32> = running().into_iter().map(|(p, _)| p).collect();
    if pids.is_empty() {
        return Ok(());
    }
    for pid in &pids {
        taskkill(*pid, false);
    }
    if wait_closed(grace) {
        return Ok(());
    }
    for (pid, _) in running() {
        taskkill(pid, true);
    }
    if wait_closed(Duration::from_secs(15)) {
        Ok(())
    } else {
        bail!("Claude Desktop is still running after being asked to close")
    }
}

pub fn launch(install: &Install) -> Result<()> {
    match &install.kind {
        InstallKind::Msix { family, app_id, .. } => {
            Command::new("explorer.exe").arg(format!("shell:AppsFolder\\{family}!{app_id}")).spawn()?;
        }
        _ => {
            Command::new(&install.exe).creation_flags(0x0000_0008).spawn()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_the_desktop_app_from_the_cli() {
        let app = Path::new(r"C:\Program Files\WindowsApps\Claude_6.28899.6.0_x64__pzs8sxrjxfjjc\app\Claude.exe");
        let bundled =
            Path::new(r"C:\Users\u\AppData\Local\Claude-Data\claude_pzs8sxrjxfjjc\claude-code\2.1.330\abc\claude.exe");
        let cli = Path::new(r"C:\Users\u\.local\bin\claude.exe");
        let squirrel = Path::new(r"C:\Users\u\AppData\Local\AnthropicClaude\app-0.9.0\claude.exe");
        assert!(is_desktop_exe(app));
        assert!(is_desktop_exe(squirrel));
        assert!(!is_desktop_exe(bundled));
        assert!(!is_desktop_exe(cli));
    }

    #[test]
    fn parses_the_package_folder() {
        assert_eq!(
            parse_package_folder("Claude_6.28899.6.0_x64__pzs8sxrjxfjjc"),
            Some(("Claude_pzs8sxrjxfjjc".into(), "6.28899.6.0".into()))
        );
        assert_eq!(
            parse_package_folder("AnthropicPBC.Claude_1.2.3.0_x64__fnn82j28hfe8t"),
            Some(("AnthropicPBC.Claude_fnn82j28hfe8t".into(), "1.2.3.0".into()))
        );
        assert_eq!(parse_package_folder("Claude"), None);
    }

    #[test]
    fn msix_config_lives_in_claude_data_first() {
        let install = Install {
            exe: PathBuf::from(r"C:\x\app\Claude.exe"),
            kind: InstallKind::Msix {
                family: "Claude_pzs8sxrjxfjjc".into(),
                app_id: "Claude".into(),
                package_dir: PathBuf::from(r"C:\x"),
                version: "1".into(),
            },
            found_by: "running",
        };
        let c = config_candidates(Some(&install));
        let first = c[0].to_string_lossy().to_lowercase();
        assert!(first.ends_with(r"claude-data\claude_pzs8sxrjxfjjc\claude_desktop_config.json"), "{first}");
        assert!(c.iter().any(|p| p.to_string_lossy().contains(r"Packages\Claude_pzs8sxrjxfjjc\LocalCache")));
    }
}
