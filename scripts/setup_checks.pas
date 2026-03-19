// setup_checks.pas
// Inno Setup Pascal script - thin scheduler only
// All install logic lives in bat scripts; this file just runs them in order.

var
  LogFile: String;

// ─── Logging ───────────────────────────────────────────────────────────────

procedure LogInit();
begin
  LogFile := ExpandConstant('{tmp}\ewan-openclaw-install.log');
end;

procedure Log(Msg: String);
var
  TimeStr, Line: String;
begin
  if LogFile = '' then LogInit();
  try
    TimeStr := GetDateTimeString('yyyy-mm-dd hh:nn:ss', '-', ':');
    Line := '[' + TimeStr + '] ' + Msg + #13#10;
    SaveStringToFile(LogFile, Line, True);
  except
  end;
end;

// ─── Save logs to desktop ewan-openclaw-logs\ ───────────────────────────────

procedure SaveLog();
var
  DesktopDir, AppLogDir: String;
begin
  if (LogFile = '') or (not FileExists(LogFile)) then Exit;

  DesktopDir := ExpandConstant('{userdesktop}\ewan-openclaw-logs');
  AppLogDir  := ExpandConstant('{app}\logs');

  try ForceDirectories(AppLogDir); except end;
  try CopyFile(LogFile, AppLogDir + '\install.log', False); except end;

  try
    ForceDirectories(DesktopDir);
    CopyFile(LogFile, DesktopDir + '\install.log', False);
    // check.bat log
    if FileExists(AppLogDir + '\ewan-openclaw-check.log') then
      CopyFile(AppLogDir + '\ewan-openclaw-check.log', DesktopDir + '\check.log', False);
    // install.bat log
    if FileExists(AppLogDir + '\ewan-openclaw-wsl-install.log') then
      CopyFile(AppLogDir + '\ewan-openclaw-wsl-install.log', DesktopDir + '\wsl-install.log', False);
    // fallback logs from %TEMP%
    if FileExists(ExpandConstant('{tmp}\ewan-openclaw-check.log')) then
      CopyFile(ExpandConstant('{tmp}\ewan-openclaw-check.log'), DesktopDir + '\check.log', False);
    if FileExists(ExpandConstant('{tmp}\ewan-openclaw-wsl-install.log')) then
      CopyFile(ExpandConstant('{tmp}\ewan-openclaw-wsl-install.log'), DesktopDir + '\wsl-install.log', False);
  except
  end;
end;

// ─── Read last N lines from a log file ─────────────────────────────────────

function ReadLastLines(FilePath: String; MaxLines: Integer): String;
var
  Lines: TArrayOfString;
  I, StartIdx: Integer;
begin
  Result := '';
  if not FileExists(FilePath) then
  begin
    Result := '(log file not found: ' + FilePath + ')';
    Exit;
  end;
  if not LoadStringsFromFile(FilePath, Lines) then
  begin
    Result := '(failed to read log file)';
    Exit;
  end;
  StartIdx := GetArrayLength(Lines) - MaxLines;
  if StartIdx < 0 then StartIdx := 0;
  for I := StartIdx to GetArrayLength(Lines) - 1 do
    Result := Result + Lines[I] + #13#10;
end;

// ─── Run a bat script and show error dialog on failure ─────────────────────

function RunBat(BatName, BatPath, LogPath, ExtraArgs: String): Boolean;
var
  CmdExe, CmdArgs: String;
  ExitCode: Integer;
  Output: String;
begin
  Result := True;
  Log('running ' + BatName + ': ' + BatPath);

  CmdExe := ExpandConstant('{win}\Sysnative\cmd.exe');
  if not FileExists(CmdExe) then
    CmdExe := ExpandConstant('{sys}\cmd.exe');

  CmdArgs := '/C ""' + BatPath + '" ' + ExtraArgs + '"';

  Exec(CmdExe, CmdArgs, '', SW_HIDE, ewWaitUntilTerminated, ExitCode);
  Log(BatName + ' exit: ' + IntToStr(ExitCode));

  if ExitCode <> 0 then
  begin
    Result := False;
    SaveLog();
    Output := ReadLastLines(LogPath, 40);
    MsgBox(
      BatName + ' failed (exit code: ' + IntToStr(ExitCode) + ')' + #13#10#13#10 +
      '--- output ---' + #13#10 +
      Output + #13#10 +
      '--- log saved to desktop\ewan-openclaw-logs\ ---',
      mbError, MB_OK);
  end;
end;

// ─── Validate install path (local NTFS only) ───────────────────────────────

function ValidateInstallPath(Path: String): Boolean;
var
  TestFile: String;
begin
  Result := True;
  if Length(Path) < 2 then Exit;

  // Must be on a local fixed drive
  if not (Copy(Path, 2, 1) = ':') then
  begin
    MsgBox('Installation path must be on a local drive (e.g. C:\...).', mbError, MB_OK);
    Result := False;
    Exit;
  end;

  // Write permission test
  try
    ForceDirectories(Path);
  except
  end;
  TestFile := Path + '\ewan-openclaw-write-test.tmp';
  if not SaveStringToFile(TestFile, 'test', False) then
  begin
    MsgBox(
      'Cannot write to installation directory:' + #13#10 + Path + #13#10#13#10 +
      'Please choose a directory where the current user has write permission.',
      mbError, MB_OK);
    Result := False;
    Exit;
  end;
  DeleteFile(TestFile);
end;

// ─── InitializeSetup ───────────────────────────────────────────────────────

function InitializeSetup(): Boolean;
begin
  LogInit();
  Log('=== Ewan OpenClaw setup start ===');
  Result := True;
end;

// ─── CurStepChanged: run bat scripts in order ──────────────────────────────

procedure CurStepChanged(CurStep: TSetupStep);
var
  AppPath, CheckBat, InstallBat, CheckLog, InstallLog, Args: String;
  WslCompBat, WslCompLog, CmdExe, CmdArgs: String;
  ExitCode: Integer;
begin
  if CurStep = ssPostInstall then
  begin
    AppPath    := ExpandConstant('{app}');
    CheckBat   := AppPath + '\scripts\check.bat';
    InstallBat := AppPath + '\scripts\install.bat';
    CheckLog   := AppPath + '\logs\ewan-openclaw-check.log';
    InstallLog := AppPath + '\logs\ewan-openclaw-wsl-install.log';

    Log('app path: ' + AppPath);

    // Ensure log directory exists before running any bat
    try ForceDirectories(AppPath + '\logs'); except end;

    // 清除旧版 launcher.json（旧版放在 %APPDATA%\EwanOpenClaw\，新版在 {app}\）
    // 两处都清，防止残留端口/配置污染新安装
    if FileExists(ExpandConstant('{userappdata}\EwanOpenClaw\launcher.json')) then
    begin
      Log('removing old launcher.json from appdata');
      DeleteFile(ExpandConstant('{userappdata}\EwanOpenClaw\launcher.json'));
    end;
    if FileExists(ExpandConstant('{app}\launcher.json')) then
    begin
      Log('removing old launcher.json from app dir');
      DeleteFile(ExpandConstant('{app}\launcher.json'));
    end;

    // Step 1: environment check
    WizardForm.StatusLabel.Caption := 'Checking environment...';
    Args := '"' + AppPath + '" "' + CheckLog + '"';
    if not RunBat('check.bat', CheckBat, CheckLog, Args) then
    begin
      Abort;
      Exit;
    end;

    // Step 1.5: install WSL components (wsl.x64.msi + wsl_update_x64.msi)
    begin
      WslCompBat := AppPath + '\scripts\install_wsl_components.bat';
      WslCompLog := AppPath + '\logs\ewan-openclaw-wsl-components.log';
      WizardForm.StatusLabel.Caption := 'Installing WSL components...';
      Log('running install_wsl_components.bat');
      CmdExe := ExpandConstant('{win}\Sysnative\cmd.exe');
      if not FileExists(CmdExe) then
        CmdExe := ExpandConstant('{sys}\cmd.exe');
      CmdArgs := '/C ""' + WslCompBat + '" "' + AppPath + '" "' + WslCompLog + '""';
      Exec(CmdExe, CmdArgs, '', SW_HIDE, ewWaitUntilTerminated, ExitCode);
      Log('install_wsl_components.bat exit: ' + IntToStr(ExitCode));
      // 3010 = success but reboot required
      if ExitCode = 3010 then
      begin
        SaveLog();
        MsgBox(
          'WSL components were installed successfully, but a system restart is required.' + #13#10#13#10 +
          'Please restart your computer, then run the installer again to complete the installation.' + #13#10 +
          'The installer will automatically skip the already-completed steps.',
          mbInformation, MB_OK);
        Abort;
        Exit;
      end;
      // any other non-zero = failure
      if ExitCode <> 0 then
      begin
        SaveLog();
        MsgBox(
          'install_wsl_components.bat failed (exit code: ' + IntToStr(ExitCode) + ')' + #13#10#13#10 +
          ReadLastLines(WslCompLog, 30) + #13#10 +
          '--- log saved to desktop\ewan-openclaw-logs\ ---',
          mbError, MB_OK);
        Abort;
        Exit;
      end;
    end;

    // Step 2: WSL distro install
    WizardForm.StatusLabel.Caption := 'Installing AI environment (1-2 min)...';
    Args := '"' + AppPath + '" "' + InstallLog + '" {#WslDistro} "' + AppPath + '\openclaw-rootfs.tar.gz"';
    if not RunBat('install.bat', InstallBat, InstallLog, Args) then
    begin
      Abort;
      Exit;
    end;

    Log('=== setup complete ===');
    SaveLog();
  end;
end;

// ─── CurUninstallStepChanged ───────────────────────────────────────────────

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  ResultCode: Integer;
  DistroDir, AppDataDir, WslExe: String;
begin
  if CurUninstallStep = usUninstall then
  begin
    Log('=== uninstall start ===');
    WslExe := ExpandConstant('{win}\Sysnative\wsl.exe');
    if not FileExists(WslExe) then
      WslExe := ExpandConstant('{sys}\wsl.exe');

    if FileExists(WslExe) then
    begin
      Exec(WslExe, '--terminate {#WslDistro}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
      Exec(WslExe, '--unregister {#WslDistro}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    end;

    DistroDir := ExpandConstant('{app}\{#WslDistro}');
    if DirExists(DistroDir) then DelTree(DistroDir, True, True, True);

    AppDataDir := ExpandConstant('{userappdata}\EwanOpenClaw');
    if DirExists(AppDataDir) then DelTree(AppDataDir, True, True, True);

    RegDeleteValue(HKEY_CURRENT_USER,
      'Software\Microsoft\Windows\CurrentVersion\Run', 'EwanOpenClawLauncher');
    RegDeleteValue(HKEY_LOCAL_MACHINE,
      'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{B2C3D4E5-F6A7-8901-BCDE-F12345678901}_is1',
      'InstallLocation');

    Log('=== uninstall done ===');
    SaveLog();
  end;
end;

// ─── NextButtonClick: validate install path ────────────────────────────────

function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;
  if CurPageID = wpSelectDir then
  begin
    Result := ValidateInstallPath(WizardDirValue());
    if not Result then
      Log('install path validation failed: ' + WizardDirValue());
  end;
end;

// ─── UpdateReadyMemo ───────────────────────────────────────────────────────

function UpdateReadyMemo(Space, NewLine, MemoUserInfoInfo, MemoDirInfo,
  MemoTypeInfo, MemoComponentsInfo, MemoGroupInfo, MemoTasksInfo: String): String;
begin
  Result := 'Setup is ready to install Ewan OpenClaw AI Assistant.' + NewLine + NewLine;
  Result := Result + 'Steps:' + NewLine;
  Result := Result + Space + '1. Run check.bat (environment check)' + NewLine;
  Result := Result + Space + '2. Install WSL components (offline, silent)' + NewLine;
  Result := Result + Space + '3. Run install.bat (WSL distro install, ~1-2 min)' + NewLine;
  Result := Result + NewLine + 'Do not close the window during installation.';
  if MemoTasksInfo <> '' then
    Result := Result + NewLine + NewLine + MemoTasksInfo;
end;
