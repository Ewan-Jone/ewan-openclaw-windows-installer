/// config.rs
/// Launcher configuration management
///
/// Reads/writes {exe_dir}\launcher.json

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub autostart: bool,
    /// WSL2 distro name (default: "ewan-openclaw")
    pub wsl_distro: String,
    /// Gateway listen port (default: 17789)
    pub gateway_port: u16,
    /// Whether this is the first run (triggers onboarding)
    pub first_run: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            autostart: true,
            wsl_distro: "ewan-openclaw".to_string(),
            gateway_port: 17789,
            first_run: true,
        }
    }
}

impl Config {
    /// Config file path: {exe_dir}\launcher.json
    pub fn config_path() -> Result<PathBuf> {
        let exe_dir = std::env::current_exe()
            .context("Failed to get exe path")?
            .parent()
            .context("Failed to get exe directory")?
            .to_path_buf();
        Ok(exe_dir.join("launcher.json"))
    }

    /// Load config from disk; returns default if file does not exist
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            debug!("Config file not found, using defaults: {:?}", path);
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config: {:?}", path))?;

        let mut config: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse config: {:?}", path))?;

        // Migration: old default port 18789 -> 17789
        if config.gateway_port == 18789 {
            info!("Migrating old default port 18789 -> 17789");
            config.gateway_port = 17789;
            let _ = config.save();
        }

        debug!("Config loaded: {:?}", path);
        Ok(config)
    }

    /// Save config to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config dir: {:?}", parent))?;
        }

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize config")?;

        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write config: {:?}", path))?;

        info!("Config saved: {:?}", path);
        Ok(())
    }

    /// Returns the Gateway URL
    pub fn gateway_url(&self) -> String {
        format!("http://localhost:{}", self.gateway_port)
    }
}
