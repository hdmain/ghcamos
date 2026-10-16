use crate::error::{AppError, Result};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    #[allow(dead_code)]
    pub bg: &'static str,
    #[allow(dead_code)]
    pub border: &'static str,
    pub title: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
    pub accent: &'static str,
    pub card: &'static str,
    pub bar_track: &'static str,
}

pub const GITHUB_DARK: Theme = Theme {
    name: "github_dark",
    bg: "#0d1117",
    border: "#30363d",
    title: "#e6edf3",
    text: "#c9d1d9",
    muted: "#8b949e",
    accent: "#58a6ff",
    card: "#161b22",
    bar_track: "#21262d",
};

pub const LIGHT: Theme = Theme {
    name: "light",
    bg: "#ffffff",
    border: "#d0d7de",
    title: "#1f2328",
    text: "#1f2328",
    muted: "#656d76",
    accent: "#0969da",
    card: "#f6f8fa",
    bar_track: "#eaeef2",
};

pub const NORD: Theme = Theme {
    name: "nord",
    bg: "#2e3440",
    border: "#4c566a",
    title: "#eceff4",
    text: "#d8dee9",
    muted: "#88c0d0",
    accent: "#88c0d0",
    card: "#3b4252",
    bar_track: "#434c5e",
};

pub const ALL_THEMES: &[Theme] = &[GITHUB_DARK, LIGHT, NORD];

pub fn parse_theme(name: &str) -> Result<Theme> {
    ALL_THEMES
        .iter()
        .find(|t| t.name.eq_ignore_ascii_case(name))
        .copied()
        .ok_or_else(|| AppError::UnknownTheme(name.to_string()))
}
