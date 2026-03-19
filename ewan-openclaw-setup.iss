; ewan-openclaw-setup.iss  —  Ewan OpenClaw AI 助手 Windows 一键安装包
; 编译：Inno Setup 6.x  →  Build → Compile
;
; dist\ 目录须包含：
;   wsl.x64.msi                    WSL 本体离线包
;   wsl_update_x64.msi             WSL2 内核更新包
;   MicrosoftEdgeWebview2Setup.exe WebView2 常绿 Bootstrapper
;     下载：https://go.microsoft.com/fwlink/p/?LinkId=2124703

#define AppName    "Ewan OpenClaw AI 助手"
#define AppVersion "1.2.46"
#define AppURL     "https://openclaw.ai"
#define WslDistro  "ewan-openclaw"

[Setup]
AppId={{B2C3D4E5-F6A7-8901-BCDE-F12345678901}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=Ewan
AppPublisherURL={#AppURL}
DefaultDirName=D:\EwanOpenClaw
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
DisableDirPage=no
OutputDir=Output
OutputBaseFilename=ewan-openclaw-setup-{#AppVersion}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
; WizardResizable=yes  ; removed: obsolete in Inno Setup 6.x
PrivilegesRequired=admin
MinVersion=10.0.19041
VersionInfoVersion={#AppVersion}
VersionInfoDescription={#AppName} 安装程序

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: desktopicon; Description: "创建桌面快捷方式"; GroupDescription: "附加任务:"; Flags: checkedonce
Name: autostart;   Description: "开机时自动启动";   GroupDescription: "附加任务:"

[Files]
Source: "ewan-openclaw-launcher\target\x86_64-pc-windows-gnu\release\ewan-openclaw-launcher.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "build\openclaw-rootfs.tar.gz";           DestDir: "{app}"; Flags: ignoreversion
Source: "assets\icon-running.ico";                DestDir: "{app}"; Flags: ignoreversion
Source: "scripts\check.bat";                      DestDir: "{app}\scripts"; Flags: ignoreversion
Source: "scripts\install.bat";                     DestDir: "{app}\scripts"; Flags: ignoreversion
Source: "scripts\install_wsl_components.bat";      DestDir: "{app}\scripts"; Flags: ignoreversion
Source: "scripts\set_config.bat";                  DestDir: "{app}\scripts"; Flags: ignoreversion
Source: "scripts\restart.bat";                     DestDir: "{app}\scripts"; Flags: ignoreversion
Source: "scripts\install_gateway.bat";             DestDir: "{app}\scripts"; Flags: ignoreversion
Source: "config\*";                               DestDir: "{app}\config"; Flags: ignoreversion
Source: "dist\wsl.x64.msi";                       DestDir: "{app}\dist"; Flags: ignoreversion
Source: "dist\wsl_update_x64.msi";                DestDir: "{app}\dist"; Flags: ignoreversion

[Dirs]
Name: "{app}\scripts"
Name: "{app}\config"
Name: "{app}\logs"


[Icons]
Name: "{autodesktop}\AI 助手";                   Filename: "{app}\ewan-openclaw-launcher.exe"; Tasks: desktopicon
Name: "{autoprograms}\{#AppName}\AI 助手";        Filename: "{app}\ewan-openclaw-launcher.exe"
Name: "{autoprograms}\{#AppName}\卸载 AI 助手";  Filename: "{uninstallexe}"

[UninstallDelete]
; 卸载时清除安装目录下的运行时文件（launcher.json、logs\）
Type: filesandordirs; Name: "{app}\logs"
Type: files;          Name: "{app}\launcher.json"
; 兼容旧版：清除 %APPDATA%\EwanOpenClaw（旧版放在这里）
Type: filesandordirs; Name: "{userappdata}\EwanOpenClaw"

[Run]
Filename: "{app}\ewan-openclaw-launcher.exe"; Description: "立即启动 AI 助手"; Flags: nowait postinstall skipifsilent

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "EwanOpenClawLauncher"; ValueData: """{app}\ewan-openclaw-launcher.exe"""; Flags: uninsdeletevalue; Tasks: autostart

[Code]
#include "scripts\setup_checks.pas"
