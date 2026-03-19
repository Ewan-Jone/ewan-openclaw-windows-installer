// Windows GUI 程序：不显示控制台窗口
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

//! main.rs
//! Ewan OpenClaw Launcher — Windows 系统托盘托管程序
//!
//! 程序入口，负责协调各模块完成：
//! 1. 初始化日志
//! 2. 加载配置
//! 3. 检查 WSL 发行版
//! 4. 首次引导
//! 5. 启动 Gateway
//! 6. 创建托盘图标
//! 7. 健康检测循环
//! 8. 处理托盘菜单事件

mod autostart;
mod config;
mod health;
mod onboarding;
mod tray;
mod wsl;



use std::os::windows::process::CommandExt;
use std::time::Duration;
use anyhow::{Context, Result};
use tray_icon::menu::MenuEvent;
use tracing::{info, warn};
use tracing_appender::non_blocking::WorkerGuard;

use crate::config::Config;

use crate::onboarding::OnboardingServer;
use crate::tray::{handle_menu_event, TrayAction, TrayManager};

/// 日志目录：{exe所在目录}\logs
pub fn log_dir() -> std::path::PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    exe_dir.join("logs")
}

fn main() -> Result<()> {
    // ── 1. 初始化日志（同时输出到文件和 stderr）─────────────────────────────
    let log_directory = log_dir();
    std::fs::create_dir_all(&log_directory).ok();

    // 文件日志：launcher.log（非阻塞写入）
    let file_appender = tracing_appender::rolling::never(&log_directory, "launcher.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 同时输出到文件和 stderr（debug 时方便看）
    use tracing_subscriber::prelude::*;
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false);
    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(false);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with(file_layer)
        .with(stderr_layer)
        .init();

    // _guard 必须持有到 main 结束，否则日志 worker 线程提前退出
    // 用 Box::leak 让它存活整个进程生命周期
    let _log_guard: &'static WorkerGuard = Box::leak(Box::new(_guard));

    info!("Ewan OpenClaw Launcher starting...");
    info!("Log directory: {}", log_directory.display());

    // ── 2. 加载配置 ──────────────────────────────────────────────────────────
    let mut config = Config::load().unwrap_or_else(|e| {
        warn!("Failed to load config, using defaults: {}", e);
        Config::default()
    });

    info!(
        "Config loaded: distro={}, port={}, first_run={}",
        config.wsl_distro, config.gateway_port, config.first_run
    );

    // ── 3. 检查 WSL 发行版 ───────────────────────────────────────────────────
    if !wsl::is_distro_installed(&config.wsl_distro) {
        let msg = format!(
            "未找到 Ewan OpenClaw WSL 发行版「{}」。\n\n\
            请重新安装 Ewan OpenClaw 或联系技术支持。",
            config.wsl_distro
        );
        onboarding::show_error("AI 助手 - 安装错误", &msg);
        return Ok(());
    }

    // ── 3.5 安装 Gateway（如未安装）──────────────────────────────────────────
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let install_gateway_bat = exe_dir.join("scripts").join("install_gateway.bat");
    if install_gateway_bat.exists() {
        info!("Calling install_gateway.bat...");
        let _ = std::process::Command::new("cmd")
            .args(["/c", &install_gateway_bat.to_string_lossy()])
            .creation_flags(0x0800_0000)
            .spawn();
    }

    // ── 4. 启动常驻配置页服务 ────────────────────────────────────────────────
    let onboarding_server = OnboardingServer::start()
        .unwrap_or_else(|e| {
            warn!("Onboarding server failed to start: {}", e);
            // 兜底：返回一个无效的服务（不影响主流程）
            panic!("Cannot start onboarding server: {}", e);
        });

    // 首次运行：自动打开配置页，等待用户完成配置
    if config.first_run {
        info!("First run: opening onboarding page");
        onboarding_server.open();

        // 阻塞等待用户完成配置（最多5分钟）
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(300);
        loop {
            if start.elapsed() > timeout {
                warn!("Onboarding timed out, skipping");
                break;
            }
            if let Some(oc) = onboarding_server.try_recv() {
                info!("Onboarding complete, writing config...");
                
                // 只写入临时配置文件，不执行 wsl 命令
                let exe_dir = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                let config_file = exe_dir.join("config").join("temp_config.bat");
                
                // 写入 batch 格式的配置文件
                let config_content = format!(
                    r#"@echo off
set BASE_URL={}
set API_KEY={}
set MODEL_NAME={}
set API_PROTOCOL={}
set WEBCHAT_PORT={}
set WORKSPACE_PATH={}
"#,
                    oc.base_url,
                    oc.api_key,
                    oc.model_name,
                    oc.api_protocol,
                    oc.webchat_port,
                    oc.workspace_path,
                );
                
                if let Err(e) = std::fs::write(&config_file, config_content) {
                    warn!("Failed to write config file: {}", e);
                } else {
                    info!("Config written to: {}", config_file.display());
                    
                    // 调用 set_config.bat 应用配置
                    let set_config_bat = exe_dir.join("scripts").join("set_config.bat");
                    if set_config_bat.exists() {
                        info!("Calling set_config.bat...");
                        let _ = std::process::Command::new("cmd")
                            .args(["/c", &set_config_bat.to_string_lossy()])
                            .creation_flags(0x0800_0000)
                            .spawn();
                    }
                }
                
                config.first_run = false;
                config.gateway_port = oc.webchat_port;
                config.save().unwrap_or_else(|e| warn!("Failed to save config: {}", e));
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    // ── 7. 创建托盘图标（简化版：无状态更新）───────────────────────────────
    let autostart_enabled = autostart::is_autostart_enabled();
    let mut tray = TrayManager::new(autostart_enabled)
        .context("创建托盘图标失败")?;

    let gateway_url = config.gateway_url();
    let gateway_url_for_open = gateway_url.clone();

    // ── 9. 主事件循环（处理托盘菜单点击 + Windows 消息泵）──────────────────
    // tray-icon 在 Windows 上依赖消息泵，必须在主线程跑 GetMessage 循环
    // 否则右键菜单无法弹出/响应
    info!("Entering main event loop");

    let menu_channel = MenuEvent::receiver();

    loop {
        // 抽干 Windows 消息队列（让托盘右键菜单能弹出）
        #[cfg(windows)]
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{
                PeekMessageW, TranslateMessage, DispatchMessageW, PM_REMOVE, MSG,
            };
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // 轮询菜单事件（非阻塞）
        if let Ok(event) = menu_channel.try_recv() {
            match handle_menu_event(event) {
                TrayAction::Open => {
                    info!("User action: open");
                    open_browser(&gateway_url_for_open);
                }

                TrayAction::Settings => {
                    info!("User action: settings");
                    onboarding_server.open();
                }

                TrayAction::Restart => {
                    info!("User action: restart");
                    // 弹出消息框，告诉用户手动执行脚本
                    let exe_dir = std::env::current_exe()
                        .ok()
                        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                        .unwrap_or_else(|| std::path::PathBuf::from("."));
                    let restart_bat = exe_dir.join("scripts").join("restart.bat");
                    let msg = format!(
                        "请手动执行以下脚本来重启服务：\n\n{}\n\n执行完成后此窗口会自动关闭",
                        restart_bat.display()
                    );
                    info!("Showing restart instructions: {}", msg);
                    std::thread::spawn(move || {
                        use windows::core::PCWSTR;
                        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK, MB_ICONINFORMATION};
                        let title: Vec<u16> = "重启服务".encode_utf16().chain(std::iter::once(0)).collect();
                        let text: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
                        unsafe {
                            MessageBoxW(None, PCWSTR(text.as_ptr()), PCWSTR(title.as_ptr()), MB_OK | MB_ICONINFORMATION);
                        }
                    });
                }

                TrayAction::ToggleAutostart => {
                    let currently_enabled = autostart::is_autostart_enabled();
                    if currently_enabled {
                        info!("User disabled autostart");
                        autostart::disable_autostart()
                            .unwrap_or_else(|e| warn!("Failed to disable autostart: {}", e));
                        tray.update_autostart_item(false);
                    } else {
                        info!("User enabled autostart");
                        autostart::enable_autostart()
                            .unwrap_or_else(|e| warn!("Failed to enable autostart: {}", e));
                        tray.update_autostart_item(true);
                    }
                }

                TrayAction::ExportLogs => {
                    info!("User action: export logs");
                    let distro = config.wsl_distro.clone();
                    let log_dir_path = log_dir();
                    std::thread::spawn(move || {
                        export_logs_to_desktop(&distro, &log_dir_path);
                    });
                }

                TrayAction::Quit => {
                    info!("User action: quit");
                    // 简化版：退出时不停止 Gateway，由 systemd 管理
                    break;
                }

                TrayAction::None => {}
            }
        }

        // 轮询配置页提交（用户在"设置"页保存配置后触发）
        if let Some(oc) = onboarding_server.try_recv() {
            info!("New config submitted: base_url={}, model={}, port={}, protocol={}",
                oc.base_url, oc.model_name, oc.webchat_port, oc.api_protocol);
            
            // 只写入临时配置文件，不执行 wsl 命令
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            let config_file = exe_dir.join("config").join("temp_config.bat");
            
            // 写入 batch 格式的配置文件
            let config_content = format!(
                r#"@echo off
set BASE_URL={}
set API_KEY={}
set MODEL_NAME={}
set API_PROTOCOL={}
set WEBCHAT_PORT={}
set WORKSPACE_PATH={}
"#,
                oc.base_url,
                oc.api_key,
                oc.model_name,
                oc.api_protocol,
                oc.webchat_port,
                oc.workspace_path,
            );
            
            if let Err(e) = std::fs::write(&config_file, config_content) {
                warn!("Failed to write config file: {}", e);
            } else {
                info!("Config written to: {}", config_file.display());
                
                // 调用 set_config.bat 应用配置
                let set_config_bat = exe_dir.join("scripts").join("set_config.bat");
                if set_config_bat.exists() {
                    info!("Calling set_config.bat...");
                    let _ = std::process::Command::new("cmd")
                        .args(["/c", &set_config_bat.to_string_lossy()])
                        .creation_flags(0x0800_0000)
                        .spawn();
                }
            }
        }

        // 简化版：不更新托盘状态图标

        // 避免 CPU 空转
        std::thread::sleep(Duration::from_millis(100));
    }

    info!("Ewan OpenClaw Launcher exiting");
    Ok(())
}

/// 用系统默认浏览器打开指定 URL
fn open_browser(url: &str) {
    info!("Opening browser: {}", url);

    #[cfg(windows)]
    {
        
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", url])
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .spawn();
    }

    #[cfg(not(windows))]
    {
        // 开发调试用（Linux/macOS）
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

/// 导出所有日志到桌面的 ewan-openclaw-logs 文件夹
///
/// 收集：
///   1. Launcher 日志：{app}\logs\launcher.log
///   2. Gateway 日志：{app}\logs\gateway\（WSL 里日志可能映射过来）
///   3. 安装日志：%USERPROFILE%\Desktop\ewan-openclaw-install.log
fn export_logs_to_desktop(distro: &str, launcher_log_dir: &std::path::Path) {
    // 目标目录：桌面\ewan-openclaw-logs\
    let desktop = std::env::var("USERPROFILE")
        .map(|p| std::path::PathBuf::from(p).join("Desktop"))
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let dest = desktop.join("ewan-openclaw-logs");
    if let Err(e) = std::fs::create_dir_all(&dest) {
        warn!("Failed to create log export dir: {}", e);
        return;
    }

    info!("Exporting logs to: {}", dest.display());

    // 1. 复制 Launcher 日志
    let launcher_log = launcher_log_dir.join("launcher.log");
    if launcher_log.exists() {
        let dst = dest.join("launcher.log");
        match std::fs::copy(&launcher_log, &dst) {
            Ok(_) => info!("Copied launcher log: {}", dst.display()),
            Err(e) => warn!("Failed to copy launcher log: {}", e),
        }
    }

    // 2. 复制安装日志（桌面根目录）
    let install_log = desktop.join("ewan-openclaw-install.log");
    if install_log.exists() {
        let dst = dest.join("install.log");
        match std::fs::copy(&install_log, &dst) {
            Ok(_) => info!("Copied install log: {}", dst.display()),
            Err(e) => warn!("Failed to copy install log: {}", e),
        }
    }

    // 3. 复制 Gateway 日志和所有其他日志（从 logs 目录）
    if launcher_log_dir.exists() {
        for entry in std::fs::read_dir(launcher_log_dir).into_iter().flatten().flatten() {
            let src = entry.path();
            // 跳过 launcher.log（已经单独复制）
            if src.file_name().map(|n| n == "launcher.log").unwrap_or(false) {
                continue;
            }
            // 复制所有 .log 文件和 gateway 子目录
            if src.is_file() && src.extension().and_then(|e| e.to_str()) == Some("log") {
                if let Some(fname) = src.file_name() {
                    let dst = dest.join(fname);
                    match std::fs::copy(&src, &dst) {
                        Ok(_) => info!("Copied log: {}", dst.display()),
                        Err(e) => warn!("Failed to copy log {:?}: {}", src, e),
                    }
                }
            } else if src.is_dir() {
                // 复制整个子目录（如 gateway）
                let subdir_name = src.file_name().unwrap_or_default();
                let subdir_dest = dest.join(subdir_name);
                if let Err(e) = copy_dir_all(&src, &subdir_dest) {
                    warn!("Failed to copy dir {:?}: {}", src, e);
                } else {
                    info!("Copied dir: {}", subdir_dest.display());
                }
            }
        }
    }

    // 4. 弹窗告知用户
    info!("Log export complete: {}", dest.display());
    let msg = format!(
        "日志已导出到桌面的 ewan-openclaw-logs 文件夹：\n\n{}\n\n包含：\n  · launcher.log（启动器日志）\n  · 其他日志文件\n  · gateway\\（AI 助手服务日志）\n  · install.log（安装日志）",
        dest.display()
    );

    #[cfg(windows)]
    unsafe {
        use windows::core::PCWSTR;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK, MB_ICONINFORMATION};
        let title: Vec<u16> = "Ewan OpenClaw 日志导出".encode_utf16().chain(std::iter::once(0)).collect();
        let text:  Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
        MessageBoxW(None, PCWSTR(text.as_ptr()), PCWSTR(title.as_ptr()), MB_OK | MB_ICONINFORMATION);
    }
}

/// 递归复制目录
fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}
