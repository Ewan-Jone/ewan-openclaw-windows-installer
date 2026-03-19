# changes-applied.md

本文档记录 2026-03-12 按照 `changes-2026-03-12.md` 实际执行的代码修改。

---

## 1. `scripts/setup_checks.pas`

**状态：已修改**

### 修改说明

将 `CurStepChanged` 中的 bat 脚本执行逻辑从 `ssInstall` 步骤移到 `ssPostInstall` 步骤：

- **原因**：`ssInstall` 触发时文件尚未拷贝到 `{app}`，check.bat 根本不存在；`ssPostInstall` 触发时文件已全部就位。
- 同时在脚本执行前加了 `ForceDirectories` 确保日志目录存在。
- 失败时改用 `Abort` 代替 `WizardForm.Close`，确保安装彻底中止，用户无法继续。
- 原 `ssPostInstall` 分支仅有 `SaveLog()` 调用，已合并到新的统一分支末尾。

### 原内容

```pascal
procedure CurStepChanged(CurStep: TSetupStep);
var
  AppPath, CheckBat, InstallBat, CheckLog, InstallLog, Args: String;
begin
  if CurStep = ssInstall then
  begin
    AppPath    := ExpandConstant('{app}');
    CheckBat   := AppPath + '\check.bat';
    InstallBat := AppPath + '\install.bat';
    CheckLog   := AppPath + '\logs\openclaw-check.log';
    InstallLog := AppPath + '\logs\openclaw-wsl-install.log';

    Log('app path: ' + AppPath);

    // Step 1: environment check
    WizardForm.StatusLabel.Caption := 'Checking environment...';
    Args := '"' + AppPath + '" "' + CheckLog + '"';
    if not RunBat('check.bat', CheckBat, CheckLog, Args) then
    begin
      WizardForm.Close;
      Exit;
    end;

    // Step 2: WSL distro install
    WizardForm.StatusLabel.Caption := 'Installing AI environment (1-2 min)...';
    Args := '"' + AppPath + '" "' + InstallLog + '" {#WslDistro} "' + AppPath + '\openclaw-rootfs.tar.gz"';
    if not RunBat('install.bat', InstallBat, InstallLog, Args) then
    begin
      WizardForm.Close;
      Exit;
    end;
  end;

  if CurStep = ssPostInstall then
  begin
    Log('=== setup complete ===');
    SaveLog();
  end;
end;
```

### 新内容

```pascal
procedure CurStepChanged(CurStep: TSetupStep);
var
  AppPath, CheckBat, InstallBat, CheckLog, InstallLog, Args: String;
begin
  if CurStep = ssPostInstall then
  begin
    AppPath    := ExpandConstant('{app}');
    CheckBat   := AppPath + '\check.bat';
    InstallBat := AppPath + '\install.bat';
    CheckLog   := AppPath + '\logs\openclaw-check.log';
    InstallLog := AppPath + '\logs\openclaw-wsl-install.log';

    Log('app path: ' + AppPath);

    // Ensure log directory exists before running any bat
    try ForceDirectories(AppPath + '\logs'); except end;

    // Step 1: environment check
    WizardForm.StatusLabel.Caption := 'Checking environment...';
    Args := '"' + AppPath + '" "' + CheckLog + '"';
    if not RunBat('check.bat', CheckBat, CheckLog, Args) then
    begin
      Abort;
      Exit;
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
```

---

## 2. `openclaw-launcher/src/wsl.rs`

**状态：无需修改（已是目标版本）**

需求文档要求将 `start_gateway` 的端口从写死 `17789` 改为参数传入：

```rust
// 目标签名
pub fn start_gateway(distro: &str, port: u16) -> Result<()>
```

检查源文件发现当前代码已经是该签名，改动在本次任务之前已完成，无需重复修改。

---

## 3. `openclaw-setup.iss`

**状态：无需修改（已是目标版本）**

需求文档要求去掉调试期间临时添加的"将 check.bat 复制到 `{tmp}`"条目。检查当前 `[Files]` 节，该条目不存在，清理在本次任务之前已完成，无需重复修改。

---

## 汇总

| 文件 | 操作 | 说明 |
|------|------|------|
| `scripts/setup_checks.pas` | **已修改** | bat 执行从 ssInstall → ssPostInstall，失败改用 Abort，加 ForceDirectories |
| `openclaw-launcher/src/wsl.rs` | 无需修改 | start_gateway 端口参数化已完成 |
| `openclaw-setup.iss` | 无需修改 | 多余 {tmp} 条目已清理 |
