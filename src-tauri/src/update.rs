//! Versions, release notes and the self-update.
//!
//! The notes come from CHANGELOG.md (Keep a Changelog) and its pt-BR copy,
//! built into the exe. A release publishes `latest.json` with the new
//! version, the exe's URL, its minisign signature and the whole changelog in
//! each language, so the app can list every version it has not installed.
//!
//! The update swaps the exe in place instead of running an installer: Claude
//! keeps `mcp-manager.exe --mcp` running, and Windows refuses to overwrite a
//! running exe but lets it be renamed. The new exe is checked against the
//! public key below, the running one becomes `mcp-manager.old.exe`, the new
//! one takes its name, and the app restarts from it. The MCP processes keep
//! the old image until Claude restarts; the next start deletes the old file.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use serde::{Deserialize, Serialize};

const CHANGELOG: &str = include_str!("../../CHANGELOG.md");
const CHANGELOG_PT_BR: &str = include_str!("../../locales/changelog/pt-BR.md");
const FEED_URL: &str = "https://github.com/Filipe-Soares-de-Almeida/mcp-manager/releases/latest/download/latest.json";
/// The minisign public key releases are signed with, as `tauri signer` writes it.
const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDBEQ0RDRDdEQjI5Mzg5ODMKUldTRGlaT3lmYzNORGVnL042c0cyVXVwZ2ZEWjNWZ2lFcDB3MWRKRTBUcXVNVFZOMTRneFZFbWQK";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// One heading of a version's notes ("Added", "Fixed") and its entries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Group {
    pub kind: String,
    pub items: Vec<String>,
}

/// A released version and what changed in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Release {
    pub version: String,
    pub date: Option<String>,
    /// The paragraph between the version's heading and its first group.
    pub summary: Option<String>,
    pub groups: Vec<Group>,
}

/// Every released section of a Keep a Changelog text, in file order; Unreleased is left out.
pub fn releases(text: &str) -> Vec<Release> {
    let mut out = Vec::new();
    let mut body = String::new();
    let mut current: Option<(String, Option<String>)> = None;
    let mut flush = |current: &mut Option<(String, Option<String>)>, body: &mut String| {
        if let Some((version, date)) = current.take() {
            out.push(Release { version, date, summary: summary(body), groups: groups(body) });
        }
        body.clear();
    };
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("## [") {
            flush(&mut current, &mut body);
            let Some((version, tail)) = rest.split_once(']') else { continue };
            if version.eq_ignore_ascii_case("unreleased") {
                continue;
            }
            let date = tail.trim().strip_prefix('-').map(|d| d.trim().to_string()).filter(|d| !d.is_empty());
            current = Some((version.to_string(), date));
        } else if current.is_some() {
            body.push_str(line);
            body.push('\n');
        }
    }
    flush(&mut current, &mut body);
    out
}

/// The plain text before a version's first `### Heading`, its lines joined.
fn summary(body: &str) -> Option<String> {
    let text = body
        .lines()
        .take_while(|line| !line.starts_with("### "))
        .map(plain)
        .filter(|line| !line.is_empty() && !is_link_reference(line))
        .collect::<Vec<_>>()
        .join(" ");
    (!text.is_empty()).then_some(text)
}

fn is_link_reference(line: &str) -> bool {
    line.starts_with('[') && line.contains("]: ")
}

/// `### Heading` groups and their `- ` entries, wrapped lines joined.
fn groups(body: &str) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    let mut open_item = false;
    for line in body.lines() {
        if let Some(kind) = line.strip_prefix("### ") {
            out.push(Group { kind: kind.trim().to_string(), items: Vec::new() });
            open_item = false;
        } else if let Some(entry) = line.strip_prefix("- ") {
            let Some(group) = out.last_mut() else { continue };
            group.items.push(plain(entry));
            open_item = true;
        } else if open_item && line.starts_with("  ") && !line.trim().is_empty() {
            if let Some(item) = out.last_mut().and_then(|g| g.items.last_mut()) {
                item.push(' ');
                item.push_str(&plain(line));
            }
        } else if line.trim().is_empty() || line.starts_with('[') {
            open_item = false;
        }
    }
    out.retain(|g| !g.items.is_empty());
    out
}

fn plain(text: &str) -> String {
    text.trim().replace('`', "")
}

/// `1.2.3` as numbers; anything after `-` or `+` is ignored.
pub fn version_key(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.trim().trim_start_matches('v').split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    Some((parts.next()??, parts.next().unwrap_or(Some(0))?, parts.next().unwrap_or(Some(0))?))
}

/// The English releases, each replaced by its translation where one exists.
fn localized(english: &str, translated: Option<&str>) -> Vec<Release> {
    let local = translated.map(releases).unwrap_or_default();
    releases(english)
        .into_iter()
        .map(|r| local.iter().find(|l| l.version == r.version).cloned().unwrap_or(r))
        .collect()
}

/// The notes of every version up to the one running, newest first.
pub fn release_notes(lang: &str) -> Vec<Release> {
    let current = version_key(VERSION);
    let translated = (lang == "pt-BR").then_some(CHANGELOG_PT_BR);
    localized(CHANGELOG, translated)
        .into_iter()
        .filter(|r| version_key(&r.version).is_some_and(|k| Some(k) <= current))
        .collect()
}

// ---- the feed ----------------------------------------------------------

/// `latest.json`, as the release workflow writes it.
#[derive(Clone, Debug, Deserialize)]
pub struct Feed {
    pub version: String,
    #[serde(default)]
    pub pub_date: Option<String>,
    pub url: String,
    pub signature: String,
    /// The whole changelog per language ("en", "pt-BR").
    #[serde(default)]
    pub changelog: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Check {
    pub current: String,
    pub latest: String,
    pub available: bool,
    /// The versions newer than the one running, newest first.
    pub pending: Vec<Release>,
}

fn feed_url() -> String {
    std::env::var("MCP_MANAGER_UPDATE_FEED").unwrap_or_else(|_| FEED_URL.to_string())
}

fn agent() -> ureq::Agent {
    use ureq::tls::{TlsConfig, TlsProvider};
    ureq::Agent::config_builder()
        .tls_config(TlsConfig::builder().provider(TlsProvider::NativeTls).build())
        .timeout_global(Some(Duration::from_secs(120)))
        .build()
        .into()
}

pub fn fetch_feed() -> Result<Feed> {
    let url = feed_url();
    let mut response = agent().get(&url).call().with_context(|| format!("asking {url}"))?;
    let text = response.body_mut().read_to_string().context("reading the update feed")?;
    serde_json::from_str(&text).context("the update feed is not valid JSON")
}

/// What `feed` offers over the running version.
pub fn compare(feed: &Feed, current: &str, lang: &str) -> Check {
    let current_key = version_key(current);
    let latest_key = version_key(&feed.version);
    let available = matches!((latest_key, current_key), (Some(l), Some(c)) if l > c);
    let mut pending = Vec::new();
    if available {
        let english = feed.changelog.get("en").map(String::as_str);
        let translated = feed.changelog.get(lang).map(String::as_str).filter(|_| lang != "en");
        if let Some(english) = english {
            pending = localized(english, translated)
                .into_iter()
                .filter(|r| {
                    let k = version_key(&r.version);
                    k > current_key && k <= latest_key
                })
                .collect();
        }
        if pending.is_empty() {
            pending.push(Release {
                version: feed.version.clone(),
                date: feed.pub_date.as_deref().map(|d| d.chars().take(10).collect()),
                summary: None,
                groups: Vec::new(),
            });
        }
    }
    Check { current: current.to_string(), latest: feed.version.clone(), available, pending }
}

pub fn check(lang: &str) -> Result<(Check, Feed)> {
    let feed = fetch_feed()?;
    Ok((compare(&feed, VERSION, lang), feed))
}

// ---- download, signature, swap ------------------------------------------

/// Checks `data` against a `tauri signer` signature, and that the signature
/// was made for `version` when it names one.
pub fn verify(data: &[u8], signature_b64: &str, public_key_b64: &str, version: &str) -> Result<()> {
    let decode = |b64: &str| -> Result<String> {
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64.trim()).context("not base64")?;
        String::from_utf8(bytes).context("not text")
    };
    let key = minisign_verify::PublicKey::decode(&decode(public_key_b64)?).map_err(|e| anyhow!("public key: {e}"))?;
    let signature =
        minisign_verify::Signature::decode(&decode(signature_b64)?).map_err(|e| anyhow!("signature: {e}"))?;
    key.verify(data, &signature, false).map_err(|_| anyhow!("the download does not match its signature"))?;
    let signed_for = signature
        .trusted_comment()
        .split('\t')
        .find_map(|field| field.strip_prefix("version:"))
        .map(str::to_string);
    if let Some(signed) = signed_for {
        if signed != version {
            bail!("the download is signed for version {signed}, not {version}");
        }
    }
    Ok(())
}

fn sibling(exe: &Path, suffix: &str) -> PathBuf {
    let stem = exe.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "mcp-manager".into());
    exe.with_file_name(format!("{stem}.{suffix}.exe"))
}

/// Downloads the release, checks its signature and puts it in place of
/// `exe`, which keeps running under its new name.
pub fn install(feed: &Feed, exe: &Path, progress: &dyn Fn(u64, Option<u64>)) -> Result<()> {
    let response = agent().get(&feed.url).call().with_context(|| format!("downloading {}", feed.url))?;
    let total = response.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok());
    let mut reader = response.into_body().into_reader();
    let mut data = Vec::new();
    let mut chunk = vec![0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut chunk).context("downloading the update")?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&chunk[..n]);
        progress(data.len() as u64, total);
    }
    verify(&data, &feed.signature, PUBLIC_KEY, &feed.version)?;
    let fresh = sibling(exe, "new");
    std::fs::write(&fresh, &data).with_context(|| format!("writing {}", fresh.display()))?;
    swap(exe, &fresh)
}

/// Renames the running `exe` out of the way and moves `fresh` into its place.
pub fn swap(exe: &Path, fresh: &Path) -> Result<()> {
    let mut old = sibling(exe, "old");
    if old.exists() && std::fs::remove_file(&old).is_err() {
        // A previous old exe is still running (an MCP process Claude started).
        old = sibling(exe, &format!("old.{}", chrono::Local::now().format("%Y%m%d%H%M%S")));
    }
    std::fs::rename(exe, &old).with_context(|| format!("moving {} aside", exe.display()))?;
    if let Err(e) = std::fs::rename(fresh, exe) {
        let _ = std::fs::rename(&old, exe);
        return Err(e).with_context(|| format!("putting the new version in {}", exe.display()));
    }
    Ok(())
}

/// Deletes the exes earlier updates left beside this one; the ones still running stay.
pub fn clean_old(exe: &Path) {
    let Some(dir) = exe.parent() else { return };
    let stem = exe.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let leftover = name.starts_with(&format!("{stem}.old")) || name == format!("{stem}.new.exe");
        if leftover && name.ends_with(".exe") {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Changelog\n\n## [Unreleased]\n\n### Added\n\n- Not out yet.\n\n## [0.3.0] - 2026-12-10\n\nFaster checks.\n\n### Added\n\n- A `queue` view,\n  with filters.\n\n### Fixed\n\n- A crash.\n\n## [0.2.0] - 2026-12-01\n\n### Changed\n\n- New icon.\n\n## [0.1.0] - 2026-11-24\n\n### Added\n\n- First version.\n\n[Unreleased]: https://example.invalid\n";
    const TRANSLATION: &str = "## [0.3.0] - 2026-12-10\n\nVerificações mais rápidas.\n\n### Added\n\n- Uma tela da fila.\n";

    // Signed with the key in PUBLIC_KEY: `tauri signer sign --app-version 9.9.9` over "hello\r\n".
    const FIXTURE: &[u8] = b"hello\r\n";
    const FIXTURE_SIG: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVTRGlaT3lmYzNORFNKZmZWRlNoTzZBVlFCZUlXOWhKSjhGTmtEWVA1eHBWQlJBTHl1SnFiQUE0V3hYcHFaOEcyOFIwV1JNbDUrK2dWWStoWThpSmlkdm5WWTZQNDVWdFE0PQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk1NTc1Mjk4CWZpbGU6ZmlsZS5iaW4JdmVyc2lvbjo5LjkuOQo2bmRLbEdEL0NxcitoWm5RK0M4SjlhY2U0cGdLOTdzZEd1RDdneTZjU3g5em1PeEhyZExHdnpLaUtWblFQUjRab0g5c1pjcDBUMHUwTStkcVVUOGxDUT09Cg==";

    #[test]
    fn reads_released_sections_and_skips_unreleased() {
        let list = releases(SAMPLE);
        assert_eq!(list.iter().map(|r| r.version.as_str()).collect::<Vec<_>>(), ["0.3.0", "0.2.0", "0.1.0"]);
        assert_eq!(list[0].summary.as_deref(), Some("Faster checks."));
        assert_eq!(list[0].groups[0].items, ["A queue view, with filters."]);
        assert_eq!(list[0].groups[1].kind, "Fixed");
        assert_eq!(list[1].summary, None);
        assert_eq!(list[2].date.as_deref(), Some("2026-11-24"));
    }

    #[test]
    fn versions_compare_by_number() {
        assert!(version_key("0.10.0") > version_key("0.9.3"));
        assert_eq!(version_key("v1.2"), Some((1, 2, 0)));
        assert_eq!(version_key("Unreleased"), None);
    }

    fn feed(version: &str) -> Feed {
        let mut changelog = BTreeMap::new();
        changelog.insert("en".to_string(), SAMPLE.to_string());
        changelog.insert("pt-BR".to_string(), TRANSLATION.to_string());
        Feed { version: version.into(), pub_date: None, url: String::new(), signature: String::new(), changelog }
    }

    #[test]
    fn lists_every_version_between_the_running_one_and_the_latest() {
        let check = compare(&feed("0.3.0"), "0.1.0", "en");
        assert!(check.available);
        assert_eq!(check.pending.iter().map(|r| r.version.as_str()).collect::<Vec<_>>(), ["0.3.0", "0.2.0"]);
        let pt = compare(&feed("0.3.0"), "0.1.0", "pt-BR");
        assert_eq!(pt.pending[0].summary.as_deref(), Some("Verificações mais rápidas."));
        assert_eq!(pt.pending[1].groups[0].items, ["New icon."]);
        assert!(!compare(&feed("0.3.0"), "0.3.0", "en").available);
        assert!(!compare(&feed("0.2.0"), "0.3.0", "en").available);
    }

    #[test]
    fn a_feed_without_notes_still_names_its_version() {
        let mut bare = feed("0.4.0");
        bare.changelog.clear();
        let check = compare(&bare, "0.3.0", "en");
        assert_eq!(check.pending.len(), 1);
        assert_eq!(check.pending[0].version, "0.4.0");
    }

    #[test]
    fn the_signature_is_checked_and_bound_to_its_version() {
        assert!(verify(FIXTURE, FIXTURE_SIG, PUBLIC_KEY, "9.9.9").is_ok());
        assert!(verify(b"hellO\r\n", FIXTURE_SIG, PUBLIC_KEY, "9.9.9").is_err());
        assert!(verify(FIXTURE, FIXTURE_SIG, PUBLIC_KEY, "1.0.0").is_err());
    }

    #[test]
    fn swap_moves_the_old_exe_aside_and_clean_removes_it() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("mcp-manager.exe");
        let fresh = dir.path().join("mcp-manager.new.exe");
        std::fs::write(&exe, b"old").unwrap();
        std::fs::write(&fresh, b"new").unwrap();
        swap(&exe, &fresh).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), b"new");
        assert_eq!(std::fs::read(dir.path().join("mcp-manager.old.exe")).unwrap(), b"old");
        clean_old(&exe);
        assert!(!dir.path().join("mcp-manager.old.exe").exists());
        assert!(exe.exists());
    }

    /// The verb each entry of a group starts with, so a line read alone names its kind.
    fn group_verb(kind: &str, portuguese: bool) -> &'static str {
        match (kind, portuguese) {
            ("Added", false) => "Add",
            ("Changed", false) => "Change",
            ("Deprecated", false) => "Deprecate",
            ("Removed", false) => "Remove",
            ("Fixed", false) => "Fix",
            ("Security", false) => "Secure",
            ("Added", true) => "Adiciona",
            ("Changed", true) => "Altera",
            ("Deprecated", true) => "Descontinua",
            ("Removed", true) => "Remove",
            ("Fixed", true) => "Corrige",
            ("Security", true) => "Protege",
            _ => panic!("unknown changelog heading {kind}"),
        }
    }

    #[test]
    fn every_entry_starts_with_its_groups_verb() {
        for (text, portuguese) in [(CHANGELOG, false), (CHANGELOG_PT_BR, true)] {
            for release in releases(text) {
                for group in &release.groups {
                    let verb = group_verb(&group.kind, portuguese);
                    for item in &group.items {
                        assert!(
                            item.split_whitespace().next() == Some(verb),
                            "{} {}: \"{item}\" must start with \"{verb}\"",
                            release.version,
                            group.kind
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_shipped_changelogs_parse_and_match() {
        let english = releases(CHANGELOG);
        let portuguese = releases(CHANGELOG_PT_BR);
        assert!(!english.is_empty());
        for r in &english {
            assert!(version_key(&r.version).is_some(), "{}", r.version);
            assert!(!r.groups.is_empty(), "{} has no entries", r.version);
            let pt = portuguese.iter().find(|p| p.version == r.version);
            assert!(pt.is_some(), "{} has no pt-BR notes", r.version);
            assert_eq!(pt.unwrap().date, r.date, "{} dates differ", r.version);
        }
        assert!(english.iter().any(|r| r.version == VERSION), "CHANGELOG.md has no section for {VERSION}");
    }
}
