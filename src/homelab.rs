//! Homelab host metrics from a public ghcamos-host endpoint + rolling history.

use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

pub const DEFAULT_URL: &str = "https://homelabstats.alfahost.eu/stats";
pub const DEFAULT_HISTORY_URL: &str = "https://hdmain.github.io/ghcamos/homelab-history.json";
pub const HISTORY_LEN: usize = 4;
pub const HOT_THRESHOLD_PCT: f32 = 90.0;

#[derive(Debug, Clone, Deserialize)]
pub struct HomelabStats {
    pub ok: bool,
    pub label: String,
    pub uptime_secs: u64,
    pub loadavg: [f64; 3],
    pub cpu: CpuStats,
    pub memory: MemStats,
    pub swap: SwapStats,
    #[serde(default)]
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
    pub total_bytes: u64,
    pub used_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DiskStats {
    #[allow(dead_code)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomelabSample {
    pub ts: String,
    pub cpu_pct: f32,
    pub mem_used_bytes: u64,
    pub mem_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
    pub load1: f64,
    pub cores: usize,
    pub temp_c: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HomelabHistory {
    pub samples: Vec<HomelabSample>,
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
        pct(self.memory.used_bytes, self.memory.total_bytes)
    }

    pub fn swap_used_pct(&self) -> f32 {
        pct(self.swap.used_bytes, self.swap.total_bytes)
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

    pub fn to_sample(&self) -> HomelabSample {
        HomelabSample {
            ts: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            cpu_pct: self.cpu.usage_pct,
            mem_used_bytes: self.memory.used_bytes,
            mem_total_bytes: self.memory.total_bytes,
            swap_used_bytes: self.swap.used_bytes,
            swap_total_bytes: self.swap.total_bytes,
            load1: self.loadavg[0],
            cores: self.cpu.cores,
            temp_c: self.package_temp_c(),
        }
    }

    /// True when any pressure metric is at/above 90%.
    pub fn is_hot(&self) -> bool {
        self.cpu.usage_pct >= HOT_THRESHOLD_PCT
            || self.mem_used_pct() >= HOT_THRESHOLD_PCT
            || self.swap_used_pct() >= HOT_THRESHOLD_PCT
            || self
                .package_temp_c()
                .map(|t| t >= HOT_THRESHOLD_PCT)
                .unwrap_or(false)
            || (self.cpu.cores > 0
                && (self.loadavg[0] as f32) / self.cpu.cores as f32 * 100.0 >= HOT_THRESHOLD_PCT)
    }
}

impl HomelabSample {
    pub fn mem_pct(&self) -> f32 {
        pct(self.mem_used_bytes, self.mem_total_bytes)
    }
}

impl HomelabHistory {
    pub fn push_sample(&mut self, sample: HomelabSample) {
        // Avoid duplicate if re-run within the same minute.
        if let Some(last) = self.samples.last() {
            if last.ts.get(..16) == sample.ts.get(..16) {
                *self.samples.last_mut().unwrap() = sample;
                return;
            }
        }
        self.samples.push(sample);
        if self.samples.len() > HISTORY_LEN {
            let skip = self.samples.len() - HISTORY_LEN;
            self.samples.drain(0..skip);
        }
    }
}

fn pct(used: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        (used as f64 / total as f64 * 100.0) as f32
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
    let client = client()?;
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(AppError::Api(format!("homelab {status}: {text}")));
    }

    let stats: HomelabStats = response
        .json()
        .await
        .map_err(|e| AppError::Api(format!("homelab decode failed: {e}")))?;

    if !stats.ok {
        return Err(AppError::Api("homelab ok=false".into()));
    }
    Ok(stats)
}

pub async fn fetch_homelab_history(url: &str) -> HomelabHistory {
    let Ok(client) = client() else {
        return HomelabHistory::default();
    };
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => resp
            .json::<HomelabHistory>()
            .await
            .unwrap_or_default(),
        _ => HomelabHistory::default(),
    }
}

fn client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent("ghcamos/0.1 (+https://github.com/hdmain/ghcamos)")
        .timeout(std::time::Duration::from_secs(12))
        .build()?)
}
