@echo off
setlocal EnableDelayedExpansion
:: Switch to C:\Windows to avoid UNC path issues
cd /d C:\Windows

:: ============================================================
:: OpenClaw WSL install script
:: Args: %1=APP_DIR %2=LOG_FILE %3=DISTRO %4=ROOTFS
:: ============================================================

set "APP_DIR=%~1"
set "BAT_LOG=%~2"
set "DISTRO=%~3"
set "ROOTFS=%~4"
set "DISTRO_DIR=%APP_DIR%\%DISTRO%"

:: Default log path
if "%BAT_LOG%"=="" set "BAT_LOG=%APP_DIR%\logs\ewan-openclaw-wsl-install.log"

:: Ensure dirs exist (2>nul suppresses "already exists" errors)
md "%APP_DIR%\logs" 2>nul

:: Init log
echo [install.bat] === OpenClaw WSL install script start === > "%BAT_LOG%"

call :log "APP_DIR:    %APP_DIR%"
call :log "BAT_LOG:    %BAT_LOG%"
call :log "DISTRO:     %DISTRO%"
call :log "ROOTFS:     %ROOTFS%"
call :log "DISTRO_DIR: %DISTRO_DIR%"

:: Detect wsl.exe path
if exist "%SystemRoot%\Sysnative\wsl.exe" (
    set "WSL=%SystemRoot%\Sysnative\wsl.exe"
    call :log "WSL: %SystemRoot%\Sysnative\wsl.exe"
) else if exist "%SystemRoot%\System32\wsl.exe" (
    set "WSL=%SystemRoot%\System32\wsl.exe"
    call :log "WSL: %SystemRoot%\System32\wsl.exe"
) else (
    call :log "ERROR: wsl.exe not found"
    exit /b 1
)

:: [1] Check rootfs
call :log "[1] rootfs: %ROOTFS%"
if not exist "%ROOTFS%" (
    call :log "ERROR: rootfs not found"
    exit /b 2
)
for %%F in ("%ROOTFS%") do call :log "rootfs size: %%~zF bytes"

:: [2] WSL version
call :log "[2] wsl --version:"
"%WSL%" --version >> "%BAT_LOG%" 2>&1
call :log "wsl --version exit: %ERRORLEVEL%"

:: [3] List distros
call :log "[3] wsl --list --verbose:"
"%WSL%" --list --verbose >> "%BAT_LOG%" 2>&1

:: [4] Clean old distro (use reg query to avoid UTF-16 wsl --list issues)
call :log "[4] check old distro..."
reg query "HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Lxss" /s /f "%DISTRO%" >nul 2>&1
if %ERRORLEVEL% == 0 (
    call :log "old distro found, unregistering..."
    "%WSL%" --terminate %DISTRO% >> "%BAT_LOG%" 2>&1
    "%WSL%" --unregister %DISTRO% >> "%BAT_LOG%" 2>&1
    if exist "%DISTRO_DIR%" rmdir /s /q "%DISTRO_DIR%"
    ping -n 3 127.0.0.1 >nul 2>&1
) else (
    call :log "no old distro"
)

:: [5] Create distro dir
call :log "[5] mkdir: %DISTRO_DIR%"
md "%DISTRO_DIR%" 2>nul

:: [6] wsl --import via stdin
:: PowerShell  Start-Process  wsl --import
:: Feed rootfs via stdin to avoid shell file association popup
call :log "[6] wsl --import (via stdin) %DISTRO% %DISTRO_DIR%"
type "%ROOTFS%" | "%WSL%" --import "%DISTRO%" "%DISTRO_DIR%" - >> "%BAT_LOG%" 2>&1
set "IMPORT_EXIT=%ERRORLEVEL%"
call :log "wsl --import exit: %IMPORT_EXIT%"

if not %IMPORT_EXIT% == 0 (
    call :log "ERROR: wsl --import failed"
    exit /b %IMPORT_EXIT%
)

:: Poll registry until distro appears (max 30s)
call :log "[7] waiting for distro to register..."
set "WAIT_COUNT=0"
:wait_loop
reg query "HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Lxss" /s /f "%DISTRO%" >nul 2>&1
if %ERRORLEVEL% == 0 (
    call :log "distro registered after %WAIT_COUNT%s"
    ping -n 3 127.0.0.1 >nul 2>&1
    goto :distro_ready
)
if %WAIT_COUNT% geq 30 (
    call :log "ERROR: distro not registered after 30s"
    exit /b 3
)
ping -n 2 127.0.0.1 >nul 2>&1
set /a WAIT_COUNT+=1
goto :wait_loop

:distro_ready
:: [7] shutdown WSL service
call :log "[7] shutting down WSL service..."
"%WSL%" --shutdown >nul 2>&1
call :log "WSL shutdown done"

:: [7] Phase 1: wait for WSL service to restart (poll wsl --status, max 120s)
call :log "[7] waiting for WSL service restart..."
set "SVC_WAIT=0"
:svc_status_loop
"%WSL%" --status >nul 2>&1
if %ERRORLEVEL% == 0 (
    call :log "WSL service up after %SVC_WAIT%s"
    goto :svc_up
)
if %SVC_WAIT% GEQ 120 (
    call :log "ERROR: WSL service not up after 120s"
    exit /b 4
)
ping -n 3 127.0.0.1 >nul 2>&1
set /a SVC_WAIT+=2
goto :svc_status_loop

:svc_up
:: [7] Phase 2: poll wsl -d distro, max 60s
call :log "[7] WSL service ready, waiting for distro (max 60s)..."
call :log "[DEBUG] WSL=%WSL% DISTRO=%DISTRO%"
call :log "[DEBUG] CMD: %WSL% -d %DISTRO% -- echo ok"
set "D_WAIT=0"
:distro_probe_loop
"%WSL%" -d %DISTRO% -- echo ok 2>nul
if %ERRORLEVEL% == 0 (
    call :log "distro accessible after !D_WAIT!s"
    goto :wsl_ready
)
if !D_WAIT! GEQ 60 (
    call :log "ERROR: distro not accessible after 60s"
    exit /b 4
)
call :log "distro not ready (!D_WAIT!s)"
ping -n 4 127.0.0.1 >nul 2>&1
set /a D_WAIT+=3
goto :distro_probe_loop

:wsl_ready
call :log "=== install.bat done ==="
exit /b 0

:: Log function
:log
set "MSG=%~1"
set "TS=%DATE% %TIME%"
echo [%TS%] %MSG%
echo [%TS%] %MSG% >> "%BAT_LOG%" 2>nul
goto :eof
