# Finds hardcoded colors outside the design-token layers.
# Usage: npm run check:colors
# Exit 1 if any actionable hardcodes remain in UI code.

param(
  [switch]$Strict
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$src = Join-Path $root 'src'

# Token/theme definition layers may contain raw values by design.
$excludePathRegex = '(?i)[\\/](styles[\\/]tokens[\\/]|styles[\\/]themes[\\/]|components[\\/]Decorations[\\/])'

# Intentional / non-UI chrome exceptions (matched against "path:line:content").
$allowContentRegex = @(
  '(?i)--q-window-close-bg-(hover|active)\s*:'
  '(?i)linear-gradient\(#fff\s+0\s+0\)'          # CSS mask utility, not a painted color
  '(?i)rgb\(255\s+255\s+255\s*/\s*var\(--q-button-shine' # primary button shine on accent
  '(?i)rgb\(255\s+255\s+255\s*/\s*0\)'           # transparent end-stop of shine gradient
)

$pattern = '(?i)(#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()'
$files = Get-ChildItem -Path $src -Recurse -Include *.css,*.tsx,*.ts |
  Where-Object { $_.FullName -notmatch $excludePathRegex }

$hits = @()
foreach ($file in $files) {
  $lineNo = 0
  foreach ($line in Get-Content -LiteralPath $file.FullName) {
    $lineNo++
    if ($line -notmatch $pattern) { continue }
    $rel = $file.FullName.Substring($root.Length + 1).Replace('\', '/')
    $entry = "${rel}:${lineNo}: $($line.Trim())"
    $allowed = $false
    foreach ($allow in $allowContentRegex) {
      if ($entry -match $allow) { $allowed = $true; break }
    }
    if (-not $allowed) {
      $hits += [pscustomobject]@{ File = $rel; Line = $lineNo; Text = $line.Trim() }
    }
  }
}

if ($hits.Count -eq 0) {
  Write-Host "OK: no actionable hardcoded colors in UI sources."
  exit 0
}

Write-Host "Found $($hits.Count) hardcoded color hit(s):"
Write-Host ""
$hits | ForEach-Object { Write-Host ("{0}:{1}: {2}" -f $_.File, $_.Line, $_.Text) }
Write-Host ""
Write-Host "Allowed by design: styles/tokens/**, styles/themes/**, Decorations/,"
Write-Host "window-close reds, CSS mask #fff, primary button white shine."
Write-Host "Fix UI hits via semantic/component tokens + dark.css overrides."

if ($Strict) { exit 1 }
exit 0
