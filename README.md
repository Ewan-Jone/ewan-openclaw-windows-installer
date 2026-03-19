# ewan-openclaw-windows-installer

**Ewan OpenClaw AI 助手 · Windows 一键安装包构建工程**

让普通 Windows 用户无需任何技术背景，一键完成 OpenClaw AI 助手的安装和配置。安装程序会自动完成 WSL 环境搭建、服务启动和首次配置引导，整个过程约 2–3 分钟。

---

## 项目是什么

本项目负责将 OpenClaw AI 助手打包成 Windows 标准安装程序（`.exe`），核心做了三件事：

1. **把 Linux 运行环境打进安装包** — 预构建好的 WSL 镜像（`ewan-openclaw-rootfs.tar.gz`）随安装包一起分发，用户无需手动配置 WSL。
2. **自动完成 WSL Distro 导入与服务启动** — 安装过程中静默执行环境检查、WSL distro 注册、systemd 服务创建。
3. **提供 Windows 侧的托盘管理程序** — `ewan-openclaw-launcher.exe`（Rust 编写）在系统托盘运行，负责监控服务健康、首次配置引导、开机自启。

---

## 项目结构

```
ewan-openclaw-windows-installer/
├── ewan-openclaw-setup.iss          # Inno Setup 主打包脚本
├── preview-onboarding.html          # 首次配置引导页面（浏览器展示）
├── build.json                       # 构建时组件下载地址配置
├── build/
│   └── ewan-openclaw-rootfs.tar.gz  # 预打包的 Linux 环境（需自行准备，见下）
├── ewan-openclaw-launcher/          # Windows 托盘管理程序（Rust）
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                  # 入口：launcher 主循环
│       ├── config.rs                # 配置读写
│       ├── wsl.rs                   # WSL distro 管理
│       ├── health.rs                # OpenClaw 服务健康检查
│       ├── tray.rs                  # 系统托盘图标与菜单
│       ├── autostart.rs             # 开机自启配置
│       └── onboarding.rs            # 首次配置引导本地 HTTP server
├── assets/
│   └── icon-running.ico
└── scripts/
    ├── check.bat                    # 环境检查（Windows 版本、WSL 是否可用）
    ├── install.bat                  # WSL distro 安装与服务启动
    ├── install_gateway.bat          # OpenClaw gateway 安装脚本
    ├── install_wsl_components.bat   # WSL 组件离线安装
    ├── set_config.bat               # 写入初始配置
    ├── restart.bat                  # 重启 OpenClaw 服务
    └── setup_checks.pas             # Inno Setup 调度脚本（Pascal）
```

---

## 构建步骤

### 前置依赖

| 工具 | 用途 |
|------|------|
| [Inno Setup 6.x](https://jrsoftware.org/isinfo.php) | 打包 Windows 安装程序 |
| Rust + Cargo | 编译 ewan-openclaw-launcher |
| WSL2 | 脚本开发与本地测试 |

---

### 步骤一：准备 rootfs

将预构建的 Linux 环境镜像放到：

```
build/ewan-openclaw-rootfs.tar.gz
```

> rootfs 需要单独构建（参见 openclaw-rootfs 项目），约 200–300 MB。

---

### 步骤二：编译 Launcher（Rust 跨平台编译）

在 WSL 中交叉编译 Windows 可执行文件：

```bash
cd ewan-openclaw-launcher

# 首次需要安装 Windows 目标
rustup target add x86_64-pc-windows-gnu

# 编译
cargo build --release --target x86_64-pc-windows-gnu
```

输出：`ewan-openclaw-launcher/target/x86_64-pc-windows-gnu/release/ewan-openclaw-launcher.exe`

> 也可以在 Windows 本机用 `cargo build --release` 直接编译，输出在 `target/release/`。

---

### 步骤三：打包安装程序

**方式一：从 WSL 调用（推荐）**

```bash
/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe \
  -NonInteractive -NoProfile -Command \
  "& 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe' \
  '$(wslpath -w /home/zym/.openclaw/workspace/projects/ewan-openclaw-windows-installer/ewan-openclaw-setup.iss)'"
```

**方式二：直接在 Windows 上编译**

```
ISCC.exe ewan-openclaw-setup.iss
```

打包完成后输出：

```
Output/ewan-openclaw-setup-x.x.x.exe
```

---

## 安装流程说明（终端用户视角）

1. **环境检查**（`check.bat`）：确认 Windows 版本 ≥ 19041，WSL 可用，修复 `.wslconfig` 换行符问题
2. **WSL Distro 安装**（`install.bat`）：将 rootfs 通过 `wsl --import` 注册为 `ewan-openclaw` distro，启动 systemd 服务，挂载 workspace 目录
3. **首次配置**（`ewan-openclaw-launcher`）：安装后自动打开浏览器引导页，填写 API Key、端口等配置，配置完成后正式启动

---

## 系统要求

**终端用户：**
- Windows 10 Build 19041 或更高 / Windows 11
- WSL2 已启用（`wsl --install`）
- 至少 2 GB 可用磁盘空间

**开发构建：**
- Inno Setup 6.x（Windows）
- Rust 1.70+ with `x86_64-pc-windows-gnu` target
- WSL2（用于脚本开发和测试）

---

## 日志位置

安装过程中所有日志保存到：

- `{安装目录}\logs\ewan-openclaw-check.log`
- `{安装目录}\logs\ewan-openclaw-wsl-install.log`
- `%USERPROFILE%\Desktop\ewan-openclaw-logs\`（安装结束后自动复制）
