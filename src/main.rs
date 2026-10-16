mod error;
mod github;
mod svg;
mod theme;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::error::{AppError, Result};
use crate::github::{fetch_lifetime_contributions, fetch_user_stats};
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
            token,
        } => {
            let token = resolve_token(token)?;
            let theme = parse_theme(&theme)?;
            let mut stats = fetch_user_stats(&token, &user).await?;
            let (commits, prs, issues, reviews) =
                fetch_lifetime_contributions(&token, &user, &stats.created_at).await?;
            stats.commits = commits;
            stats.pull_requests = prs;
            stats.issues = issues;
            stats.reviews = reviews;

            let svg = render_stats_card(&stats, &theme, width);
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
