<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" alt="MCP Manager logo: a coral spark between light angle brackets, on a dark rounded square" width="128" height="128">
</p>

<h1 align="center">MCP Manager</h1>

<p align="center">
  <b>A manager for Claude Desktop's MCP servers.</b>
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  <img alt="Platform: Windows" src="https://img.shields.io/badge/platform-Windows-0078d6.svg">
  <img alt="Built with Rust and Tauri" src="https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri-b7410e.svg">
</p>

<p align="center">
  <b>English</b> · <a href="README.pt-BR.md">Português (Brasil)</a>
</p>

<p align="center">
  <img src=".github/assets/readme/hero.png" width="880" alt="MCP Manager's Servers screen while Claude is open: a banner says two changes are queued, brave-search shows 'Queued: add' among the enabled servers, sqlite-tools shows 'Queued: disable' among the disabled ones, and the app itself is registered as mcp-manager.">
</p>

---

MCP Manager is a free, open-source Windows app that adds, edits, disables and removes Claude
Desktop's MCP servers, from a window or from Claude's own chat. Every change made while Claude is
open waits in a queue and is written to `claude_desktop_config.json` when Claude closes; **Restart
Claude** closes it, applies the queue and opens it again. It also keeps disabled servers so you can
turn them back on, backs up the config before every write, and waits in the tray, one click away.

It runs on Windows (x64), in English and Brazilian Portuguese, and works with both ways Claude
Desktop is installed: the MSIX package and the older installer in `%LOCALAPPDATA%\AnthropicClaude`.
It is for anyone who sets up MCP servers in Claude Desktop and would rather not edit the JSON by hand
or time Claude's restart around it. It changes Claude Desktop's config only: the Claude Code CLI's
`~/.claude.json` is out of scope, and so are URL-only remote servers, which that config does not take.

## Highlights

- **Nothing changes while Claude is open.** Claude Desktop rewrites its own config while it runs, so
  every change made with Claude open goes into a queue. The queue is written once Claude's last
  process exits; **Restart Claude** closes it, applies the queue and opens it again.
- **Disabling does not delete.** A disabled server leaves the config and is kept in MCP Manager's
  database with its command, arguments, environment variables and any other field. Enabling it puts
  the same entry back.
- **Also from the chat.** The app is an MCP server too: registered with one click, it lets Claude
  list, add, edit, disable and remove servers. Through it, environment values reach the chat as `***`
  unless the agent asks for them.
- **A backup before every write.** `claude_desktop_config.json` is copied before each write and when
  the app opens, if the config changed; the 100 newest copies are kept, and any of them can be
  restored. Only `mcpServers` is touched: `preferences` and every other key are written back as read.
- **Finds the config from Claude's executable.** The path comes from the running `Claude.exe` or from
  the package registered in Windows: `%LOCALAPPDATA%\Claude-Data\<package>\` for MSIX,
  `%APPDATA%\Claude\` for the older installer.
- **Local.** MCP Manager makes no network requests: it only reads and writes files on this PC.

---

## Install

There is no installer yet: the executable is built from source. You need:

- Windows x64 with the
  [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/), which ships with
  Windows 11 and with up-to-date Windows 10.
- Rust 1.89 or newer, with the MSVC toolchain (`x86_64-pc-windows-msvc`) and Visual Studio Build
  Tools.
- Node.js 20.19+, 22.12+ or 24+.

```powershell
npm install
npm run app:build
```

The executable lands in `src-tauri\target\release\mcp-manager.exe`. Copy it to a permanent folder,
such as `%LOCALAPPDATA%\Programs\MCP Manager\`, and double-click it. On the first run, the **First
steps** card in Servers registers the app's MCP in Claude, turns on starting with Windows and creates
the Start menu shortcut.

> [!IMPORTANT]
> The executable's path is written into Claude's config when the app's MCP is registered, and into
> the start-with-Windows entry. Pick the permanent folder first. If you move the file later, Settings
> shows "Points to another executable" with an **Update the path** button; turn **Start with
> Windows** off and on again to fix the other entry.

<details>
<summary><b>Development</b> — hot reload, tests, and testing without touching Claude</summary>

```powershell
npm run app:dev
npm run dev
npm run check
cd src-tauri; cargo test
```

`npm run app:dev` opens the app with hot reload. `npm run dev` serves only the interface in a
browser, at `http://localhost:1420`, with sample data; `?hero` opens the state of the screenshot
above, and `?lang=en-US` or `?lang=pt-BR` picks the language.

| Variable | Effect |
|---|---|
| `MCP_MANAGER_HOME` | Data folder instead of `%USERPROFILE%\.mcp-manager` |
| `MCP_MANAGER_CONFIG` | Config used instead of the detected one |
| `MCP_MANAGER_FAKE_CLAUDE` | `1` pretends Claude is open, `0` closed |
| `MCP_MANAGER_NO_WATCHER` | Starts neither the watcher nor the tray app |

</details>

---

## The window

Open it from the tray icon or the Start menu shortcut. The **+** button adds a server. The window
draws its own title bar: it shows whether Claude is open and, while changes are queued, the
**Restart Claude** button, with a banner below that explains the queue.

| Screen | What it does |
|---|---|
| **Servers** | The enabled and the disabled servers, with a switch to turn each one on or off, edit, remove, and undo the last queued change. The editor takes JSON pasted from a README and fills in the command, arguments and variables. |
| **Changes** | The queue, where you cancel one change or discard them all, and the history of applied changes, with the ones that failed and why. |
| **Backups** | The config copies, with their date and number of servers. **View** lists a copy's servers and **Restore** queues its list. |
| **Settings** | Starting with Windows and with Claude, the Start menu shortcut, the language (automatic, English or Portuguese), registering the app's MCP, the Claude executable and package found, the config path (or another one you choose) and the data folder. |

## The tray

The bracket icon sits in the notification area. A click opens the window; its menu has **Open MCP
Manager**, **Restart Claude and apply the queue** and **Quit**, and its tooltip says how many changes
are queued. Closing the window only hides it. The app starts in the tray on its own, without a window,
when Windows starts (if turned on in Settings) and when Claude starts the app's MCP while the app is
not running.

## The app's MCP

**Settings › The app's own MCP › Add to Claude** writes the executable itself into Claude's config,
with `--mcp`:

```json
{
  "mcpServers": {
    "mcp-manager": {
      "command": "C:\\Users\\you\\AppData\\Local\\Programs\\MCP Manager\\mcp-manager.exe",
      "args": ["--mcp"]
    }
  }
}
```

| Tool | What it does |
|---|---|
| `list_servers` | Lists the enabled servers, the disabled ones and the queue, with variable names but not their values. |
| `get_server` | Shows a server's whole entry; variable values only with `reveal_env`. |
| `add_server` | Adds a server with its command, arguments, variables and other fields. |
| `update_server` | Changes only the fields given: `env` adds variables to the existing ones, `remove_env` deletes some, `rename` changes the name. |
| `remove_server` | Removes the server from the config or, if it is disabled, from MCP Manager's database. |
| `disable_server` | Takes the server out of the config and keeps it in the database, to enable later. |
| `enable_server` | Puts a disabled server back into the config. |
| `list_changes` | The queue and the latest applied changes. |
| `cancel_change` | Cancels a queued change by number, or all of them. |
| `open_manager` | Opens the MCP Manager window. |

While Claude is open, every change comes back as `queued`, with a note that Claude needs a restart. A
watcher without a window (`mcp-manager.exe --watch`) waits for Claude to close, writes the queue and
exits.

## Where the data lives

| Path | What it is |
|---|---|
| `%USERPROFILE%\.mcp-manager\mcp-manager.db` | The queue, the disabled servers, the history and the settings. Disabled servers exist only here: deleting the file loses them. |
| `%USERPROFILE%\.mcp-manager\backups\` | The copies of `claude_desktop_config.json`. |
| `%USERPROFILE%\.mcp-manager\mcp-manager.log` | The log of the watcher and of every write to the config. |
| `%LOCALAPPDATA%\com.mcpmanager.desktop\` | The window's own files (WebView2). |
| `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, value `MCP Manager` | Starting with Windows, when turned on. |
| `%APPDATA%\Microsoft\Windows\Start Menu\Programs\MCP Manager.lnk` | The Start menu shortcut, when created. |

**Settings › MCP Manager's data** opens the folder. To remove the app from the PC: remove the
`mcp-manager` server in Servers, turn off **Start with Windows**, quit from the tray, then delete the
executable, both folders and the shortcut.

## License

MCP Manager is released under the [MIT License](LICENSE). The interface icons are drawn from Google's
Material Icons, under the Apache License 2.0.
