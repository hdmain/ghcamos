//! Last.fm recent / now-playing track for the stats card.

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::error::{AppError, Result};

const LASTFM_API: &str = "https://ws.audioscrobbler.com/2.0/";
/// Public readme widget used only as a keyless fallback (aria-label + art scrape).
const FALLBACK_SVG: &str = "https://lastfm-recently-played.jeffreyca.workers.dev/svg";

#[derive(Debug, Clone)]
pub struct RecentTrack {
    pub artist: String,
    pub name: String,
    pub album: Option<String>,
    pub now_playing: bool,
    /// Self-contained `data:image/...;base64,...` for Camo-safe SVG embeds.
    pub image_data_uri: Option<String>,
}

impl RecentTrack {
    pub fn label(&self) -> &'static str {
        if self.now_playing {
            "Now playing"
        } else {
            "Last played"
        }
    }
}

#[derive(Debug, Deserialize)]
struct ApiRoot {
    recenttracks: Option<ApiRecentTracks>,
    message: Option<String>,
    error: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ApiRecentTracks {
    track: Option<Value>,
}

pub async fn fetch_recent_track(user: &str, api_key: Option<&str>) -> Result<Option<RecentTrack>> {
    if let Some(key) = api_key.filter(|k| !k.is_empty()) {
        match fetch_via_api(user, key).await {
            Ok(track) => return Ok(track),
            Err(err) => {
                eprintln!("lastfm api: {err}; trying fallback");
            }
        }
    } else {
        eprintln!("lastfm: no LASTFM_API_KEY; using fallback widget");
    }
    fetch_via_fallback_svg(user).await
}

async fn fetch_via_api(user: &str, api_key: &str) -> Result<Option<RecentTrack>> {
    let client = Client::builder()
        .user_agent("ghcamos/0.1 (+https://github.com/hdmain/ghcamos)")
        .build()?;

    let response = client
        .get(LASTFM_API)
        .query(&[
            ("method", "user.getrecenttracks"),
            ("user", user),
            ("api_key", api_key),
            ("limit", "1"),
            ("format", "json"),
        ])
        .send()
        .await?;

    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        return Err(AppError::Api(format!("last.fm {status}: {text}")));
    }

    let parsed: ApiRoot = serde_json::from_str(&text)?;
    if let Some(code) = parsed.error {
        let msg = parsed.message.unwrap_or_else(|| "unknown".into());
        return Err(AppError::Api(format!("last.fm error {code}: {msg}")));
    }

    let Some(rt) = parsed.recenttracks else {
        return Ok(None);
    };
    let Some(track_val) = rt.track else {
        return Ok(None);
    };

    let track_obj = match track_val {
        Value::Array(arr) => arr.into_iter().next(),
        Value::Object(map) => Some(Value::Object(map)),
        _ => None,
    };

    let Some(track) = track_obj else {
        return Ok(None);
    };

    let mut recent = parse_track_json(&track)?;
    if let Some(url) = pick_image_url(&track) {
        recent.image_data_uri = fetch_image_data_uri(&client, &url).await.ok().flatten();
    }
    Ok(Some(recent))
}

fn parse_track_json(track: &Value) -> Result<RecentTrack> {
    let name = track
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let artist = track
        .get("artist")
        .map(|a| {
            if let Some(s) = a.as_str() {
                s.to_string()
            } else {
                a.get("#text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string()
            }
        })
        .unwrap_or_else(|| "Unknown".into());

    let album = track
        .get("album")
        .and_then(|a| a.get("#text").and_then(|v| v.as_str()))
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let now_playing = track
        .get("@attr")
        .and_then(|a| a.get("nowplaying"))
        .and_then(|v| v.as_str())
        .map(|s| s == "true")
        .unwrap_or(false);

    Ok(RecentTrack {
        artist,
        name,
        album,
        now_playing,
        image_data_uri: None,
    })
}

fn pick_image_url(track: &Value) -> Option<String> {
    let images = track.get("image")?.as_array()?;
    // Prefer medium, then large, then whatever is non-empty.
    let prefer = ["medium", "large", "extralarge", "small"];
    for size in prefer {
        for img in images {
            let sz = img.get("size").and_then(|v| v.as_str()).unwrap_or("");
            let url = img
                .get("#text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if sz == size && !url.is_empty() {
                return Some(url.to_string());
            }
        }
    }
    images.iter().rev().find_map(|img| {
        let url = img.get("#text").and_then(|v| v.as_str()).unwrap_or("").trim();
        if url.is_empty() {
            None
        } else {
            Some(url.to_string())
        }
    })
}

async fn fetch_image_data_uri(client: &Client, url: &str) -> Result<Option<String>> {
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Ok(None);
    }
    let ctype = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
    let bytes = response.bytes().await?;
    if bytes.is_empty() || bytes.len() > 250_000 {
        return Ok(None);
    }
    let mime = if ctype.starts_with("image/") {
        ctype.split(';').next().unwrap_or("image/jpeg")
    } else {
        "image/jpeg"
    };
    Ok(Some(format!(
        "data:{mime};base64,{}",
        B64.encode(bytes)
    )))
}

async fn fetch_via_fallback_svg(user: &str) -> Result<Option<RecentTrack>> {
    let client = Client::builder()
        .user_agent("ghcamos/0.1 (+https://github.com/hdmain/ghcamos)")
        .build()?;

    let response = client
        .get(FALLBACK_SVG)
        .query(&[("user", user), ("count", "1")])
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(AppError::Api(format!(
            "last.fm fallback {status}: {text}"
        )));
    }

    let svg = response.text().await?;
    Ok(parse_fallback_svg(&svg))
}

fn parse_fallback_svg(svg: &str) -> Option<RecentTrack> {
    let label = extract_attr(svg, "aria-label")?;
    // "Scrobbling now: Track by Artist" | "Last played: Track by Artist"
    let (now_playing, rest) = if let Some(r) = label.strip_prefix("Scrobbling now: ") {
        (true, r)
    } else if let Some(r) = label.strip_prefix("Last played: ") {
        (false, r)
    } else if let Some(r) = label.strip_prefix("Recently played: ") {
        (false, r)
    } else {
        return None;
    };

    let (name, artist) = split_track_by_artist(rest)?;
    let image_data_uri = extract_data_image(svg);
    Some(RecentTrack {
        artist,
        name,
        album: None,
        now_playing,
        image_data_uri,
    })
}

fn extract_data_image(svg: &str) -> Option<String> {
    let markers = [
        "href=\"data:image/",
        "xlink:href=\"data:image/",
    ];
    for marker in markers {
        if let Some(start) = svg.find(marker) {
            let from = start + marker.len() - "data:image/".len();
            let end = svg[from..].find('"')? + from;
            let uri = &svg[from..end];
            if uri.starts_with("data:image/") && uri.contains(";base64,") {
                return Some(uri.to_string());
            }
        }
    }
    None
}

fn split_track_by_artist(s: &str) -> Option<(String, String)> {
    // Prefer last " by " so titles containing " by " still work reasonably.
    let idx = s.rfind(" by ")?;
    let name = s[..idx].trim().to_string();
    let artist = s[idx + 4..].trim().to_string();
    if name.is_empty() || artist.is_empty() {
        None
    } else {
        Some((name, artist))
    }
}

fn extract_attr(svg: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=\"");
    let start = svg.find(&needle)? + needle.len();
    let end = svg[start..].find('"')? + start;
    Some(svg[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_now_playing_aria() {
        let svg = r#"<svg aria-label="Scrobbling now: Uppercut by JDHD beats" href="data:image/png;base64,aaa"></svg>"#;
        let t = parse_fallback_svg(svg).unwrap();
        assert!(t.now_playing);
        assert_eq!(t.name, "Uppercut");
        assert_eq!(t.artist, "JDHD beats");
        assert_eq!(
            t.image_data_uri.as_deref(),
            Some("data:image/png;base64,aaa")
        );
    }
}
