/// health.rs
/// Gateway health monitor
///
/// Periodically probes the Gateway via HTTP.
/// Triggers automatic restart after consecutive failures.

use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info, warn};

#[derive(Debug, Clone, PartialEq)]
pub enum GatewayStatus {
    Starting,
    Running,
    Stopped,
}

impl GatewayStatus {
    pub fn tooltip(&self) -> &'static str {
        match self {
            GatewayStatus::Starting => "AI Assistant - Starting...",
            GatewayStatus::Running  => "AI Assistant - Running",
            GatewayStatus::Stopped  => "AI Assistant - Stopped",
        }
    }
}

pub struct HealthChecker {
    gateway_url: String,
    pub status: Arc<Mutex<GatewayStatus>>,
    fail_count: u32,
    fail_threshold: u32,
}

impl HealthChecker {
    pub fn new(gateway_url: String) -> Self {
        Self {
            gateway_url,
            status: Arc::new(Mutex::new(GatewayStatus::Starting)),
            fail_count: 0,
            fail_threshold: 3,
        }
    }

    /// Run one health check; returns current status
    pub async fn check(&mut self, distro: &str) -> GatewayStatus {
        match self.probe().await {
            Ok(true) => {
                if self.fail_count > 0 {
                    info!("Gateway recovered");
                }
                self.fail_count = 0;
                let s = GatewayStatus::Running;
                self.update_status(s.clone());
                s
            }
            Ok(false) | Err(_) => {
                self.fail_count += 1;
                warn!("Gateway health check failed ({}/{})", self.fail_count, self.fail_threshold);

                if self.fail_count >= self.fail_threshold {
                    error!("Gateway unresponsive for {} checks, restarting...", self.fail_threshold);
                    self.restart(distro);
                    self.fail_count = 0;
                }

                let s = GatewayStatus::Stopped;
                self.update_status(s.clone());
                s
            }
        }
    }

    async fn probe(&self) -> Result<bool> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        match client.get(&self.gateway_url).send().await {
            Ok(resp) => {
                let ok = resp.status().is_success() || resp.status().as_u16() == 401;
                Ok(ok)
            }
            Err(e) => {
                warn!("HTTP probe failed: {}", e);
                Ok(false)
            }
        }
    }

    fn restart(&self, distro: &str) {
        info!("Executing gateway restart...");
        let _ = crate::wsl::stop_gateway(distro);
        // 等待 gateway 真正停止，而不是硬编码 sleep
        if !crate::wsl::wait_gateway_stopped(distro, 10) {
            warn!("Gateway did not stop, proceeding anyway");
        }
        if let Err(e) = crate::wsl::start_gateway(distro, 17789) {
            error!("Gateway restart failed: {}", e);
        }
    }

    fn update_status(&self, new_status: GatewayStatus) {
        if let Ok(mut s) = self.status.lock() {
            *s = new_status;
        }
    }

    /// Wait until gateway is ready, up to timeout_secs. Returns true if ready.
    pub async fn wait_until_ready(&self, timeout_secs: u64) -> bool {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .expect("Failed to build HTTP client");

        let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);

        while tokio::time::Instant::now() < deadline {
            match client.get(&self.gateway_url).send().await {
                Ok(resp) if resp.status().as_u16() < 500 => {
                    info!("Gateway is ready");
                    return true;
                }
                _ => {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }

        error!("Timed out waiting for gateway to be ready ({}s)", timeout_secs);
        false
    }
}
