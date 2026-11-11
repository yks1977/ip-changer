@echo off
setlocal
chcp 65001 >nul

title IP Changer - Build

echo ========================================
echo  IP Changer Build
echo ========================================
echo.

where cargo >nul 2>&1
if errorlevel 1 (
    echo [ERROR] cargo is not installed.
    pause
    exit /b 1
)

echo [1/3] Building...
cargo build --release
if errorlevel 1 (
    echo [ERROR] Build failed!
    pause
    exit /b 1
)

echo [2/3] Copying...
if not exist "%~dp0target\release\ip-changer.exe" (
    echo [ERROR] ip-changer.exe not found
    pause
    exit /b 1
)

copy /Y "%~dp0target\release\ip-changer.exe" "%~dp0IP Changer.exe" >nul
if errorlevel 1 (
    echo [ERROR] Copy failed
    pause
    exit /b 1
)

echo [3/3] Done: %~dp0IP Changer.exe
pause
endlocal