@echo off
setlocal EnableDelayedExpansion
cd /d C:\Windows

:: ============================================================
:: OpenClaw environment check script
:: Args: %1=APP_DIR %2=LOG_FILE
:: Exit codes:
::   0 = all checks passed
::   1 = Windows version too old (< 10.0.19041)
::   2 = wsl.exe not found
:: ============================================================

set "APP_DIR=%~1"
set "CHK_LOG=%~2"

if "%CHK_LOG%"=="" set "CHK_LOG=%APP_DIR%\logs\ewan-openclaw-check.log"

md "%APP_DIR%\logs" 2>nul

echo [check.bat] === OpenClaw environment check start === > "%CHK_LOG%"

call :log "APP_DIR: %APP_DIR%"

:: [1] Windows version check via registry (more reliable than parsing ver output)
call :log "[1] checking Windows version..."
set "WIN_BUILD="
for /f "tokens=3" %%a in ('reg query "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion" /v CurrentBuildNumber 2^>nul') do set "WIN_BUILD=%%a"
call :log "Windows build: %WIN_BUILD%"
if "%WIN_BUILD%"=="" (
    call :log "WARNING: could not read build number, skipping version check"
    goto :version_ok
)
if %WIN_BUILD% LSS 19041 (
    call :log "ERROR: Windows build %WIN_BUILD% is too old, need >= 19041"
    exit /b 1
)
:version_ok
call :log "Windows version OK"

:: [2] Check wsl.exe exists
call :log "[2] checking wsl.exe..."
if exist "%SystemRoot%\System32\wsl.exe" (
    set "WSL=%SystemRoot%\System32\wsl.exe"
    call :log "wsl.exe found: System32\wsl.exe"
) else if exist "%SystemRoot%\Sysnative\wsl.exe" (
    set "WSL=%SystemRoot%\Sysnative\wsl.exe"
    call :log "wsl.exe found: Sysnative\wsl.exe"
) else (
    call :log "ERROR: wsl.exe not found - WSL is not installed"
    exit /b 2
)

:: [3] WSL version info
call :log "[3] wsl --version:"
"%WSL%" --version >> "%CHK_LOG%" 2>&1
call :log "wsl --version exit: %ERRORLEVEL%"

:: [4] Fix .wslconfig LF line endings
call :log "[4] checking .wslconfig files..."
for %%U in ("%USERPROFILE%" "C:\Users\Administrator") do (
    if exist "%%~U\.wslconfig" (
        call :log "found .wslconfig: %%~U\.wslconfig"
        powershell -NonInteractive -NoProfile -Command "$f='%%~U\.wslconfig'; $c=[System.IO.File]::ReadAllText($f); $fixed=$c -replace '(?<!\r)\n',\"`r`n\"; if ($c -ne $fixed) { [System.IO.File]::WriteAllText($f,$fixed); Write-Output 'fixed CRLF' } else { Write-Output 'already CRLF' }" >> "%CHK_LOG%" 2>&1
        call :log ".wslconfig fix exit: %ERRORLEVEL%"
    )
)

call :log "=== check.bat done: all checks passed ==="
exit /b 0

:log
set "MSG=%~1"
set "TS=%DATE% %TIME%"
echo [%TS%] %MSG%
echo [%TS%] %MSG% >> "%CHK_LOG%" 2>nul
goto :eof
