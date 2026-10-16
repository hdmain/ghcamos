mod bg;
mod error;
mod fire;
mod github;
mod homelab;
mod lastfm;
mod retry;
mod svg;
mod theme;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::bg::Period;
use crate::error::{AppError, Result};
use crate::github::{
    fetch_lifetime_contributions, fetch_stats_snapshot, fetch_user_stats, DEFAULT_SNAPSHOT_URL,
};
use crate::homelab::{
    fetch_homelab_history, fetch_homelab_stats, DEFAULT_HISTORY_URL, DEFAULT_URL as HOMELAB_DEFAULT_URL,
};
use crate::lastfm::fetch_recent_track;
use crate::svg::render_stats_card;
use crate::theme::parse_theme;

#[derive(Debug, Parser)]
#[command(name = "ghcamos", about = "Render GitHub stats SVG cards for profile READMEs")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Fetch stats and write a self-contained SVG card
    Render {
        /// GitHub login
        #[arg(long, default_value = "hdmain")]
        user: String,

        /// Theme: github_dark | light | nord
        #[arg(long, default_value = "github_dark")]
        theme: String,

        /// Card width in pixels (360–1600)
        #[arg(long, default_value_t = 680)]
        width: u32,

        /// Output path (defaults to stdout)
        #[arg(long, short)]
        out: Option<PathBuf>,

        /// Background period: auto | morning | day | night (auto = Europe/Warsaw clock)
        #[arg(long, default_value = "auto")]
        period: String,

        /// Show corner pixel-fire sprites (off by default)
        #[arg(long, default_value_t = false)]
        fires: bool,

        /// Last.fm username for recent / now-playing track
        #[arg(long, default_value = "hdmain", env = "LASTFM_USER")]
        lastfm_user: String,

        /// Last.fm API key (optional; falls back to public widget scrape)
        #[arg(long, env = "LASTFM_API_KEY")]
        lastfm_api_key: Option<String>,

        /// Skip Last.fm section
        #[arg(long, default_value_t = false)]
        no_lastfm: bool,

        /// Homelab stats URL (ghcamos-host JSON)
        #[arg(long, default_value = HOMELAB_DEFAULT_URL, env = "HOMELAB_STATS_URL")]
        homelab_url: String,

        /// Rolling homelab history JSON (published on Pages)
        #[arg(long, default_value = DEFAULT_HISTORY_URL, env = "HOMELAB_HISTORY_URL")]
        homelab_history_url: String,

        /// Skip homelab section
        #[arg(long, default_value_t = false)]
        no_homelab: bool,

        /// Previous GitHub stats snapshot JSON (for ↑/↓ arrows)
        #[arg(long, default_value = DEFAULT_SNAPSHOT_URL, env = "GITHUB_STATS_SNAPSHOT_URL")]
        stats_snapshot_url: String,

        /// GitHub token (or set GH_TOKEN / GITHUB_TOKEN)
        #[arg(long, env = "GH_TOKEN")]
        token: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Render {
            user,
            theme,
            width,
            out,
            period,
            fires,
            lastfm_user,
            lastfm_api_key,
            no_lastfm,
            homelab_url,
            homelab_history_url,
            no_homelab,
            stats_snapshot_url,
            token,
        } => {
            let token = resolve_token(token)?;
            let theme = parse_theme(&theme)?;
            let period = Period::parse(&period).ok_or_else(|| {
                AppError::Api(format!(
                    "unknown period `{period}`; expected auto|morning|day|night"
                ))
            })?;
            // GitHub profile is required for the card shell; optional sections hide on failure.
            let mut stats = crate::retry::with_retry(3, "github", || {
                fetch_user_stats(&token, &user)
            })
            .await?;

            match crate::retry::with_retry(3, "github-lifetime", || {
                fetch_lifetime_contributions(&token, &user, &stats.created_at)
            })
            .await
            {
                Ok((commits, prs, issues, reviews)) => {
                    stats.commits = commits;
                    stats.pull_requests = prs;
                    stats.issues = issues;
                    stats.reviews = reviews;
                }
                Err(err) => {
                    eprintln!("github-lifetime omitted (keeping last-year totals): {err}");
                }
            }

            let prev_snapshot = fetch_stats_snapshot(&stats_snapshot_url).await;
            if prev_snapshot.is_some() {
                eprintln!("stats-snapshot: loaded previous totals for trends");
            } else {
                eprintln!("stats-snapshot: none (first run or fetch failed) - flat arrows");
            }
            let trends = stats.trends(prev_snapshot.as_ref());

            // Optional boxes: on error / empty → section is simply not rendered.
            let lastfm = if no_lastfm {
                None
            } else {
                match crate::retry::with_retry(3, "lastfm", || {
                    fetch_recent_track(&lastfm_user, lastfm_api_key.as_deref())
                })
                .await
                {
                    Ok(Some(t)) => {
                        eprintln!("lastfm={} - {} · {}", t.label(), t.artist, t.name);
                        Some(t)
                    }
                    Ok(None) => {
                        eprintln!("lastfm omitted: no recent track");
                        None
                    }
                    Err(err) => {
                        eprintln!("lastfm omitted: {err}");
                        None
                    }
                }
            };

            let mut homelab_history = None;
            let homelab = if no_homelab {
                None
            } else {
                match crate::retry::with_retry(3, "homelab", || {
                    fetch_homelab_stats(&homelab_url)
                })
                .await
                {
                    Ok(h) => {
                        let mut history =
                            fetch_homelab_history(&homelab_history_url).await;
                        history.push_sample(h.to_sample());
                        eprintln!(
                            "homelab={} load={:.2}/{} ram={}/{} samples={}",
                            h.label,
                            h.loadavg[0],
                            h.cpu.cores,
                            crate::homelab::format_bytes(h.memory.used_bytes),
                            crate::homelab::format_bytes(h.memory.total_bytes),
                            history.samples.len()
                        );
                        homelab_history = Some(history);
                        Some(h)
                    }
                    Err(err) => {
                        eprintln!("homelab omitted: {err}");
                        None
                    }
                }
            };

            let show_fires = fires
                || homelab
                    .as_ref()
                    .map(|h| h.is_hot())
                    .unwrap_or(false);
            if show_fires && !fires {
                eprintln!("fires auto-enabled: homelab metric >= 90%");
            }

            eprintln!("period={} fires={show_fires}", period.as_str());
            let svg = render_stats_card(
                &stats,
                &theme,
                width,
                period,
                show_fires,
                lastfm.as_ref(),
                homelab.as_ref(),
                homelab_history.as_ref(),
                trends,
            );
            match out {
                Some(path) => {
                    if let Some(parent) = path.parent() {
                        if !parent.as_os_str().is_empty() {
                            std::fs::create_dir_all(parent)?;
                        }
                    }
                    std::fs::write(&path, svg.as_bytes())?;
                    eprintln!("wrote {}", path.display());
                    let out_dir = path
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new("."));
                    let snap_path = out_dir.join("github-stats.json");
                    let snap_json = serde_json::to_string_pretty(&stats.snapshot())?;
                    std::fs::write(&snap_path, snap_json)?;
                    eprintln!("wrote {}", snap_path.display());
                    if let Some(history) = homelab_history.as_ref() {
                        let hist_path = out_dir.join("homelab-history.json");
                        let json = serde_json::to_string_pretty(history)?;
                        std::fs::write(&hist_path, json)?;
                        eprintln!("wrote {}", hist_path.display());
                    }
                }
                None => print!("{svg}"),
            }
            Ok(())
        }
    }
}

fn resolve_token(cli_token: Option<String>) -> Result<String> {
    if let Some(t) = cli_token.filter(|s| !s.is_empty()) {
        return Ok(t);
    }
    if let Ok(t) = std::env::var("GITHUB_TOKEN") {
        if !t.is_empty() {
            return Ok(t);
        }
    }
    if let Ok(t) = std::env::var("GH_TOKEN") {
        if !t.is_empty() {
            return Ok(t);
        }
    }
    Err(AppError::MissingToken)
}
