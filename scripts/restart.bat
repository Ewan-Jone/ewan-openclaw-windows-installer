@echo off
:: ================================================
:: Restart Ewan OpenClaw Gateway Service
:: ================================================
:: IMPORTANT: Keep this window open to keep the service running!
:: When you close this window, the AI assistant will stop.
:: This script will restart the gateway to apply config changes.

echo.
echo ================================================
echo  Restarting Ewan OpenClaw Gateway
echo ================================================
echo.
echo IMPORTANT: Keep this window open to keep the service running!
echo When you close this window, the AI assistant will stop.
echo.

:: Load config from temp_config.bat (if exists)
if exist "%~dp0..\config\temp_config.bat" (
    call "%~dp0..\config\temp_config.bat"
    echo Loaded config: WEBCHAT_PORT=%WEBCHAT_PORT%
    echo.
) else (
    echo Warning: temp_config.bat not found, using default settings
    echo.
)

:: Re-install gateway to pick up any config changes (like port)
call "%~dp0install_gateway.bat"

:: Restart the gateway
echo Restarting gateway...
wsl -d ewan-openclaw bash -c "openclaw daemon restart; exec bash"

echo.
echo Gateway restart completed.
echo Keep this window open to continue using the AI assistant.
pause
