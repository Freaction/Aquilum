$ErrorActionPreference = 'Continue'
$PSDefaultParameterValues['*:ErrorAction'] = 'Stop'
$ProgressPreference = 'SilentlyContinue'

. "$PSScriptRoot\env.ps1"

function Fetch($url, $zip, $dest) {
    $tmp = Join-Path $env:TEMP $zip
    curl.exe -fsSL -o $tmp $url
    if ($LASTEXITCODE -ne 0) { throw "download failed: $url" }
    $unpacked = "$dest.unpack"
    if (Test-Path $unpacked) { Remove-Item -Recurse -Force $unpacked }
    Expand-Archive -Path $tmp -DestinationPath $unpacked
    Remove-Item -Force $tmp
    $inner = Get-ChildItem $unpacked | Select-Object -First 1
    New-Item -ItemType Directory -Force (Split-Path $dest) | Out-Null
    Move-Item $inner.FullName $dest
    Remove-Item -Recurse -Force $unpacked
}

if (-not (Test-Path "$env:JAVA_HOME\bin\java.exe")) {
    Write-Host "JDK $JdkVersion"
    Fetch "https://api.adoptium.net/v3/binary/latest/$JdkVersion/ga/windows/x64/jdk/hotspot/normal/eclipse" 'jdk.zip' $env:JAVA_HOME
}

if (-not (Test-Path $Gradle)) {
    Write-Host "Gradle $GradleVersion"
    Fetch "https://services.gradle.org/distributions/gradle-$GradleVersion-bin.zip" 'gradle.zip' (Split-Path (Split-Path $Gradle))
}

$sdkmanager ="$env:ANDROID_HOME\cmdline-tools\latest\bin\sdkmanager.bat"
if (-not (Test-Path $sdkmanager)) {
    Write-Host "cmdline-tools $CmdlineTools"
    Fetch "https://dl.google.com/android/repository/commandlinetools-win-${CmdlineTools}_latest.zip" 'cmdline-tools.zip' "$env:ANDROID_HOME\cmdline-tools\latest"
}

function Sdk([string[]]$Arguments) {
    $quoted = ($Arguments | ForEach-Object { "`"$_`"" }) -join ' '
    cmd /c "`"$sdkmanager`" $quoted 2>&1"
    if ($LASTEXITCODE -ne 0) { throw "sdkmanager $quoted failed" }
}

Write-Host 'licenses'
(1..40 | ForEach-Object { 'y' }) -join "`n" | Sdk '--licenses' | Out-Null

foreach ($package in 'platform-tools', "platforms;android-$CompileSdk", "build-tools;$BuildTools", "ndk;$NdkVersion") {
    Write-Host $package
    Sdk '--install', $package | Select-String -NotMatch 'Unzipping|\[#|\[\.' | Out-Host
}
if (-not (Test-Path "$env:ANDROID_NDK_HOME\source.properties")) { throw "NDK $NdkVersion is missing" }

Write-Host 'rust targets'
rustup target add $RustTargets
if ($LASTEXITCODE -ne 0) { throw 'rustup failed' }

if (-not (Get-Command cargo-ndk -ErrorAction SilentlyContinue)) {
    Write-Host 'cargo-ndk'
    cargo install cargo-ndk --locked
    if ($LASTEXITCODE -ne 0) { throw 'cargo install cargo-ndk failed' }
}

Write-Host "ready: ANDROID_HOME=$env:ANDROID_HOME JAVA_HOME=$env:JAVA_HOME NDK=$env:ANDROID_NDK_HOME"
