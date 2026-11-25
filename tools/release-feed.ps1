# Writes a release's files from the signed exe and the changelogs: the exe and its signature,
# latest.json (the update feed the app reads), the release notes and SHA256SUMS.txt.
# Usage: powershell -ExecutionPolicy Bypass -File tools/release-feed.ps1 -Version 0.2.0 [-Exe <path>] [-Out <folder>]
# The exe must already carry its signature beside it (<exe>.sig, from `tauri signer sign`).
param(
    [Parameter(Mandatory = $true)] [string]$Version,
    [string]$Exe,
    [string]$Out,
    [string]$Repo = 'Filipe-Soares-de-Almeida/mcp-manager',
    [string]$Url
)
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
if (-not $Exe) { $Exe = Join-Path $root 'src-tauri\target\release\mcp-manager.exe' }
if (-not $Out) { $Out = Join-Path $root 'release' }
if (-not $Url) { $Url = "https://github.com/$Repo/releases/download/v$Version/mcp-manager.exe" }
$utf8 = New-Object System.Text.UTF8Encoding $false
foreach ($file in @($Exe, "$Exe.sig")) {
    if (-not (Test-Path $file)) { throw "$file is missing: build the exe and sign it first." }
}

# The body of `## [<version>] - <date>`, up to the next version or the links.
function Get-Section([string]$text, [string]$version) {
    $lines = $text -split "`r?`n"
    $start = -1
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match ('^## \[' + [regex]::Escape($version) + '\] - \d{4}-\d{2}-\d{2}\s*$')) { $start = $i; break }
    }
    if ($start -lt 0) { return $null }
    $body = for ($i = $start + 1; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match '^## \[' -or $lines[$i] -match '^\[[^\]]+\]: ') { break }
        $lines[$i]
    }
    ($body -join "`n").Trim()
}

$english = [System.IO.File]::ReadAllText((Join-Path $root 'CHANGELOG.md'), $utf8)
$portuguese = [System.IO.File]::ReadAllText((Join-Path $root 'locales\changelog\pt-BR.md'), $utf8)
$section = Get-Section $english $Version
if (-not $section) { throw "CHANGELOG.md has no '## [$Version] - YYYY-MM-DD' section." }

New-Item -ItemType Directory -Force $Out | Out-Null
Copy-Item $Exe (Join-Path $Out 'mcp-manager.exe') -Force
Copy-Item "$Exe.sig" (Join-Path $Out 'mcp-manager.exe.sig') -Force

$feed = [ordered]@{
    version   = $Version
    pub_date  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    url       = $Url
    signature = ([System.IO.File]::ReadAllText("$Exe.sig", $utf8)).Trim()
    changelog = [ordered]@{ en = $english; 'pt-BR' = $portuguese }
}
[System.IO.File]::WriteAllText((Join-Path $Out 'latest.json'), ($feed | ConvertTo-Json -Depth 5), $utf8)

$footer = @'

---

### Install

Download `mcp-manager.exe` below, put it in a folder of its own and open it. An app that is already
installed updates itself from its Changelog screen.

The exe is not code-signed yet: Windows may show "Windows protected your PC". Check that the file came
from this page and that its SHA-256 matches `SHA256SUMS.txt`, then choose **More info**, then **Run
anyway**.

```powershell
(Get-FileHash .\mcp-manager.exe -Algorithm SHA256).Hash
```

[Full changelog](https://github.com/{repo}/blob/v{version}/CHANGELOG.md)
'@
$footer = $footer.Replace('{version}', $Version).Replace('{repo}', $Repo)
[System.IO.File]::WriteAllText((Join-Path $Out 'release-notes.md'), "$section`n$footer", $utf8)

$sums = foreach ($name in @('mcp-manager.exe', 'mcp-manager.exe.sig', 'latest.json')) {
    "$((Get-FileHash (Join-Path $Out $name) -Algorithm SHA256).Hash.ToLowerInvariant())  $name"
}
[System.IO.File]::WriteAllText((Join-Path $Out 'SHA256SUMS.txt'), (($sums -join "`n") + "`n"), $utf8)
"Release $Version written to $Out"
