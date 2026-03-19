# openclaw-launcher

OpenClaw AI 助手的 Windows 系统托盘启动器。

负责管理运行在 WSL2 内的 OpenClaw Gateway 服务，提供图形化的状态反馈和控制界面。

## 功能

| 功能 | 说明 |
|------|------|
| 系统托盘图标 | 三色状态指示：🟡 启动中 / 🟢 运行中 / 🔴 已停止 |
| 健康检测 | 每 30 秒检测，连续 3 次失败自动重启 |
| 首次引导 | 弹窗提示用户完成 API Key 配置 |
| 开机自启 | 写入注册表 `HKCU\...\Run` |
| 快速打开 | 托盘菜单一键打开 `http://localhost:18789` |

## 项目结构

```
openclaw-launcher/
├── Cargo.toml
├── assets/
│   ├── icon-starting.png   # 黄色（启动中）
│   ├── icon-running.png    # 绿色（运行中）
│   └── icon-stopped.png    # 红色（已停止）
└── src/
    ├── main.rs             # 程序入口 & 主事件循环
    ├── config.rs           # 配置文件管理（%APPDATA%\OpenClaw\launcher.json）
    ├── wsl.rs              # WSL2 控制（启动/停止 Gateway）
    ├── health.rs           # 健康检测逻辑
    ├── tray.rs             # 系统托盘图标 & 菜单
    ├── autostart.rs        # 开机自启（注册表）
    └── onboarding.rs       # 首次运行引导弹窗
```

## 编译

### 前置条件

- Rust 工具链（`rustup` 安装）
- Windows GNU 目标：`rustup target add x86_64-pc-windows-gnu`
- mingw-w64：`sudo apt install mingw-w64`（WSL/Linux 交叉编译）

### 编译命令

```bash
# 语法检查
cargo check --target x86_64-pc-windows-gnu

# Debug 构建
cargo build --target x86_64-pc-windows-gnu

# Release 构建（最小体积）
cargo build --release --target x86_64-pc-windows-gnu

# 输出文件
target/x86_64-pc-windows-gnu/release/openclaw-launcher.exe
```

## 配置文件

位置：`%APPDATA%\OpenClaw\launcher.json`

```json
{
  "autostart": true,
  "wsl_distro": "openclaw",
  "gateway_port": 18789,
  "first_run": false
}
```

## 待完善事项

- [ ] 替换占位图标为正式设计图标（16×16 + 32×32）
- [ ] 添加 `build.rs` 设置 .exe 版本信息和图标（winresource）
- [ ] 考虑用 `windows-service` crate 封装为 Windows 服务
- [ ] 日志输出到文件（`%APPDATA%\OpenClaw\launcher.log`）
- [ ] 添加 GitHub Actions 自动构建 workflow
- [ ] WSL 发行版未安装时，引导用户运行安装程序
