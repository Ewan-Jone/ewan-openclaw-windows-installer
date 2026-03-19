@echo off
setlocal EnableDelayedExpansion
call "%~dp0..\config\temp_config.bat"

:: Get timestamp
for /f "tokens=1-4 delims=/ " %%a in ('date /t') do set "DATE_STAMP=%%a-%%b-%%c"
for /f "tokens=1-2 delims=: " %%a in ('time /t') do set "TIME_STAMP=%%a:%%b"
set "TS=%DATE_STAMP% %TIME_STAMP%"

echo [%TS%] BASE_URL=%BASE_URL% > %~dp0..\logs\set_config_debug.log
echo [%TS%] MODEL_NAME=%MODEL_NAME% >> %~dp0..\logs\set_config_debug.log
echo [%TS%] WEBCHAT_PORT=%WEBCHAT_PORT% >> %~dp0..\logs\set_config_debug.log

:: Build JSON config
set "JSON_CONFIG={\"baseUrl\":\"%BASE_URL%\",\"apiKey\":\"%API_KEY%\",\"api\":\"%API_PROTOCOL%\",\"models\":[{\"id\":\"%MODEL_NAME%\",\"name\":\"%MODEL_NAME%\"}]}"

echo [%TS%] Executing openclaw config commands... >> %~dp0..\logs\set_config_debug.log

:: Execute openclaw config commands directly via wsl
wsl -d ewan-openclaw -u root openclaw config set gateway.mode local >> %~dp0..\logs\set_config.log 2>&1
wsl -d ewan-openclaw -u root openclaw config set gateway.controlUi.allowInsecureAuth true >> %~dp0..\logs\set_config.log 2>&1
wsl -d ewan-openclaw -u root openclaw config set gateway.controlUi.dangerouslyAllowHostHeaderOriginFallback true >> %~dp0..\logs\set_config.log 2>&1
wsl -d ewan-openclaw -u root openclaw config set gateway.auth.mode none >> %~dp0..\logs\set_config.log 2>&1
wsl -d ewan-openclaw -u root openclaw config set gateway.port %WEBCHAT_PORT% >> %~dp0..\logs\set_config.log 2>&1
wsl -d ewan-openclaw -u root openclaw config set models.providers.custom "%JSON_CONFIG%" --strict-json >> %~dp0..\logs\set_config.log 2>&1
wsl -d ewan-openclaw -u root openclaw config set agents.defaults.model.primary "custom/%MODEL_NAME%" >> %~dp0..\logs\set_config.log 2>&1

echo [%TS%] Done, exitcode=%ERRORLEVEL% >> %~dp0..\logs\set_config_debug.log
