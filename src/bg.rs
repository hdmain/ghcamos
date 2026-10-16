//! Time-of-day pixel backgrounds from layered `bgs/{mornig,day,night}/N.png`.

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::{Timelike, Utc};
use chrono_tz::Europe::Warsaw;

const SRC_W: f64 = 576.0;
const SRC_H: f64 = 324.0;

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

    /// Layer PNGs bottom→top: sky (static) then cloud sheets (drift).
    fn layers(self) -> &'static [&'static [u8]] {
        match self {
            Self::Morning => &[
                include_bytes!("../bgs/mornig/1.png"),
                include_bytes!("../bgs/mornig/2.png"),
                include_bytes!("../bgs/mornig/3.png"),
                include_bytes!("../bgs/mornig/4.png"),
                include_bytes!("../bgs/mornig/5.png"),
            ],
            Self::Day => &[
                include_bytes!("../bgs/day/1.png"),
                include_bytes!("../bgs/day/2.png"),
                include_bytes!("../bgs/day/3.png"),
                include_bytes!("../bgs/day/4.png"),
            ],
            Self::Night => &[
                include_bytes!("../bgs/night/1.png"),
                include_bytes!("../bgs/night/2.png"),
                include_bytes!("../bgs/night/3.png"),
            ],
        }
    }

    /// Scrim over the pixel art so stats stay readable.
    pub fn scrim(self) -> (&'static str, f64) {
        match self {
            Self::Morning => ("#1a1020", 0.62),
            Self::Day => ("#0d1117", 0.58),
            Self::Night => ("#0d1117", 0.48),
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

fn data_uri(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", B64.encode(png))
}

/// Parallax drift duration (seconds) per layer index; layer 0 is static sky.
fn drift_dur(layer_idx: usize) -> Option<f64> {
    match layer_idx {
        0 => None,
        1 => Some(140.0),
        2 => Some(95.0),
        3 => Some(65.0),
        _ => Some(45.0),
    }
}

fn render_static_layer(uri: &str, w: f64, h: f64, y: f64) -> String {
    format!(
        r##"<image x="0" y="{y:.1}" width="{w:.1}" height="{h:.1}" href="{uri}" xlink:href="{uri}" style="image-rendering:pixelated"/>"##,
        y = y,
        w = w,
        h = h,
        uri = uri,
    )
}

/// Two tiled copies scrolling left for a seamless loop (SMIL on `x`, not CSS transform).
fn render_drifting_layer(uri: &str, w: f64, h: f64, y: f64, dur: f64, bob: f64) -> String {
    let bob_amp = bob;
    format!(
        r##"<g>
  <image x="0" y="{y:.1}" width="{w:.1}" height="{h:.1}" href="{uri}" xlink:href="{uri}" style="image-rendering:pixelated">
    <animate attributeName="x" from="0" to="{neg_w:.1}" dur="{dur:.1}s" repeatCount="indefinite"/>
    <animate attributeName="y" values="{y:.1};{y_up:.1};{y:.1};{y_dn:.1};{y:.1}" dur="{bob_dur:.1}s" repeatCount="indefinite"/>
  </image>
  <image x="{w:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" href="{uri}" xlink:href="{uri}" style="image-rendering:pixelated">
    <animate attributeName="x" from="{w:.1}" to="0" dur="{dur:.1}s" repeatCount="indefinite"/>
    <animate attributeName="y" values="{y:.1};{y_up:.1};{y:.1};{y_dn:.1};{y:.1}" dur="{bob_dur:.1}s" repeatCount="indefinite"/>
  </image>
</g>"##,
        y = y,
        w = w,
        h = h,
        uri = uri,
        neg_w = -w,
        dur = dur,
        y_up = y - bob_amp,
        y_dn = y + bob_amp,
        bob_dur = dur * 0.35,
    )
}

pub fn render_background(period: Period, width: u32, height: u32) -> String {
    let card_w = width as f64;
    let card_h = height as f64;
    let scale = (card_w / SRC_W).max(card_h / SRC_H);
    let disp_w = SRC_W * scale;
    let disp_h = SRC_H * scale;
    // Prefer anchoring clouds toward the bottom of the tall card.
    let y = (card_h - disp_h).min(0.0);

    let (scrim, opacity) = period.scrim();
    let accent = period.accent();

    let mut layers = String::new();
    for (i, png) in period.layers().iter().enumerate() {
        let uri = data_uri(png);
        match drift_dur(i) {
            None => layers.push_str(&render_static_layer(&uri, disp_w, disp_h, y)),
            Some(dur) => {
                let bob = 1.5 + i as f64 * 0.5;
                layers.push_str(&render_drifting_layer(&uri, disp_w, disp_h, y, dur, bob));
            }
        }
        layers.push('\n');
    }

    format!(
        r##"<defs>
  <clipPath id="cardClip"><rect width="{cw}" height="{ch}" rx="12"/></clipPath>
</defs>
<g clip-path="url(#cardClip)">
  {layers}
  <rect width="{cw}" height="{ch}" fill="{scrim}" opacity="{opacity}"/>
</g>
<rect width="{cw}" height="{ch}" rx="12" fill="none" stroke="{border}" stroke-width="1"/>
<rect class="accent-line" x="24" y="4" width="48" height="3" rx="1.5" fill="{accent}"/>
"##,
        cw = width,
        ch = height,
        layers = layers,
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
