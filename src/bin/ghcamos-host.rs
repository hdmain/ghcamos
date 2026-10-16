//! Public host metrics HTTP daemon for Linux (no private identifiers).
//!
//! ```bash
//! cargo run --release --bin ghcamos-host
//! # listens on 0.0.0.0:7428
//! curl http://127.0.0.1:7428/stats
//! ```

use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use clap::Parser;
use serde::Serialize;
use sysinfo::{Components, Disks, Networks, System};
use tower_http::cors::{Any, CorsLayer};

#[derive(Debug, Parser)]
#[command(
    name = "ghcamos-host",
    about = "Expose non-private host stats over HTTP for ghcamos cards"
)]
struct Cli {
    /// Bind address (default public LAN listener)
    #[arg(long, env = "GHCAMOS_HOST_LISTEN", default_value = "0.0.0.0:7428")]
    listen: SocketAddr,

    /// Public display label (never use real hostname / username)
    #[arg(long, env = "GHCAMOS_HOST_LABEL", default_value = "host")]
    label: String,
}

#[derive(Clone)]
struct AppState {
    label: String,
}

#[derive(Debug, Serialize)]
struct StatsResponse {
    ok: bool,
    label: String,
    uptime_secs: u64,
    loadavg: [f64; 3],
    cpu: CpuStats,
    memory: MemStats,
    swap: SwapStats,
    disk: DiskStats,
    temps_c: Vec<TempReading>,
}

#[derive(Debug, Serialize)]
struct CpuStats {
    usage_pct: f32,
    cores: usize,
}

#[derive(Debug, Serialize)]
struct MemStats {
    total_bytes: u64,
    used_bytes: u64,
    available_bytes: u64,
}

#[derive(Debug, Serialize)]
struct SwapStats {
    total_bytes: u64,
    used_bytes: u64,
}

#[derive(Debug, Serialize)]
struct DiskStats {
    /// Aggregate used % across local disks (no paths / mountpoints).
    used_pct: f32,
    total_bytes: u64,
    available_bytes: u64,
}

#[derive(Debug, Serialize)]
struct TempReading {
    /// Sanitized sensor name (no serials / board IDs).
    name: String,
    celsius: f32,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let state = AppState { label: sanitize_label(&cli.label) };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(stats_handler))
        .route("/stats", get(stats_handler))
        .route("/health", get(|| async { StatusCode::OK }))
        .layer(cors)
        .with_state(state);

    eprintln!("ghcamos-host listening on http://{}", cli.listen);
    eprintln!("endpoints: /stats  /health");

    let listener = tokio::net::TcpListener::bind(cli.listen)
        .await
        .unwrap_or_else(|e| {
            eprintln!("bind {}: {e}", cli.listen);
            std::process::exit(1);
        });

    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| {
            eprintln!("server error: {e}");
            std::process::exit(1);
        });
}

async fn stats_handler(State(state): State<AppState>) -> impl IntoResponse {
    match tokio::task::spawn_blocking(collect_stats).await {
        Ok(mut stats) => {
            stats.label = state.label;
            Json(stats).into_response()
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("collect failed: {err}"),
        )
            .into_response(),
    }
}

fn collect_stats() -> StatsResponse {
    let mut sys = System::new();
    sys.refresh_cpu_all();
    // Second sample so usage_pct is meaningful.
    std::thread::sleep(Duration::from_millis(200));
    sys.refresh_cpu_all();
    sys.refresh_memory();

    let loadavg = System::load_average();
    let cores = sys.cpus().len();
    let usage_pct = sys.global_cpu_usage();

    let disks = Disks::new_with_refreshed_list();
    let (disk_total, disk_avail) = disks.list().iter().fold((0u64, 0u64), |acc, d| {
        // Skip pseudo / remote-ish filesystems by keeping only disks with a non-zero total.
        let total = d.total_space();
        if total == 0 {
            return acc;
        }
        (acc.0.saturating_add(total), acc.1.saturating_add(d.available_space()))
    });
    let disk_used_pct = if disk_total == 0 {
        0.0
    } else {
        let used = disk_total.saturating_sub(disk_avail) as f64;
        ((used / disk_total as f64) * 100.0) as f32
    };

    let components = Components::new_with_refreshed_list();
    let mut temps: Vec<TempReading> = components
        .iter()
        .filter_map(|c| {
            let temp = c.temperature()?;
            if !temp.is_finite() || temp <= 0.0 || temp > 150.0 {
                return None;
            }
            Some(TempReading {
                name: sanitize_sensor_name(c.label()),
                celsius: (temp * 10.0).round() / 10.0,
            })
        })
        .collect();
    temps.sort_by(|a, b| a.name.cmp(&b.name));
    temps.dedup_by(|a, b| a.name == b.name);

    // Touch Networks so sysinfo initializes it on some platforms — we intentionally
    // do NOT expose interface names, MACs, or addresses.
    let _ = Networks::new_with_refreshed_list();

    StatsResponse {
        ok: true,
        label: "host".into(),
        uptime_secs: System::uptime(),
        loadavg: [loadavg.one, loadavg.five, loadavg.fifteen],
        cpu: CpuStats {
            usage_pct: (usage_pct * 10.0).round() / 10.0,
            cores,
        },
        memory: MemStats {
            total_bytes: sys.total_memory(),
            used_bytes: sys.used_memory(),
            available_bytes: sys.available_memory(),
        },
        swap: SwapStats {
            total_bytes: sys.total_swap(),
            used_bytes: sys.used_swap(),
        },
        disk: DiskStats {
            used_pct: (disk_used_pct * 10.0).round() / 10.0,
            total_bytes: disk_total,
            available_bytes: disk_avail,
        },
        temps_c: temps,
    }
}

fn sanitize_label(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(32)
        .collect();
    if cleaned.is_empty() {
        "host".into()
    } else {
        cleaned
    }
}

fn sanitize_sensor_name(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    // Map common Linux/sysinfo labels to short public names.
    if lower.contains("pkg") || lower.contains("package") || lower.contains("tctl") {
        return "cpu_package".into();
    }
    if lower.contains("core") {
        // Collapse per-core temps into a single public bucket name + index if present.
        if let Some(digits) = lower.chars().filter(|c| c.is_ascii_digit()).collect::<String>().chars().next() {
            return format!("cpu_core_{digits}");
        }
        return "cpu_core".into();
    }
    if lower.contains("nvme") || lower.contains("ssd") || lower.contains("hdd") || lower.contains("drivetemp") {
        return "disk".into();
    }
    if lower.contains("gpu") || lower.contains("edge") || lower.contains("junction") {
        return "gpu".into();
    }
    if lower.contains("acpi") || lower.contains("pch") || lower.contains("wifi") || lower.contains("iwlwifi") {
        return "board".into();
    }
    // Generic fallback — strip digits/serial-looking chunks.
    let mut out = String::new();
    for c in raw.chars().take(24) {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            out.push(c.to_ascii_lowercase());
        } else if c.is_whitespace() && !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() {
        "sensor".into()
    } else {
        out
    }
}
