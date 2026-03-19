@echo off
setlocal EnableDelayedExpansion
cd /d C:\Windows

:: ============================================================
:: install_wsl_components.bat
:: Install WSL and WSL2 kernel update, enable Windows features
:: Args: %1=APP_DIR %2=LOG_FILE
:: Exit codes:
::   0    = success, no restart needed
::   3010 = success, restart required
::   other = failure
:: ============================================================

set "APP_DIR=%~1"
set "BAT_LOG=%~2"

if "%BAT_LOG%"=="" set "BAT_LOG=%APP_DIR%\logs\ewan-openclaw-wsl-components.log"

md "%APP_DIR%\logs" 2>nul

call :log "[wsl-components] === install_wsl_components.bat start ==="
call :log "APP_DIR: %APP_DIR%"

set "NEED_REBOOT=0"
set "WSL_MSI=%APP_DIR%\dist\wsl.x64.msi"
set "KERNEL_MSI=%APP_DIR%\dist\wsl_update_x64.msi"

:: ---- Step 1: install wsl.x64.msi only if wsl.exe not found ----
call :log "[1] checking for wsl.exe..."
set "WSL_EXE="
if exist "%SystemRoot%\Sysnative\wsl.exe" set "WSL_EXE=%SystemRoot%\Sysnative\wsl.exe"
if "%WSL_EXE%"=="" if exist "%SystemRoot%\System32\wsl.exe" set "WSL_EXE=%SystemRoot%\System32\wsl.exe"

if "%WSL_EXE%"=="" (
    call :log "wsl.exe not found, installing wsl.x64.msi..."
    if not exist "%WSL_MSI%" (
        call :log "ERROR: %WSL_MSI% not found"
        exit /b 2
    )
    msiexec /i "%WSL_MSI%" /qn /norestart >> "%BAT_LOG%" 2>&1
    set "E=%ERRORLEVEL%"
    call :log "wsl.x64.msi exit: !E!"
    if !E! == 0 goto :wsl_msi_ok
    if !E! == 1638 goto :wsl_msi_ok
    if !E! == 1641 (set "NEED_REBOOT=1" && goto :wsl_msi_ok)
    if !E! == 3010 (set "NEED_REBOOT=1" && goto :wsl_msi_ok)
    call :log "ERROR: wsl.x64.msi failed with exit !E!"
    exit /b !E!
) else (
    call :log "wsl.exe found at %WSL_EXE%, skipping wsl.x64.msi"
)

:wsl_msi_ok

:: ---- Step 2: always install wsl_update_x64.msi ----
call :log "[2] installing wsl_update_x64.msi..."
if not exist "%KERNEL_MSI%" (
    call :log "ERROR: %KERNEL_MSI% not found"
    exit /b 2
)
msiexec /i "%KERNEL_MSI%" /qn /norestart >> "%BAT_LOG%" 2>&1
set "E=%ERRORLEVEL%"
call :log "wsl_update_x64.msi exit: !E!"
if %E% == 0 goto :kernel_msi_ok
if %E% == 1638 goto :kernel_msi_ok
if %E% == 1603 goto :kernel_msi_ok
if %E% == 1641 (set "NEED_REBOOT=1" && goto :kernel_msi_ok)
if %E% == 3010 (set "NEED_REBOOT=1" && goto :kernel_msi_ok)
call :log "ERROR: wsl_update_x64.msi failed with exit %E%"
exit /b %E%

:kernel_msi_ok
call :log "wsl_update_x64.msi ok"

:: ---- Step 3: check if WSL optional components are enabled ----
call :log "[3] checking WSL optional components..."
if "%WSL_EXE%"=="" (
    if exist "%SystemRoot%\Sysnative\wsl.exe" set "WSL_EXE=%SystemRoot%\Sysnative\wsl.exe"
    if "%WSL_EXE%"=="" if exist "%SystemRoot%\System32\wsl.exe" set "WSL_EXE=%SystemRoot%\System32\wsl.exe"
)

"%WSL_EXE%" --list >nul 2>&1
set "WSL_LIST_EXIT=%ERRORLEVEL%"
call :log "wsl --list exit: %WSL_LIST_EXIT%"

if %WSL_LIST_EXIT% == 0 (
    call :log "WSL optional components OK, skipping dism"
    goto :components_ok
)

call :log "WSL optional components not enabled, running dism..."
dism /online /enable-feature /featurename:Microsoft-Windows-Subsystem-Linux /all /norestart >> "%BAT_LOG%" 2>&1
call :log "dism WSL feature exit: %ERRORLEVEL%"
dism /online /enable-feature /featurename:VirtualMachinePlatform /all /norestart >> "%BAT_LOG%" 2>&1
call :log "dism VirtualMachinePlatform exit: %ERRORLEVEL%"
set "NEED_REBOOT=1"

:components_ok

if %NEED_REBOOT% == 1 (
    call :log "[wsl-components] done - REBOOT REQUIRED"
    exit /b 3010
)

call :log "[wsl-components] done - no reboot needed"
exit /b 0

:log
set "MSG=%~1"
set "TS=%DATE% %TIME%"
echo [%TS%] %MSG%
echo [%TS%] %MSG% >> "%BAT_LOG%" 2>nul
goto :eof
