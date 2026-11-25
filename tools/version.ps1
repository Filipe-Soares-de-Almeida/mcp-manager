# Releases a version: sets it in every manifest, closes Unreleased in both changelogs, commits and tags.
# Usage: powershell -ExecutionPolicy Bypass -File tools/version.ps1 -Version 0.2.0 [-Push]
# Pushing the tag starts .github/workflows/release.yml, which drafts the GitHub release.
param(
    [Parameter(Mandatory = $true)] [string]$Version,
    [switch]$Push,
    [string]$Date = (Get-Date -Format 'yyyy-MM-dd')
)
$ErrorActionPreference = 'Stop'

if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw "Use x.y.z, as in 0.2.0." }
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
if (git status --porcelain) { throw 'Commit or stash your changes first.' }
$branch = git rev-parse --abbrev-ref HEAD
if ($branch -ne 'main') { throw "Release from main, not $branch." }
if (git tag --list "v$Version") { throw "The tag v$Version already exists." }

$utf8 = New-Object System.Text.UTF8Encoding $false
function Read-Text([string]$path) { [System.IO.File]::ReadAllText((Join-Path $root $path), $utf8) }
function Write-Text([string]$path, [string]$text) { [System.IO.File]::WriteAllText((Join-Path $root $path), $text, $utf8) }

# The Unreleased section must hold entries in both changelogs.
function Get-Unreleased([string]$text) {
    if ($text -notmatch '(?ms)^## \[Unreleased\]\s*\r?\n(.*?)(?=^## \[|^\[[^\]]+\]: |\z)') { return '' }
    $Matches[1].Trim()
}
$english = Read-Text 'CHANGELOG.md'
$portuguese = Read-Text 'locales/changelog/pt-BR.md'
if ((Get-Unreleased $english) -notmatch '(?m)^- ') { throw 'CHANGELOG.md has no entries under Unreleased.' }
if ((Get-Unreleased $portuguese) -notmatch '(?m)^- ') {
    throw 'locales/changelog/pt-BR.md has no entries under Unreleased: translate them first.'
}

# Versions in every manifest.
$edits = @(
    @{ Path = 'package.json'; Pattern = '(?m)^(  "version": ")[^"]+(")' },
    @{ Path = 'package-lock.json'; Pattern = '("name": "mcp-manager",\s*"version": ")[^"]+(")' },
    @{ Path = 'src-tauri/tauri.conf.json'; Pattern = '(?m)^(  "version": ")[^"]+(")' },
    @{ Path = 'src-tauri/Cargo.toml'; Pattern = '(?m)^(version = ")[^"]+(")' },
    @{ Path = 'src-tauri/Cargo.lock'; Pattern = '(?m)(^name = "mcp-manager"\r?\nversion = ")[^"]+(")' }
)
foreach ($edit in $edits) {
    $text = Read-Text $edit.Path
    if ($text -notmatch $edit.Pattern) { throw "$($edit.Path) has no version where expected." }
    $regex = [regex]$edit.Pattern
    $count = if ($edit.Path -eq 'package-lock.json') { -1 } else { 1 }
    Write-Text $edit.Path ($regex.Replace($text, "`${1}$Version`${2}", $count))
}

# Unreleased becomes the version in both changelogs, with a fresh Unreleased above it.
$heading = "## [Unreleased]`n`n## [$Version] - $Date"
$english = ([regex]'(?m)^## \[Unreleased\][ \t]*').Replace($english, $heading, 1)
$portuguese = ([regex]'(?m)^## \[Unreleased\][ \t]*').Replace($portuguese, $heading, 1)

# The links at the foot of CHANGELOG.md.
if ($english -match '(?m)^\[Unreleased\]: (https://github\.com/[^/]+/[^/]+)/compare/v([^.]+\.[^.]+\.[^.]+)\.\.\.HEAD') {
    $repo = $Matches[1]; $previous = $Matches[2]
    $links = "[Unreleased]: $repo/compare/v$Version...HEAD`n[$Version]: $repo/compare/v$previous...v$Version"
    $english = ([regex]'(?m)^\[Unreleased\]: .*$').Replace($english, $links, 1)
}
Write-Text 'CHANGELOG.md' $english
Write-Text 'locales/changelog/pt-BR.md' $portuguese

git add package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock CHANGELOG.md locales/changelog/pt-BR.md
git commit -q -m "chore: release $Version"
git tag -a "v$Version" -m "MCP Manager $Version"
"Committed and tagged v$Version."
if ($Push) {
    git push origin main
    git push origin "v$Version"
    'Pushed: the release workflow drafts the release on GitHub.'
} else {
    "Push when ready: git push origin main; git push origin v$Version"
}
