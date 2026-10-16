use std::collections::HashMap;

use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use crate::error::{AppError, Result};

const GITHUB_GRAPHQL: &str = "https://api.github.com/graphql";

#[derive(Debug, Clone)]
pub struct LanguageStat {
    pub name: String,
    pub color: String,
    pub bytes: u64,
}

#[derive(Debug, Clone)]
pub struct RepoStat {
    pub name: String,
    pub stars: u32,
    pub language: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserStats {
    pub login: String,
    pub name: Option<String>,
    pub created_at: String,
    pub followers: u32,
    #[allow(dead_code)]
    pub following: u32,
    pub total_stars: u32,
    pub commits: u32,
    pub pull_requests: u32,
    pub issues: u32,
    pub reviews: u32,
    pub languages: Vec<LanguageStat>,
    pub top_repos: Vec<RepoStat>,
}

#[derive(Debug, Deserialize)]
struct GqlResponse {
    data: Option<GqlData>,
    errors: Option<Vec<GqlError>>,
}

#[derive(Debug, Deserialize)]
struct GqlError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct GqlData {
    user: Option<GqlUser>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlUser {
    login: Option<String>,
    name: Option<String>,
    created_at: Option<String>,
    followers: Option<Count>,
    following: Option<Count>,
    contributions_collection: Contributions,
    #[serde(default)]
    repositories: Option<RepoConnection>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Count {
    total_count: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Contributions {
    total_commit_contributions: u32,
    total_pull_request_contributions: u32,
    total_issue_contributions: u32,
    total_pull_request_review_contributions: u32,
    restricted_contributions_count: u32,
}

#[derive(Debug, Deserialize)]
struct RepoConnection {
    nodes: Vec<RepoNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepoNode {
    name: String,
    stargazer_count: u32,
    is_fork: bool,
    primary_language: Option<LanguageNode>,
    languages: Option<LanguageConnection>,
}

#[derive(Debug, Deserialize)]
struct LanguageNode {
    name: String,
    color: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LanguageConnection {
    edges: Vec<LanguageEdge>,
}

#[derive(Debug, Deserialize)]
struct LanguageEdge {
    size: u64,
    node: LanguageNode,
}

const QUERY: &str = r#"
query($login: String!) {
  user(login: $login) {
    login
    name
    createdAt
    followers { totalCount }
    following { totalCount }
    contributionsCollection {
      totalCommitContributions
      totalPullRequestContributions
      totalIssueContributions
      totalPullRequestReviewContributions
      restrictedContributionsCount
    }
    repositories(
      first: 100
      ownerAffiliations: OWNER
      isFork: false
      orderBy: { field: STARGAZERS, direction: DESC }
    ) {
      nodes {
        name
        stargazerCount
        isFork
        primaryLanguage { name color }
        languages(first: 10, orderBy: { field: SIZE, direction: DESC }) {
          edges {
            size
            node { name color }
          }
        }
      }
    }
  }
}
"#;

pub async fn fetch_user_stats(token: &str, login: &str) -> Result<UserStats> {
    let client = Client::builder()
        .user_agent("ghcamos/0.1")
        .build()?;

    let body = json!({
        "query": QUERY,
        "variables": { "login": login }
    });

    let response = client
        .post(GITHUB_GRAPHQL)
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(AppError::Api(format!("{status}: {text}")));
    }

    let text = response.text().await?;
    let parsed: GqlResponse = serde_json::from_str(&text).map_err(|e| {
        AppError::Api(format!("decode failed: {e}; body={}", truncate(&text, 400)))
    })?;

    if let Some(errors) = parsed.errors {
        let msg = errors
            .into_iter()
            .map(|e| e.message)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(AppError::Api(msg));
    }

    let user = parsed
        .data
        .and_then(|d| d.user)
        .ok_or_else(|| AppError::Api(format!("user `{login}` not found")))?;

    Ok(aggregate(user))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

fn aggregate(user: GqlUser) -> UserStats {
    let mut lang_bytes: HashMap<String, (u64, String)> = HashMap::new();
    let mut total_stars = 0u32;
    let mut top_repos = Vec::new();

    let repos = user.repositories.as_ref().map(|r| r.nodes.as_slice()).unwrap_or(&[]);
    for repo in repos {
        if repo.is_fork {
            continue;
        }
        total_stars = total_stars.saturating_add(repo.stargazer_count);

        if top_repos.len() < 3 {
            top_repos.push(RepoStat {
                name: repo.name.clone(),
                stars: repo.stargazer_count,
                language: repo.primary_language.as_ref().map(|l| l.name.clone()),
            });
        }

        if let Some(langs) = &repo.languages {
            for edge in &langs.edges {
                let color = edge
                    .node
                    .color
                    .clone()
                    .unwrap_or_else(|| "#8b949e".to_string());
                let entry = lang_bytes
                    .entry(edge.node.name.clone())
                    .or_insert((0, color));
                entry.0 = entry.0.saturating_add(edge.size);
            }
        }
    }

    let mut languages: Vec<LanguageStat> = lang_bytes
        .into_iter()
        .map(|(name, (bytes, color))| LanguageStat { name, color, bytes })
        .collect();
    languages.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    languages.truncate(6);

    let c = &user.contributions_collection;
    // Placeholder last-year totals; overwritten by lifetime walk in main.
    let commits = c
        .total_commit_contributions
        .saturating_add(c.restricted_contributions_count);

    UserStats {
        login: user.login.unwrap_or_default(),
        name: user.name,
        created_at: user.created_at.unwrap_or_default(),
        followers: user.followers.map(|c| c.total_count).unwrap_or(0),
        following: user.following.map(|c| c.total_count).unwrap_or(0),
        total_stars,
        commits,
        pull_requests: c.total_pull_request_contributions,
        issues: c.total_issue_contributions,
        reviews: c.total_pull_request_review_contributions,
        languages,
        top_repos,
    }
}

/// Fetch lifetime contribution totals by walking yearly windows since account creation.
pub async fn fetch_lifetime_contributions(
    token: &str,
    login: &str,
    created_at: &str,
) -> Result<(u32, u32, u32, u32)> {
    let client = Client::builder()
        .user_agent("ghcamos/0.1")
        .build()?;

    let created = chrono::DateTime::parse_from_rfc3339(created_at)
        .map_err(|e| AppError::Api(format!("bad createdAt: {e}")))?
        .with_timezone(&chrono::Utc);
    let now = chrono::Utc::now();

    let mut year_start = created;
    let mut commits = 0u32;
    let mut prs = 0u32;
    let mut issues = 0u32;
    let mut reviews = 0u32;

    let query = r#"
    query($login: String!, $from: DateTime!, $to: DateTime!) {
      user(login: $login) {
        contributionsCollection(from: $from, to: $to) {
          totalCommitContributions
          totalPullRequestContributions
          totalIssueContributions
          totalPullRequestReviewContributions
          restrictedContributionsCount
        }
      }
    }
    "#;

    while year_start < now {
        let mut year_end = year_start + chrono::Duration::days(365);
        if year_end > now {
            year_end = now;
        }

        let body = json!({
            "query": query,
            "variables": {
                "login": login,
                "from": year_start.to_rfc3339(),
                "to": year_end.to_rfc3339(),
            }
        });

        let response = client
            .post(GITHUB_GRAPHQL)
            .bearer_auth(token)
            .header("Accept", "application/vnd.github+json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::Api(format!("{status}: {text}")));
        }

        let text = response.text().await?;
        let parsed: GqlResponse = serde_json::from_str(&text).map_err(|e| {
            AppError::Api(format!("decode failed: {e}; body={}", truncate(&text, 400)))
        })?;
        if let Some(errors) = parsed.errors {
            let msg = errors
                .into_iter()
                .map(|e| e.message)
                .collect::<Vec<_>>()
                .join("; ");
            return Err(AppError::Api(msg));
        }

        if let Some(user) = parsed.data.and_then(|d| d.user) {
            let c = user.contributions_collection;
            commits = commits
                .saturating_add(c.total_commit_contributions)
                .saturating_add(c.restricted_contributions_count);
            prs = prs.saturating_add(c.total_pull_request_contributions);
            issues = issues.saturating_add(c.total_issue_contributions);
            reviews = reviews.saturating_add(c.total_pull_request_review_contributions);
        }

        year_start = year_end + chrono::Duration::seconds(1);
    }

    Ok((commits, prs, issues, reviews))
}
