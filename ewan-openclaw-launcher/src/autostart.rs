/// autostart.rs
/// Windows autostart management via registry
///
/// Registry path: HKCU\Software\Microsoft\Windows\CurrentVersion\Run

use anyhow::{Context, Result};
use tracing::{info, warn};

#[cfg(windows)]
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

const REG_KEY_NAME: &str = "OpenClawLauncher";
const REG_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Enable autostart: write exe path to registry Run key
pub fn enable_autostart() -> Result<()> {
    let exe_path = std::env::current_exe()
        .context("Failed to get current exe path")?;
    let exe_str = exe_path.to_string_lossy().to_string();

    #[cfg(windows)]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey(REG_PATH)
            .context("Failed to open registry Run key")?;
        key.set_value(REG_KEY_NAME, &exe_str)
            .context("Failed to write registry value")?;
        info!("Autostart enabled: {}", exe_str);
    }

    #[cfg(not(windows))]
    {
        warn!("Non-Windows platform, skipping registry (path: {})", exe_str);
    }

    Ok(())
}

/// Disable autostart: remove exe entry from registry Run key
pub fn disable_autostart() -> Result<()> {
    #[cfg(windows)]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        match hkcu.open_subkey_with_flags(REG_PATH, winreg::enums::KEY_WRITE) {
            Ok(key) => {
                match key.delete_value(REG_KEY_NAME) {
                    Ok(_) => info!("Autostart disabled"),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        warn!("Autostart key not found, nothing to remove");
                    }
                    Err(e) => return Err(e).context("Failed to delete registry value")?,
                }
            }
            Err(e) => {
                warn!("Failed to open registry key (may not exist): {}", e);
            }
        }
    }

    #[cfg(not(windows))]
    {
        warn!("Non-Windows platform, skipping registry");
    }

    Ok(())
}

/// Check if autostart is currently enabled
pub fn is_autostart_enabled() -> bool {
    #[cfg(windows)]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        match hkcu.open_subkey(REG_PATH) {
            Ok(key) => {
                let val: Result<String, _> = key.get_value(REG_KEY_NAME);
                val.is_ok()
            }
            Err(_) => false,
        }
    }

    #[cfg(not(windows))]
    {
        false
    }
}
