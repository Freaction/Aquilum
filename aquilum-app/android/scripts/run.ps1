param(
    [switch]$Release,
    [switch]$NoBuild
)

$ErrorActionPreference = 'Continue'
. "$PSScriptRoot\env.ps1"

$kind = if ($Release) { 'release' } else { 'debug' }
$apk = if ($NoBuild) { Get-Item "$(Split-Path $PSScriptRoot)\app\build\outputs\apk\$kind\app-$kind.apk" } else { & "$PSScriptRoot\build.ps1" -Release:$Release }

adb install -r $apk.FullName
if ($LASTEXITCODE -ne 0) { throw 'adb install failed' }
adb logcat -c
adb shell am start -W -n com.dmitriy.aquilum/.MainActivity
