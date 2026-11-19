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

rem In the D:\claude workspace the exe is built into the shared target dir; ask cargo where it is.
set "TGT=%~dp0target"
for /f "usebackq delims=" %%T in (`powershell -NoProfile -Command "(cargo metadata --format-version 1 --no-deps --manifest-path '%~dp0Cargo.toml' 2>$null | ConvertFrom-Json).target_directory"`) do set "TGT=%%T"
echo [2/3] Copying...
if not exist "%TGT%\release\ip-changer.exe" (
    echo [ERROR] ip-changer.exe not found
    pause
    exit /b 1
)

copy /Y "%TGT%\release\ip-changer.exe" "%~dp0IP Changer.exe" >nul
if errorlevel 1 (
    echo [ERROR] Copy failed
    pause
    exit /b 1
)

echo [3/3] Done: %~dp0IP Changer.exe
pause
endlocal