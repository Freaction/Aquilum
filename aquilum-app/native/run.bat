@echo off
rem Builds and runs the native Aquilum UI (release).
rem No arguments: opens the last used knowledge base. Any arguments are passed
rem to the window as is, e.g.: run.bat --vault "D:\Notes"
rem The window runs from a copy, so a running window never blocks the build;
rem the running window is closed normally (it saves) before the new one starts.
rem ASCII only: cmd parses this file in the OEM code page.
setlocal
set "NATIVE=%~dp0"
set "APP=%NATIVE%.."
set "TARGET=%NATIVE%..\..\.artifacts\cargo-native"
set "RUN=%NATIVE%..\..\.artifacts\native-run"

set "BUILT=%TARGET%\release\aquilum-native.exe"
if exist "%BUILT%.old" del /F /Q "%BUILT%.old" >nul 2>&1
if exist "%BUILT%" move /Y "%BUILT%" "%BUILT%.old" >nul 2>&1

pushd "%APP%"
cargo build --release -p aquilum-native
set "BUILD=%errorlevel%"
popd
if not "%BUILD%"=="0" (
    echo.
    echo Build failed.
    pause
    exit /b 1
)

tasklist /FI "IMAGENAME eq aquilum-native.exe" | find /I "aquilum-native.exe" >nul
if not errorlevel 1 (
    taskkill /IM aquilum-native.exe >nul 2>&1
    for /L %%i in (1,1,15) do (
        tasklist /FI "IMAGENAME eq aquilum-native.exe" | find /I "aquilum-native.exe" >nul || goto closed
        ping -n 2 127.0.0.1 >nul
    )
    echo The running Aquilum window did not close. Close it and run again.
    pause
    exit /b 1
)
:closed

if not exist "%RUN%" mkdir "%RUN%"
copy /Y "%TARGET%\release\aquilum-native.exe" "%RUN%\aquilum-native.exe" >nul || (
    echo Could not copy the new build.
    pause
    exit /b 1
)
start "" "%RUN%\aquilum-native.exe" %*
