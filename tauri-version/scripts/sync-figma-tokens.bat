@echo off
setlocal
chcp 65001 >nul

cd /d "%~dp0.."

echo ========================================
echo Aquilum - Figma token synchronization
echo ========================================
echo.
echo Working directory:
cd
echo.

call npm run tokens:sync
if errorlevel 1 goto :failed

call npm run tokens:diff
if errorlevel 1 goto :failed

call npm run check:figma-names
if errorlevel 1 goto :failed

call npm run check:colors
if errorlevel 1 goto :failed

call npm run check:tokens
if errorlevel 1 goto :failed

echo.
echo ========================================
echo Synchronization completed successfully.
echo ========================================
goto :done

:failed
echo.
echo ========================================
echo Synchronization failed.
echo Review the log above.
echo ========================================

:done
echo.
pause
endlocal
