@echo off
rem Rebuilds and restarts the native window on every source change (scripts\watch.ps1).
rem No arguments: opens the last used knowledge base. Ctrl+C stops watching.
rem ASCII only: cmd parses this file in the OEM code page.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\watch.ps1" %*
