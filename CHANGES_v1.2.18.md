# OpenClaw Windows Installer 改动文档 (v1.2.18)

## 改动日期
2026-03-12

## 改动概述

### 1. 新 rootfs

- 基于原始 rootfs（带 systemd=true）
- wsl.conf boot command:
  ```
  command=mkdir -p /run/user/0 && openclaw gateway install && openclaw daemon restart
  ```
- 启动时自动安装并启动 gateway
- rootfs 大小：235MB（比原始 249MB 小）

### 2. Launcher 简化

#### 移除的功能
- ❌ 健康检测循环（不再每 30 秒检查 gateway 状态）
- ❌ 托盘状态图标实时更新（不再显示黄/红/绿状态）
- ❌ 退出时停止 gateway
- ❌ tokio 异步运行时（不再需要）

#### 简化的操作
- **配置更新后**：只执行 `openclaw daemon restart`，不等待结果
- **托盘菜单 - 重启**：只执行 `openclaw daemon restart`
- **托盘菜单 - 退出**：直接退出，不停止 gateway

### 3. wsl.rs

以下函数保留但未被调用（可后续清理）：
- `stop_gateway()`
- `start_gateway()`
- `is_gateway_running()`
- `wait_gateway_stopped()`

## 测试要点

1. ✅ 首次安装后，gateway 应自动启动（通过 wsl.conf）
2. ✅ 配置页保存后，应自动触发 daemon restart
3. ✅ 托盘菜单"重启"应能触发 daemon restart
4. ✅ 托盘图标保持静态（无状态颜色变化）
5. ✅ 日志中不再有 "Timeout waiting for gateway to stop" 或 "Starting OpenClaw Gateway"

## 文件清单

| 文件 | 改动 |
|------|------|
| `build/openclaw-rootfs.tar.gz` | 新 wsl.conf boot command (235MB) |
| `openclaw-launcher/src/main.rs` | 移除健康检测，简化托盘操作 |
| `openclaw-setup.iss` | 版本号 1.2.18 |

## 备份

- 原始 launcher: `openclaw-launcher.bak.20260312_2112/`
- 原始 rootfs: `build/openclaw-rootfs.tar.gz.bak.20260312_143906`
- 原始 iss: `openclaw-setup.iss.bak.20260312_2112`
