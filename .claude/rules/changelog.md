---
paths:
  - "CHANGELOG.md"
  - "locales/changelog/pt-BR.md"
---

# Changelog rules

[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) with
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). The app shows these notes in its
Changelog screen and in the release feed, so they are written for someone using the app.

- New work goes under `## [Unreleased]` in both files, in the same commit as the change.
- Headings in this order and no others: `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`,
  `Security`. The pt-BR file keeps the English headings: the app translates them.
- **Every entry starts with its heading's verb.** A line read on its own, far down a long list, still
  says what kind of change it is.

  | Heading | English | pt-BR |
  |---|---|---|
  | Added | Add … | Adiciona … |
  | Changed | Change … | Altera … |
  | Deprecated | Deprecate … | Descontinua … |
  | Removed | Remove … | Remove … |
  | Fixed | Fix … | Corrige … |
  | Security | Secure … | Protege … |

  Yes: "Add groups on the Servers screen." / "Fix the editor closing when the list refreshes."
  No: "Groups on the Servers screen." / "The editor no longer closes when the list refreshes."
  `cargo test` refuses an entry that breaks this, in either file.
- One line, one sentence, ending with a period; never past 100 characters in English.
- Name what the user sees (a screen, a button, a behaviour), never the mechanism.
- Plain text only: the app strips backticks and shows no Markdown.
- An optional one-line summary may sit between a version's heading and its first group.
- Release with `tools/version.ps1 -Version x.y.z`, which closes Unreleased in both files with the
  date, updates the links at the foot of CHANGELOG.md, commits and tags.
- Before 1.0.0, a breaking change bumps the minor, everything else the patch.
