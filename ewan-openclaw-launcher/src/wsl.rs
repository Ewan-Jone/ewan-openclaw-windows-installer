/// wsl.rs
/// WSL2 control module
///
/// Handles interaction with WSL2: check distro, start/stop OpenClaw Gateway.
/// All child processes use CREATE_NO_WINDOW to hide console windows.

use anyhow::{bail, Context, Result};
use std::process::Command;
use tracing::{error, info, warn};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Windows API constant: do not show console window when creating process
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Add hidden window flag to Command (Windows only)
fn hide_window(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Check if the specified WSL2 distro is installed
pub fn is_distro_installed(distro: &str) -> bool {
    info!("Checking if distro '{}' is installed", distro);
    let output = hide_window(&mut Command::new("wsl"))
        .args(["-l", "-q"])
        .output();

    match output {
        Ok(out) => {
            // wsl -l -q outputs UTF-16 LE on Windows
            let raw = out.stdout;
            let text = decode_wsl_list_output(&raw);
            let found = text.lines().any(|line| {
                line.trim().eq_ignore_ascii_case(distro)
            });
            if found {
                info!("WSL distro '{}' is installed", distro);
            } else {
                warn!("WSL distro '{}' not found, installed list:\n{}", distro, text);
            }
            found
        }
        Err(e) => {
            error!("Failed to run wsl -l -q: {}", e);
            false
        }
    }
}

/// Check if the OpenClaw Gateway is already running
pub fn is_gateway_running(distro: &str) -> bool {
    let output = hide_window(&mut Command::new("wsl"))
        .args([
            "-d", distro, "--",
            "bash", "-lc",
            "pgrep -f 'openclaw gateway' > /dev/null 2>&1 && echo running || echo stopped"
        ])
        .output();

    match output {
        Ok(out) => {
            // wsl -d distro -- bash ... outputs UTF-8
            let text = decode_wsl_exec_output(&out.stdout);
            let running = text.trim() == "running";
            info!("Gateway status: {}", text.trim());
            running
        }
        Err(e) => {
            warn!("Failed to check gateway status: {}", e);
            false
        }
    }
}

/// Wait for gateway to stop, with timeout.
/// Returns true if gateway stopped, false if timeout.
pub fn wait_gateway_stopped(distro: &str, timeout_secs: u64) -> bool {
    let start = std::time::Instant::now();
    loop {
        if !is_gateway_running(distro) {
            info!("Gateway stopped");
            return true;
        }
        if start.elapsed().as_secs() >= timeout_secs {
            warn!("Timeout waiting for gateway to stop after {}s", timeout_secs);
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

/// Start the OpenClaw Gateway
///
/// Skips if already running.
/// Gateway log is written to {app_dir}\logs\gateway.log via drvfs mount.
pub fn start_gateway(distro: &str, port: u16) -> Result<()> {
    info!("Starting OpenClaw Gateway (distro: {}, port: {})...", distro, port);

    if is_gateway_running(distro) {
        info!("Gateway already running, skipping");
        return Ok(());
    }

    // Convert exe directory (Windows path) to WSL drvfs path for the log file
    // e.g. D:\OpenClaw -> /mnt/d/OpenClaw/logs/gateway.log
    let log_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .and_then(|win_dir| {
            let s = win_dir.to_string_lossy().replace('\\', "/");
            if s.len() >= 3 && s.chars().nth(1) == Some(':') {
                let drive = s.chars().next().unwrap().to_lowercase().to_string();
                let rest = &s[3..];
                Some(format!("/mnt/{}/{}/logs/gateway.log", drive, rest))
            } else {
                None
            }
        })
        .unwrap_or_else(|| "/tmp/openclaw-gateway.log".to_string());

    info!("Gateway log path (WSL): {}", log_path);

    // Use sentinel to capture inner exit code, because WSL may return non-zero
    // even when the inner command succeeds (e.g., systemd issues).
    let script = format!(
        "mkdir -p \"$(dirname '{log}')\" && nohup openclaw gateway --port {port} --force > '{log}' 2>&1 & echo __OPENCLAW_EXIT__:$?",
        log = log_path,
        port = port,
    );

    let output = hide_window(&mut Command::new("wsl"))
        .args(["-d", distro, "--", "bash", "-lc", &script])
        .output()
        .context("Failed to execute wsl command")?;

    // Parse inner exit code from sentinel
    let stdout = String::from_utf8_lossy(&output.stdout);
    let inner_exit = stdout
        .lines()
        .find_map(|l| l.strip_prefix("__OPENCLAW_EXIT__:"))
        .and_then(|v| v.trim().parse::<i32>().ok())
        .unwrap_or(-1);

    if inner_exit == 0 {
        info!("Gateway start command succeeded");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Gateway start failed, inner exit: {}, stdout: {}, stderr: {}", inner_exit, stdout.trim(), stderr.trim())
    }
}

/// Stop the OpenClaw Gateway
pub fn stop_gateway(distro: &str) -> Result<()> {
    info!("Stopping OpenClaw Gateway (distro: {})...", distro);

    let status = hide_window(&mut Command::new("wsl"))
        .args(["-d", distro, "--", "bash", "-lc", "pkill -SIGTERM -f 'openclaw gateway' 2>/dev/null; true"])
        .status()
        .context("Failed to execute wsl command")?;

    if status.success() {
        info!("Gateway stopped");
        Ok(())
    } else {
        warn!("Gateway stop command returned non-zero: {:?}", status.code());
        Ok(())
    }
}

/// Decode output from `wsl -l -q` / `wsl --list` (UTF-16 LE on Windows)
///
/// Do NOT use this for `wsl -d distro -- bash ...` output, which is UTF-8.
fn decode_wsl_list_output(raw: &[u8]) -> String {
    if raw.len() >= 2 && raw.len() % 2 == 0 {
        let utf16: Vec<u16> = raw
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        if let Ok(s) = String::from_utf16(&utf16) {
            return s.replace('\0', "").trim().to_string();
        }
    }
    String::from_utf8_lossy(raw).replace('\0', "").trim().to_string()
}

/// Decode output from `wsl -d <distro> -- bash ...` (UTF-8)
fn decode_wsl_exec_output(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw).replace('\0', "").trim().to_string()
}

/// Read a file inside a WSL distro
pub fn read_file_in_wsl(distro: &str, path: &str) -> Option<String> {
    let output = std::process::Command::new("wsl")
        .args(["-d", distro, "--", "bash", "-lc", &format!("cat {}", path)])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if output.status.success() {
        Some(decode_wsl_exec_output(&output.stdout))
    } else {
        None
    }
}

/// Find the absolute path of the openclaw binary inside a WSL distro
pub fn find_openclaw_bin(distro: &str) -> Option<String> {
    // Try bash -lc which (loads login init files to get correct PATH)
    let output = std::process::Command::new("wsl")
        .args(["-d", distro, "--", "bash", "-lc", "which openclaw 2>/dev/null"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if output.status.success() {
        let path = decode_wsl_exec_output(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }
    // Fallback: search common install locations
    let fallback = std::process::Command::new("wsl")
        .args([
            "-d", distro, "--", "bash", "-c",
            "find /root/.nvm/versions /usr/local/bin /usr/bin -name openclaw 2>/dev/null | head -1"
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    let path = decode_wsl_exec_output(&fallback.stdout).trim().to_string();
    if !path.is_empty() { Some(path) } else { None }
}
