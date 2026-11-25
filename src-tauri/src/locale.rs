//! The interface language: Brazilian Portuguese or US English. "auto" follows
//! the Windows display language. The page has its own dictionaries; this
//! module only resolves the choice and holds the tray's few labels.

pub const SUPPORTED: [&str; 2] = ["pt-BR", "en-US"];

/// The Windows display language, mapped to a supported locale.
pub fn system() -> &'static str {
    // SAFETY: takes no arguments and only reads the user's settings.
    let lang = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() };
    const LANG_PORTUGUESE: u16 = 0x16;
    if lang & 0x3ff == LANG_PORTUGUESE { "pt-BR" } else { "en-US" }
}

/// "auto" or anything unknown resolves to the system language.
pub fn resolve(setting: &str) -> &'static str {
    SUPPORTED.iter().find(|l| **l == setting).copied().unwrap_or_else(system)
}

pub struct TrayText {
    pub open: &'static str,
    pub restart: &'static str,
    pub quit: &'static str,
}

pub fn tray(locale: &str) -> TrayText {
    match locale {
        "pt-BR" => TrayText {
            open: "Abrir MCP Manager",
            restart: "Reiniciar o Claude e aplicar a fila",
            quit: "Sair",
        },
        _ => TrayText { open: "Open MCP Manager", restart: "Restart Claude and apply the queue", quit: "Quit" },
    }
}

pub fn tray_tooltip(locale: &str, pending: i64) -> String {
    match (locale, pending) {
        (_, 0) => "MCP Manager".into(),
        ("pt-BR", 1) => "MCP Manager · 1 alteração na fila".into(),
        ("pt-BR", n) => format!("MCP Manager · {n} alterações na fila"),
        (_, 1) => "MCP Manager · 1 change queued".into(),
        (_, n) => format!("MCP Manager · {n} changes queued"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_locales_and_falls_back_to_the_system() {
        assert_eq!(resolve("pt-BR"), "pt-BR");
        assert_eq!(resolve("en-US"), "en-US");
        assert!(SUPPORTED.contains(&resolve("auto")));
        assert_eq!(tray_tooltip("en-US", 2), "MCP Manager · 2 changes queued");
        assert_eq!(tray_tooltip("pt-BR", 1), "MCP Manager · 1 alteração na fila");
    }
}
