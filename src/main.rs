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
use crate::github::{fetch_lifetime_contributions, fetch_user_stats};
use crate::homelab::{fetch_homelab_stats, DEFAULT_URL as HOMELAB_DEFAULT_URL};
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

        /// Skip homelab section
        #[arg(long, default_value_t = false)]
        no_homelab: bool,

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
            no_homelab,
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

            let homelab = if no_homelab {
                None
            } else {
                match crate::retry::with_retry(3, "homelab", || {
                    fetch_homelab_stats(&homelab_url)
                })
                .await
                {
                    Ok(h) => {
                        eprintln!(
                            "homelab={} cpu={:.0}% mem={:.0}% disk={:.0}%",
                            h.label,
                            h.cpu.usage_pct,
                            h.mem_used_pct(),
                            h.disk.used_pct
                        );
                        Some(h)
                    }
                    Err(err) => {
                        eprintln!("homelab omitted: {err}");
                        None
                    }
                }
            };

            eprintln!("period={} fires={fires}", period.as_str());
            let svg = render_stats_card(
                &stats,
                &theme,
                width,
                period,
                fires,
                lastfm.as_ref(),
                homelab.as_ref(),
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
