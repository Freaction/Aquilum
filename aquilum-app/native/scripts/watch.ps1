# Пересборка и перезапуск нативного окна при правке кода.
#
# Следит за исходниками окна, ядра, токенами стилей и переводами. После сохранения собирает
# профиль native-watch (зависимости оптимизированы, своя сборка быстрая) и перезапускает окно:
# старое закрывается штатно — дописывает несохранённое, запоминает размер, положение и
# открытую заметку. Если сборка упала, старое окно остаётся работать.
#
# Запуск: native\watch.bat [аргументы окна]; без аргументов — последняя открытая база.

param([Parameter(ValueFromRemainingArguments = $true)][string[]]$AppArgs)

$ErrorActionPreference = 'Stop'
$native = Split-Path $PSScriptRoot -Parent
$app = Split-Path $native -Parent
$exe = Join-Path (Split-Path $app -Parent) '.artifacts\cargo-native\native-watch\aquilum-native.exe'

if (-not $AppArgs) {
    $AppArgs = @()
}
$AppArgs += '--keep-position'
if ($AppArgs -notcontains '--data-dir') {
    $AppArgs += @('--data-dir', (Join-Path $env:APPDATA 'com.dmitriy.aquilum-app.native-dev'))
}

# Профиль задаётся флагами, чтобы не трогать Cargo.toml рабочей версии.
$cargoArgs = @(
    'build', '-p', 'aquilum-native', '--profile', 'native-watch',
    '--config', "profile.native-watch.inherits='dev'",
    '--config', "profile.native-watch.package.'*'.opt-level=3",
    '--config', 'profile.native-watch.opt-level=1',
    '--config', 'profile.native-watch.debug-assertions=false',
    '--config', "profile.native-watch.debug='line-tables-only'"
)

$watched = @(
    (Join-Path $native 'src'),
    (Join-Path $native 'build.rs'),
    (Join-Path $native 'Cargo.toml'),
    (Join-Path $app 'core\src'),
    (Join-Path $native 'assets')
)

function Get-Stamp {
    $latest = [datetime]::MinValue
    foreach ($path in $watched) {
        if (-not (Test-Path $path)) { continue }
        foreach ($file in Get-ChildItem $path -Recurse -File -ErrorAction SilentlyContinue) {
            if ($file.LastWriteTimeUtc -gt $latest) { $latest = $file.LastWriteTimeUtc }
        }
    }
    $latest
}

function Quote([string]$arg) {
    if ($arg -match '[\s"]') { '"' + ($arg -replace '"', '\"') + '"' } else { $arg }
}

function Stop-App($process) {
    if (-not $process -or $process.HasExited) { return }
    # Штатное закрытие (WM_CLOSE): окно сохраняет правку, размер и открытую заметку.
    [void]$process.CloseMainWindow()
    if (-not $process.WaitForExit(5000)) { $process.Kill() }
}

function Build {
    Write-Host ''
    Write-Host ("[{0:HH:mm:ss}] сборка..." -f (Get-Date)) -ForegroundColor Cyan
    Push-Location $app
    try { & cargo @cargoArgs } finally { Pop-Location }
    $LASTEXITCODE -eq 0
}

$process = $null
$stamp = Get-Stamp
try {
    while ($true) {
        if (Build) {
            Stop-App $process
            $process = Start-Process $exe -ArgumentList (($AppArgs | ForEach-Object { Quote $_ }) -join ' ') -PassThru
            Write-Host ("[{0:HH:mm:ss}] окно запущено, жду правок (Ctrl+C — выход)" -f (Get-Date)) -ForegroundColor Green
        } else {
            Write-Host 'Сборка не удалась — прежнее окно работает, жду исправления.' -ForegroundColor Yellow
        }
        # Ждём правку, затем ещё полсекунды тишины: редактор может сохранять несколько файлов.
        do {
            Start-Sleep -Milliseconds 500
            $next = Get-Stamp
        } while ($next -eq $stamp)
        do {
            $stamp = $next
            Start-Sleep -Milliseconds 500
            $next = Get-Stamp
        } while ($next -ne $stamp)
    }
} finally {
    Stop-App $process
}
