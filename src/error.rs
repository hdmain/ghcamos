use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("missing GitHub token; set GH_TOKEN or GITHUB_TOKEN")]
    MissingToken,

    #[error("GitHub API error: {0}")]
    Api(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("unknown theme `{0}`; expected one of: github_dark, light, nord")]
    UnknownTheme(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
