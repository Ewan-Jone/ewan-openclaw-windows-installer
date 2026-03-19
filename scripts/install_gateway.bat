@echo off
setlocal

:: Read port from config file if it exists, otherwise use default
set "PORT=17789"
set "CONFIG_FILE=%~dp0..\config\temp_config.bat"

if exist "%CONFIG_FILE%" (
    call "%CONFIG_FILE%"
    if defined WEBCHAT_PORT (
        set "PORT=%WEBCHAT_PORT%"
    )
)

echo Installing gateway with port %PORT%...

:: Install gateway with dynamic port
wsl -d ewan-openclaw bash -c "openclaw gateway install --port %PORT% --force" >> %~dp0..\logs\install_gateway.log 2>&1
