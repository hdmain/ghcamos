//! Homelab host metrics from a public ghcamos-host endpoint.

use reqwest::Client;
use serde::Deserialize;

use crate::error::{AppError, Result};

pub const DEFAULT_URL: &str = "https://homelabstats.alfahost.eu/stats";

#[derive(Debug, Clone, Deserialize)]
pub struct HomelabStats {
    pub ok: bool,
    pub label: String,
    pub uptime_secs: u64,
    pub loadavg: [f64; 3],
    pub cpu: CpuStats,
    pub memory: MemStats,
    pub swap: SwapStats,
    pub disk: DiskStats,
    #[serde(default)]
    pub temps_c: Vec<TempReading>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CpuStats {
    pub usage_pct: f32,
    pub cores: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MemStats {
    pub total_bytes: u64,
    pub used_bytes: u64,
    #[allow(dead_code)]
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SwapStats {
    #[allow(dead_code)]
    pub total_bytes: u64,
    #[allow(dead_code)]
    pub used_bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiskStats {
    pub used_pct: f32,
    #[allow(dead_code)]
    pub total_bytes: u64,
    #[allow(dead_code)]
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TempReading {
    pub name: String,
    pub celsius: f32,
}

impl HomelabStats {
    pub fn package_temp_c(&self) -> Option<f32> {
        self.temps_c
            .iter()
            .find(|t| t.name == "cpu_package")
            .map(|t| t.celsius)
            .or_else(|| {
                self.temps_c
                    .iter()
                    .filter(|t| t.name.starts_with("cpu_"))
                    .map(|t| t.celsius)
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            })
    }

    pub fn mem_used_pct(&self) -> f32 {
        if self.memory.total_bytes == 0 {
            0.0
        } else {
            (self.memory.used_bytes as f64 / self.memory.total_bytes as f64 * 100.0) as f32
        }
    }

    pub fn uptime_human(&self) -> String {
        let d = self.uptime_secs / 86_400;
        let h = (self.uptime_secs % 86_400) / 3600;
        if d > 0 {
            format!("{d}d {h}h")
        } else {
            let m = (self.uptime_secs % 3600) / 60;
            format!("{h}h {m}m")
        }
    }
}

pub fn format_bytes(n: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    const TIB: f64 = GIB * 1024.0;
    let v = n as f64;
    if v >= TIB {
        format!("{:.1} TiB", v / TIB)
    } else if v >= GIB {
        format!("{:.1} GiB", v / GIB)
    } else if v >= MIB {
        format!("{:.0} MiB", v / MIB)
    } else if v >= KIB {
        format!("{:.0} KiB", v / KIB)
    } else {
        format!("{n} B")
    }
}

pub async fn fetch_homelab_stats(url: &str) -> Result<HomelabStats> {
    let client = Client::builder()
        .user_agent("ghcamos/0.1 (+https://github.com/hdmain/ghcamos)")
        .timeout(std::time::Duration::from_secs(12))
        .build()?;

    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(AppError::Api(format!("homelab {status}: {text}")));
    }

    let stats: HomelabStats = response.json().await.map_err(|e| {
        AppError::Api(format!("homelab decode failed: {e}"))
    })?;

    if !stats.ok {
        return Err(AppError::Api("homelab ok=false".into()));
    }
    Ok(stats)
}
