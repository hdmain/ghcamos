//! Time-of-day pixel backgrounds from `bgs/{mornig,day,night}/orig.png`.

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::{Timelike, Utc};
use chrono_tz::Europe::Warsaw;

const MORNING: &[u8] = include_bytes!("../bgs/mornig/orig.png");
const DAY: &[u8] = include_bytes!("../bgs/day/orig.png");
const NIGHT: &[u8] = include_bytes!("../bgs/night/orig.png");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Morning,
    Day,
    Night,
}

impl Period {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Morning => "morning",
            Self::Day => "day",
            Self::Night => "night",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "morning" | "mornig" | "sunrise" => Some(Self::Morning),
            "day" | "daytime" | "afternoon" => Some(Self::Day),
            "night" | "evening" => Some(Self::Night),
            "auto" => Some(Self::from_now()),
            _ => None,
        }
    }

    /// Local time in Europe/Warsaw:
    /// morning 05–11, day 11–18, night otherwise.
    pub fn from_now() -> Self {
        let now = Utc::now().with_timezone(&Warsaw);
        match now.hour() {
            5..11 => Self::Morning,
            11..18 => Self::Day,
            _ => Self::Night,
        }
    }

    fn png(self) -> &'static [u8] {
        match self {
            Self::Morning => MORNING,
            Self::Day => DAY,
            Self::Night => NIGHT,
        }
    }

    /// Scrim over the pixel art so stats stay readable.
    pub fn scrim(self) -> (&'static str, f64) {
        match self {
            Self::Morning => ("#1a1020", 0.72),
            Self::Day => ("#0d1117", 0.68),
            Self::Night => ("#0d1117", 0.55),
        }
    }

    pub fn accent(self) -> &'static str {
        match self {
            Self::Morning => "#ff9f6b",
            Self::Day => "#58a6ff",
            Self::Night => "#c9d1d9",
        }
    }
}

pub fn render_background(period: Period, width: u32, height: u32) -> String {
    let uri = format!("data:image/png;base64,{}", B64.encode(period.png()));
    let (scrim, opacity) = period.scrim();
    let accent = period.accent();

    format!(
        r##"<defs>
  <clipPath id="cardClip"><rect width="{w}" height="{h}" rx="12"/></clipPath>
</defs>
<g clip-path="url(#cardClip)">
  <image width="{w}" height="{h}" href="{uri}" xlink:href="{uri}" preserveAspectRatio="xMidYMid slice" style="image-rendering:pixelated"/>
  <rect width="{w}" height="{h}" fill="{scrim}" opacity="{opacity}"/>
</g>
<rect width="{w}" height="{h}" rx="12" fill="none" stroke="{border}" stroke-width="1"/>
<rect class="accent-line" x="24" y="4" width="48" height="3" rx="1.5" fill="{accent}"/>
"##,
        w = width,
        h = height,
        uri = uri,
        scrim = scrim,
        opacity = opacity,
        border = "#30363d",
        accent = accent,
    )
}

pub fn period_label(period: Period) -> String {
    let stamp = Utc::now().with_timezone(&Warsaw);
    format!(
        "{} · {:02}:{:02} Warsaw",
        period.as_str(),
        stamp.hour(),
        stamp.minute(),
    )
}
