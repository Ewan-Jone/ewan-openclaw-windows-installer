# OpenClaw Windows Installer 改动文档 (v1.2.16)

## 改动日期
2026-03-12

## 改动概述

### 1. 新 rootfs

- 基于原始 rootfs（带 systemd=true）
- wsl.conf boot command 修改为：
  ```
  command=mkdir -p /run/user/0 && openclaw gateway install && openclaw daemon restart
  ```
- 启动时自动安装并启动 gateway

### 2. Launcher 简化

#### 移除的功能
- 健康检测循环（不再每 30 秒检查 gateway 状态）
- 托盘状态图标实时更新（不再显示黄/红/绿状态）
- 退出时停止 gateway

#### 简化的操作
- **配置更新后**：只执行 `openclaw daemon restart`，不等待结果
- **托盘菜单 - 重启**：只执行 `openclaw daemon restart`，不等待结果
- **托盘菜单 - 退出**：直接退出，不停止 gateway

### 3. wsl.rs 改动

所有以下函数保留但不再被 main.rs 调用：
- `stop_gateway()`
- `start_gateway()`
- `is_gateway_running()`
- `wait_gateway_stopped()`

未来可考虑移除这些未使用的代码。

## 测试要点

1. 首次安装后，gateway 应自动启动
2. 配置页保存后，gateway 应自动重启
3. 托盘菜单"重启"应能重启 gateway
4. 托盘图标保持静态（无状态颜色变化）

## 文件清单

| 文件 | 改动 |
|------|------|
| `build/openclaw-rootfs.tar.gz` | 新增 wsl.conf boot command |
| `openclaw-launcher/src/main.rs` | 简化健康检测和托盘状态 |
| `openclaw-setup.iss` | 版本号 1.2.16 |

## 备份

- 原始 launcher: `openclaw-launcher.bak.20260312_2112/`
- 原始 rootfs: `build/openclaw-rootfs.tar.gz.bak.20260312_143906`
- 原始 iss: `openclaw-setup.iss.bak.20260312_2112`
