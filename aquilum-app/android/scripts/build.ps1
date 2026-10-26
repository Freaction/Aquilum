param(
    [switch]$Release,
    [string[]]$Abi = @('arm64-v8a')
)

$ErrorActionPreference = 'Continue'
. "$PSScriptRoot\env.ps1"

$android = Split-Path $PSScriptRoot
$workspace = Split-Path $android
$version = (Get-Content "$workspace\native\Cargo.toml" | Select-String '^version = "(.+)"').Matches[0].Groups[1].Value

$targets = @($Abi | ForEach-Object { '-t'; $_ })
$mode = @()
if ($Release) { $mode += '--release' }
Push-Location $workspace
cargo ndk @targets -P 26 -o "$android\app\src\main\jniLibs" build -p aquilum-android @mode
$cargo = $LASTEXITCODE
Pop-Location
if ($cargo -ne 0) { throw 'cargo ndk failed' }

$task = if ($Release) { 'assembleRelease' } else { 'assembleDebug' }
& $Gradle -p $android --quiet $task "-PappVersion=$version"
if ($LASTEXITCODE -ne 0) { throw 'gradle failed' }

$kind = if ($Release) { 'release' } else { 'debug' }
Get-Item "$android\app\build\outputs\apk\$kind\app-$kind.apk"
