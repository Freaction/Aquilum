# Замер памяти и простоя нативного окна Aquilum.
#
#   powershell -File native\scripts\measure.ps1 [-Exe путь] [-Threads 0,2,4] [-BenchFrames 300] [-Idle 10] [-File заметка.md]
#
# Прототип на Freya (probes/freya) меряется тем же скриптом:
#   measure.ps1 -Exe <freya-probe.exe> -Threads 0 -Size 1646x981 -Env FREYA_RENDERER=software -Label freya-software
#
# Для каждого числа потоков vello_cpu: запускает окно, ждёт первый кадр и прогон прокрутки, затем
# Idle секунд простоя (по ним считается CPU), снимает память и закрывает окно через WM_CLOSE, чтобы
# приложение успело записать свои замеры. Числа сравнимы с замерами WebView2 в memory-footprint.
param(
  [string]$Exe = "$PSScriptRoot\..\..\..\.artifacts\cargo-native\release\aquilum-native.exe",
  [int[]]$Threads = @(0, 2, 4),
  [int]$BenchFrames = 300,
  [int]$Idle = 10,
  [string]$Size = "2881x1717",
  [string]$File = "",
  # Переменная окружения для запускаемого процесса, «ИМЯ=значение».
  [string]$Env = "",
  [string]$Label = "aquilum-native",
  # Дополнительные аргументы запускаемому окну, например @("--focus") для пробы Masonry.
  [string[]]$ExtraArgs = @()
)

$ErrorActionPreference = "Stop"
$Exe = (Resolve-Path $Exe).Path
$results = @()
if ($Env) {
  $name, $value = $Env -split "=", 2
  [Environment]::SetEnvironmentVariable($name, $value, "Process")
}

foreach ($t in $Threads) {
  $metricsPath = Join-Path $env:TEMP "aquilum-native-metrics-$t.json"
  Remove-Item $metricsPath -ErrorAction SilentlyContinue
  $argList = @("--size", $Size, "--threads", $t, "--bench-frames", $BenchFrames, "--metrics", "`"$metricsPath`"")
  $argList += @("--data-dir", "`"$(Join-Path $env:APPDATA com.dmitriy.aquilum-app.native-dev)`"")
  if ($File) { $argList += @("--file", "`"$File`"") }
  $argList += $ExtraArgs

  $sw = [Diagnostics.Stopwatch]::StartNew()
  $p = Start-Process -FilePath $Exe -ArgumentList $argList -PassThru
  $windowMs = -1
  while ($sw.ElapsedMilliseconds -lt 15000) {
    $p.Refresh()
    if ($p.MainWindowHandle -ne 0) { $windowMs = $sw.ElapsedMilliseconds; break }
    Start-Sleep -Milliseconds 10
  }
  # Прогон прокрутки идёт сразу после первого кадра; даём ему закончиться и окну успокоиться.
  Start-Sleep -Seconds 3

  $p.Refresh()
  $cpu0 = $p.TotalProcessorTime
  $idleSw = [Diagnostics.Stopwatch]::StartNew()
  Start-Sleep -Seconds $Idle
  $p.Refresh()
  $cpuIdle = ($p.TotalProcessorTime - $cpu0).TotalMilliseconds / $idleSw.Elapsed.TotalMilliseconds * 100

  $privWs = -1
  $inst = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | Where-Object { $_.IDProcess -eq $p.Id }
  if ($inst) { $privWs = [math]::Round($inst.WorkingSetPrivate / 1MB, 1) }
  $row = [ordered]@{
    Label           = $Label
    Threads         = $t
    WindowMs        = $windowMs
    WS_MB           = [math]::Round($p.WorkingSet64 / 1MB, 1)
    PrivWS_MB       = $privWs
    PrivateBytes_MB = [math]::Round($p.PrivateMemorySize64 / 1MB, 1)
    PeakWS_MB       = [math]::Round($p.PeakWorkingSet64 / 1MB, 1)
    IdleCpuPct      = [math]::Round($cpuIdle, 2)
    OsThreads       = $p.Threads.Count
  }

  [void]$p.CloseMainWindow()
  if (-not $p.WaitForExit(5000)) { Stop-Process -Id $p.Id -Force }

  if (Test-Path $metricsPath) {
    $m = Get-Content $metricsPath -Raw | ConvertFrom-Json
    foreach ($prop in $m.PSObject.Properties) {
      if ($prop.Name -ne "threads") { $row[$prop.Name] = $prop.Value }
    }
  }
  $results += [pscustomobject]$row
}

$results | Format-List
