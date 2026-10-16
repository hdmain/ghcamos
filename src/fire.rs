//! Pixel fire sprites from Devkidd's Pixel Fire Asset Pack v1.2
//! (https://devkidd.itch.io/pixel-fire-asset-pack) — free to use in projects.

use base64::{engine::general_purpose::STANDARD as B64, Engine};

/// Idle red flame sprite sheet: 8 frames × 32×48 (256×48).
const FIRE_SHEET: &[u8] = include_bytes!("../assets/fire/red-idle-sheet.png");

const FRAME_W: f64 = 32.0;
const FRAME_H: f64 = 48.0;
const FRAME_COUNT: u32 = 8;
const SCALE: f64 = 2.0;

pub fn display_width() -> f64 {
    FRAME_W * SCALE
}

pub fn display_height() -> f64 {
    FRAME_H * SCALE
}

fn sheet_data_uri() -> String {
    format!("data:image/png;base64,{}", B64.encode(FIRE_SHEET))
}

/// Two mirrored pixel fires for the bottom corners of the card.
pub fn render_corner_fires(card_width: u32, card_height: u32) -> String {
    let uri = sheet_data_uri();
    let dw = display_width();
    let dh = display_height();
    let pad = 16.0;
    let y = card_height as f64 - dh - 8.0;
    let left_x = pad;
    let right_x = card_width as f64 - pad - dw;

    let mut values = Vec::with_capacity(FRAME_COUNT as usize + 1);
    for i in 0..FRAME_COUNT {
        values.push(format!("{:.0}", -(i as f64 * FRAME_W)));
    }
    values.push("0".into());
    let x_values = values.join(";");

    format!(
        r##"<defs>
  <clipPath id="fireClip"><rect width="{fw:.0}" height="{fh:.0}"/></clipPath>
</defs>
<g class="fade-in" style="animation-delay:0.9s;image-rendering:pixelated" transform="translate({left_x:.1},{y:.1}) scale({scale})">
  <g clip-path="url(#fireClip)">
    <image width="{sheet_w:.0}" height="{fh:.0}" href="{uri}" xlink:href="{uri}" x="0" y="0" style="image-rendering:pixelated">
      <animate attributeName="x" values="{x_values}" dur="0.64s" repeatCount="indefinite" calcMode="discrete"/>
    </image>
  </g>
</g>
<g class="fade-in" style="animation-delay:0.95s;image-rendering:pixelated" transform="translate({right_edge:.1},{y:.1}) scale(-{scale},{scale})">
  <g clip-path="url(#fireClip)">
    <image width="{sheet_w:.0}" height="{fh:.0}" href="{uri}" xlink:href="{uri}" x="0" y="0" style="image-rendering:pixelated">
      <animate attributeName="x" values="{x_values}" dur="0.64s" begin="0.08s" repeatCount="indefinite" calcMode="discrete"/>
    </image>
  </g>
</g>
"##,
        fw = FRAME_W,
        fh = FRAME_H,
        sheet_w = FRAME_W * FRAME_COUNT as f64,
        uri = uri,
        x_values = x_values,
        left_x = left_x,
        y = y,
        scale = SCALE,
        right_edge = right_x + dw,
    )
}
