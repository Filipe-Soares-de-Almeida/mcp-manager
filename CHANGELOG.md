# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!--
How to use this file:

- Every change adds its entries under the Unreleased heading, in the same commit that makes the
  change, here and in locales/changelog/pt-BR.md.
- Write for someone using the app, not for someone reading the code.
- Use only these headings, in this order: Added, Changed, Deprecated, Removed, Fixed, Security.
- Every entry starts with its heading's verb, so a line read alone still says what kind of change
  it is: Add, Change, Deprecate, Remove, Fix, Secure. pt-BR: Adiciona, Altera, Descontinua,
  Remove, Corrige, Protege. The tests refuse an entry that does not.
- An optional one-line summary may sit between a version's heading and its first group.
- tools/version.ps1 turns Unreleased into `[x.y.z] - YYYY-MM-DD` in both files, opens a fresh
  Unreleased, updates the links below, commits and tags.
- Before 1.0.0, breaking changes bump the minor, everything else bumps the patch.
-->

## [Unreleased]

## [0.1.0] - 2026-11-24

The first release: Claude Desktop's MCP servers in one window, and from Claude's chat.

### Added

- Add a window to add, edit, rename, remove, disable and enable Claude Desktop's MCP servers.
- Add a queue that holds changes while Claude runs and writes them once it closes.
- Add a Restart Claude button that closes Claude, applies the queue and opens it again.
- Add disabling that keeps a server in the app's database and puts it back unchanged.
- Add a backup of the config before every write, and a Backups screen that restores any copy.
- Add finding the config from Claude's executable, for the MSIX package and the older installer.
- Add an MCP server that lets Claude list, add, edit, remove, disable and enable servers.
- Add filling the editor from a server's JSON pasted from a README.
- Add a tray icon, start with Windows and a Start menu shortcut.
- Add a First steps card for the first run.
- Add the interface in English and Brazilian Portuguese.
- Add an update check that lists the versions not installed yet.
- Add a Changelog screen with the notes of every version.
- Add in-app updates that download the new version, check its signature and install it.

[Unreleased]: https://github.com/Filipe-Soares-de-Almeida/mcp-manager/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Filipe-Soares-de-Almeida/mcp-manager/releases/tag/v0.1.0
