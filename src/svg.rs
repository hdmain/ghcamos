use chrono::Utc;

use crate::bg::{self, Period};
use crate::fire::{self, render_corner_fires};
use crate::github::{Trend, UserStats};
use crate::homelab::{self, HomelabHistory, HomelabStats};
use crate::lastfm::RecentTrack;
use crate::theme::Theme;

pub fn render_stats_card(
    stats: &UserStats,
    theme: &Theme,
    width: u32,
    period: Period,
    show_fires: bool,
    lastfm: Option<&RecentTrack>,
    homelab: Option<&HomelabStats>,
    homelab_history: Option<&HomelabHistory>,
    trends: [Trend; 6],
) -> String {
    let width = width.clamp(360, 1600);
    let pad = 24.0;
    let inner_w = width as f64 - pad * 2.0;

    let header_h = 52.0;
    let stats_h = 120.0;
    let langs_h = if stats.languages.is_empty() {
        0.0
    } else {
        28.0 + stats.languages.len() as f64 * 28.0
    };
    let repos_h = if stats.top_repos.is_empty() {
        0.0
    } else {
        28.0 + stats.top_repos.len() as f64 * 28.0
    };
    let footer_h = 28.0;
    let lastfm_h = if lastfm.is_some() { 64.0 } else { 0.0 };
    let homelab_h = if homelab.is_some() { 168.0 } else { 0.0 };
    let fire_h = if show_fires {
        fire::display_height() * 0.55
    } else {
        0.0
    };
    let gap = 16.0;

    let mut y = pad;
    let mut sections = String::new();

    sections.push_str(&render_header(stats, theme, pad, y, inner_w));
    y += header_h + gap;

    sections.push_str(&render_stat_grid(stats, theme, pad, y, inner_w, stats_h, trends));
    y += stats_h + gap;

    if !stats.languages.is_empty() {
        sections.push_str(&render_languages(stats, theme, pad, y, inner_w));
        y += langs_h + gap;
    }

    if !stats.top_repos.is_empty() {
        sections.push_str(&render_repos(stats, theme, pad, y, inner_w));
        y += repos_h + gap;
    }

    if let Some(host) = homelab {
        sections.push_str(&render_homelab(
            host,
            homelab_history,
            theme,
            pad,
            y,
            inner_w,
        ));
        y += homelab_h + gap;
    }

    if let Some(track) = lastfm {
        sections.push_str(&render_lastfm(track, theme, pad, y, inner_w));
        y += lastfm_h + gap;
    }

    let footer = render_footer(theme, pad, y, inner_w, period, show_fires);
    y += footer_h + fire_h;
    y += pad;

    let height = y.ceil() as u32;
    let fires = if show_fires {
        render_corner_fires(width, height)
    } else {
        String::new()
    };
    let background = bg::render_background(period, width, height);

    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-label="{login} GitHub stats">
  <title>{login} GitHub stats</title>
  <style>
    text {{ font-family: 'Segoe UI', Helvetica, Arial, sans-serif; }}
    image {{ image-rendering: pixelated; image-rendering: crisp-edges; }}
    /* Opacity-only: CSS transform would override SVG transform="" and break layout. */
    @keyframes fadeIn {{
      from {{ opacity: 0; }}
      to {{ opacity: 1; }}
    }}
    @keyframes softPulse {{
      0%, 100% {{ opacity: 0.35; }}
      50% {{ opacity: 0.85; }}
    }}
    .fade-in {{
      opacity: 0;
      animation: fadeIn 0.55s ease-out forwards;
    }}
    .accent-line {{
      animation: softPulse 2.8s ease-in-out infinite;
    }}
  </style>
  {background}
  {sections}
  {footer}
  {fires}
</svg>
"##,
        width = width,
        height = height,
        login = escape_xml(&stats.login),
        background = background,
        sections = sections,
        footer = footer,
        fires = fires,
    )
}

fn render_header(stats: &UserStats, theme: &Theme, x: f64, y: f64, _w: f64) -> String {
    let display = stats
        .name
        .as_deref()
        .filter(|n| !n.is_empty())
        .unwrap_or(&stats.login);
    let years = years_on_github(&stats.created_at);
    let member = format_member_since(&stats.created_at);

    format!(
        r##"<g class="fade-in" style="animation-delay:0.05s" transform="translate({x:.1},{y:.1})">
  <text x="0" y="20" fill="{title}" font-size="20" font-weight="600">{display}</text>
  <text x="0" y="42" fill="{muted}" font-size="13">@{login} · Member since {member} · {years} on GitHub</text>
</g>"##,
        x = x,
        y = y,
        title = theme.title,
        muted = theme.muted,
        display = escape_xml(display),
        login = escape_xml(&stats.login),
        member = escape_xml(&member),
        years = escape_xml(&years),
    )
}

fn render_stat_grid(
    stats: &UserStats,
    theme: &Theme,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    trends: [Trend; 6],
) -> String {
    let cells = [
        ("Commits", format_num(stats.commits)),
        ("PRs", format_num(stats.pull_requests)),
        ("Issues", format_num(stats.issues)),
        ("Reviews", format_num(stats.reviews)),
        ("Stars", format_num(stats.total_stars)),
        ("Followers", format_num(stats.followers)),
    ];

    let cols = 3usize;
    let rows = 2usize;
    let gap = 10.0;
    let cell_w = (w - gap * (cols as f64 - 1.0)) / cols as f64;
    let cell_h = (h - gap * (rows as f64 - 1.0)) / rows as f64;

    let mut out = format!(r#"<g transform="translate({x:.1},{y:.1})">"#);

    for (i, (label, value)) in cells.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let cx = col as f64 * (cell_w + gap);
        let cy = row as f64 * (cell_h + gap);
        let delay = 0.12 + i as f64 * 0.06;
        let arrow = trend_arrow(trends[i]);
        out.push_str(&format!(
            r##"<g class="fade-in" style="animation-delay:{delay:.2}s" transform="translate({cx:.1},{cy:.1})">
  <rect width="{cell_w:.1}" height="{cell_h:.1}" rx="8" fill="{card}"/>
  <text x="12" y="20" fill="{muted}" font-size="12">{label}</text>
  <text x="12" y="42" fill="{title}" font-size="20" font-weight="600">{value}{arrow}</text>
</g>"##,
            delay = delay,
            cx = cx,
            cy = cy,
            cell_w = cell_w,
            cell_h = cell_h,
            card = theme.card,
            muted = theme.muted,
            title = theme.title,
            label = label,
            value = value,
            arrow = arrow,
        ));
    }

    out.push_str("</g>");
    out
}

fn trend_arrow(trend: Trend) -> String {
    match trend {
        Trend::Up => {
            r##"<tspan fill="#3fb950" font-size="14" font-weight="700"> ↑</tspan>"##.to_string()
        }
        Trend::Down => {
            r##"<tspan fill="#f85149" font-size="14" font-weight="700"> ↓</tspan>"##.to_string()
        }
        Trend::Flat => String::new(),
    }
}

fn render_languages(stats: &UserStats, theme: &Theme, x: f64, y: f64, w: f64) -> String {
    let total: u64 = stats.languages.iter().map(|l| l.bytes).sum::<u64>().max(1);
    let bar_w = w;
    let mut out = format!(
        r##"<g class="fade-in" style="animation-delay:0.45s" transform="translate({x:.1},{y:.1})">
  <text x="0" y="14" fill="{title}" font-size="14" font-weight="600">Top languages</text>
"##,
        x = x,
        y = y,
        title = theme.title,
    );

    let mut row_y = 34.0;
    for (i, lang) in stats.languages.iter().enumerate() {
        let pct = lang.bytes as f64 / total as f64;
        let fill_w = (bar_w * pct).max(2.0);
        let bar_delay = 0.55 + i as f64 * 0.08;
        // SMIL width grow only — no CSS transform on positioned elements.
        out.push_str(&format!(
            r##"<text x="0" y="{label_y:.1}" fill="{text}" font-size="12">{name}</text>
<text x="{bar_w:.1}" y="{label_y:.1}" fill="{muted}" font-size="12" text-anchor="end">{pct:.0}%</text>
<rect x="0" y="{track_y:.1}" width="{bar_w:.1}" height="6" rx="3" fill="{track}"/>
<rect x="0" y="{track_y:.1}" width="0" height="6" rx="3" fill="{color}">
  <animate attributeName="width" from="0" to="{fill_w:.1}" dur="0.85s" begin="{bar_delay:.2}s" fill="freeze" calcMode="spline" keySplines="0.22 1 0.36 1" keyTimes="0;1"/>
</rect>
"##,
            label_y = row_y,
            text = theme.text,
            muted = theme.muted,
            name = escape_xml(&lang.name),
            bar_w = bar_w,
            pct = pct * 100.0,
            track_y = row_y + 6.0,
            track = theme.bar_track,
            bar_delay = bar_delay,
            fill_w = fill_w,
            color = &lang.color,
        ));
        row_y += 28.0;
    }

    out.push_str("</g>");
    out
}

fn render_repos(stats: &UserStats, theme: &Theme, x: f64, y: f64, w: f64) -> String {
    let mut out = format!(
        r##"<g transform="translate({x:.1},{y:.1})">
  <text class="fade-in" style="animation-delay:0.7s" x="0" y="14" fill="{title}" font-size="14" font-weight="600">Top repositories</text>
"##,
        x = x,
        y = y,
        title = theme.title,
    );

    let mut row_y = 36.0;
    for (i, repo) in stats.top_repos.iter().enumerate() {
        let lang = repo.language.as_deref().unwrap_or("-");
        let delay = 0.78 + i as f64 * 0.08;
        out.push_str(&format!(
            r##"<g class="fade-in" style="animation-delay:{delay:.2}s">
  <rect x="0" y="{ry:.1}" width="{w:.1}" height="24" rx="6" fill="{card}"/>
  <text x="10" y="{ty:.1}" fill="{accent}" font-size="13" font-weight="600">{name}</text>
  <text x="{mx:.1}" y="{ty:.1}" fill="{muted}" font-size="12" text-anchor="end">★ {stars} · {lang}</text>
</g>
"##,
            delay = delay,
            ry = row_y - 16.0,
            w = w,
            card = theme.card,
            ty = row_y,
            accent = theme.accent,
            name = escape_xml(&repo.name),
            mx = w - 10.0,
            muted = theme.muted,
            stars = repo.stars,
            lang = escape_xml(lang),
        ));
        row_y += 28.0;
    }

    out.push_str("</g>");
    out
}

fn render_homelab(
    host: &HomelabStats,
    history: Option<&HomelabHistory>,
    theme: &Theme,
    x: f64,
    y: f64,
    w: f64,
) -> String {
    let temp = host
        .package_temp_c()
        .map(|t| format!("{t:.0}°C"))
        .unwrap_or_else(|| "-".into());
    let cells = [
        (
            "CPU load",
            format!("{:.2} / {}", host.loadavg[0], host.cpu.cores),
        ),
        (
            "RAM",
            format!(
                "{} / {}",
                homelab::format_bytes(host.memory.used_bytes),
                homelab::format_bytes(host.memory.total_bytes)
            ),
        ),
        (
            "Swap",
            format!(
                "{} / {}",
                homelab::format_bytes(host.swap.used_bytes),
                homelab::format_bytes(host.swap.total_bytes)
            ),
        ),
        ("Temp", temp),
        ("Up", host.uptime_human()),
    ];

    let cols = 3usize;
    let gap = 8.0;
    let cell_w = (w - gap * (cols as f64 - 1.0)) / cols as f64;
    let cell_h = 36.0;

    let mut grid = String::new();
    for (i, (label, value)) in cells.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let cx = col as f64 * (cell_w + gap);
        let cy = 28.0 + row as f64 * (cell_h + gap);
        grid.push_str(&format!(
            r##"<g transform="translate({cx:.1},{cy:.1})">
  <rect width="{cell_w:.1}" height="{cell_h:.1}" rx="7" fill="{card}"/>
  <text x="10" y="14" fill="{muted}" font-size="10">{label}</text>
  <text x="10" y="28" fill="{title}" font-size="12" font-weight="600">{value}</text>
</g>"##,
            cx = cx,
            cy = cy,
            cell_w = cell_w,
            cell_h = cell_h,
            card = theme.card,
            muted = theme.muted,
            title = theme.title,
            label = label,
            value = escape_xml(value),
        ));
    }

    let charts_y = 28.0 + 2.0 * (cell_h + gap) + 8.0;
    let charts = render_homelab_charts(history, theme, w, charts_y);
    let hot = if host.is_hot() { " · hot" } else { "" };

    format!(
        r##"<g class="fade-in" style="animation-delay:0.8s" transform="translate({x:.1},{y:.1})">
  <text x="0" y="14" fill="{title}" font-size="14" font-weight="600">Homelab</text>
  <text x="{w:.1}" y="14" fill="{muted}" font-size="11" text-anchor="end">last 4×15m{hot}</text>
  {grid}
  {charts}
</g>"##,
        x = x,
        y = y,
        title = theme.title,
        w = w,
        muted = theme.muted,
        hot = hot,
        grid = grid,
        charts = charts,
    )
}

fn render_homelab_charts(
    history: Option<&HomelabHistory>,
    theme: &Theme,
    w: f64,
    y: f64,
) -> String {
    let Some(history) = history.filter(|h| h.samples.len() >= 2) else {
        return format!(
            r##"<text x="0" y="{y:.1}" fill="{muted}" font-size="10">charts appear after a few cron samples</text>"##,
            y = y + 12.0,
            muted = theme.muted,
        );
    };

    let gap = 10.0;
    let chart_w = (w - gap) / 2.0;
    let chart_h = 44.0;

    let cpu_vals: Vec<f32> = history.samples.iter().map(|s| s.cpu_pct).collect();
    let mem_vals: Vec<f32> = history.samples.iter().map(|s| s.mem_pct()).collect();

    let cpu_delta = cpu_vals.last().copied().unwrap_or(0.0) - cpu_vals.first().copied().unwrap_or(0.0);
    let mem_delta = mem_vals.last().copied().unwrap_or(0.0) - mem_vals.first().copied().unwrap_or(0.0);

    let left = sparkline_panel(
        "CPU",
        &cpu_vals,
        cpu_delta,
        theme,
        0.0,
        y,
        chart_w,
        chart_h,
        theme.accent,
    );
    let right = sparkline_panel(
        "RAM",
        &mem_vals,
        mem_delta,
        theme,
        chart_w + gap,
        y,
        chart_w,
        chart_h,
        "#3fb950",
    );

    format!("{left}{right}")
}

fn sparkline_panel(
    title: &str,
    values: &[f32],
    delta_pp: f32,
    theme: &Theme,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    stroke: &str,
) -> String {
    let pad_x = 10.0;
    let pad_top = 18.0;
    let pad_bot = 8.0;
    let plot_w = w - pad_x * 2.0;
    let plot_h = h - pad_top - pad_bot;
    let n = values.len().max(2) as f64;
    let max_v = values
        .iter()
        .copied()
        .fold(1.0_f32, f32::max)
        .max(1.0);

    let mut points = String::new();
    for (i, v) in values.iter().enumerate() {
        let px = pad_x + (i as f64) * (plot_w / (n - 1.0));
        let py = pad_top + plot_h * (1.0 - (*v / max_v) as f64);
        if i > 0 {
            points.push(' ');
        }
        points.push_str(&format!("{px:.1},{py:.1}"));
    }

    let delta_s = if delta_pp >= 0.0 {
        format!("+{delta_pp:.1}pp")
    } else {
        format!("{delta_pp:.1}pp")
    };

    format!(
        r##"<g transform="translate({x:.1},{y:.1})">
  <rect width="{w:.1}" height="{h:.1}" rx="7" fill="{card}"/>
  <text x="10" y="13" fill="{muted}" font-size="10">{title} · 4 samples</text>
  <text x="{right:.1}" y="13" fill="{accent}" font-size="10" text-anchor="end">{delta}</text>
  <polyline fill="none" stroke="{stroke}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" points="{points}"/>
</g>"##,
        x = x,
        y = y,
        w = w,
        h = h,
        card = theme.card,
        muted = theme.muted,
        title = title,
        right = w - 10.0,
        accent = theme.accent,
        delta = delta_s,
        stroke = stroke,
        points = points,
    )
}

fn render_lastfm(track: &RecentTrack, theme: &Theme, x: f64, y: f64, w: f64) -> String {
    let status = track.label();
    let line = truncate_chars(&format!("{} - {}", track.artist, track.name), 48);
    let album = track
        .album
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| truncate_chars(s, 48))
        .unwrap_or_default();

    let art = if let Some(uri) = track.image_data_uri.as_deref() {
        format!(
            r##"<defs>
  <clipPath id="lfmArt"><rect x="8" y="28" width="28" height="28" rx="5"/></clipPath>
</defs>
<image x="8" y="28" width="28" height="28" href="{uri}" xlink:href="{uri}" clip-path="url(#lfmArt)" preserveAspectRatio="xMidYMid slice"/>
<rect x="8" y="28" width="28" height="28" rx="5" fill="none" stroke="{border}" stroke-width="1" opacity="0.55"/>"##,
            uri = escape_xml(uri),
            border = theme.border,
        )
    } else {
        format!(
            r##"<rect x="8" y="28" width="28" height="28" rx="5" fill="{border}"/>
<text x="22" y="47" text-anchor="middle" fill="{muted}" font-size="14">♪</text>"##,
            border = theme.bar_track,
            muted = theme.muted,
        )
    };

    format!(
        r##"<g class="fade-in" style="animation-delay:0.85s" transform="translate({x:.1},{y:.1})">
  <text x="0" y="14" fill="{title}" font-size="14" font-weight="600">Last.fm</text>
  <rect x="0" y="22" width="{w:.1}" height="40" rx="8" fill="{card}"/>
  {art}
  <text x="44" y="40" fill="{muted}" font-size="11">{status}</text>
  <text x="44" y="56" fill="{accent}" font-size="13" font-weight="600">{line}</text>
  <title>{tip}</title>
</g>"##,
        x = x,
        y = y,
        title = theme.title,
        w = w,
        card = theme.card,
        art = art,
        muted = theme.muted,
        status = escape_xml(status),
        accent = theme.accent,
        line = escape_xml(&line),
        tip = escape_xml(&format!(
            "{}: {} - {}{}",
            status,
            track.artist,
            track.name,
            if album.is_empty() {
                String::new()
            } else {
                format!(" ({album})")
            }
        )),
    )
}

fn truncate_chars(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn render_footer(
    theme: &Theme,
    x: f64,
    y: f64,
    _w: f64,
    period: Period,
    show_fires: bool,
) -> String {
    let stamp = Utc::now().format("%Y-%m-%d UTC").to_string();
    let period_label = bg::period_label(period);
    let credit = if show_fires {
        " · ghcamos · fire by Devkidd"
    } else {
        " · ghcamos"
    };
    format!(
        r##"<g class="fade-in" style="animation-delay:1s" transform="translate({x:.1},{y:.1})">
  <text x="0" y="14" fill="{muted}" font-size="11">Updated {stamp} · {period}{credit}</text>
</g>"##,
        x = x,
        y = y,
        muted = theme.muted,
        stamp = stamp,
        period = escape_xml(&period_label),
        credit = credit,
    )
}

fn format_num(n: u32) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 10_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn format_member_since(created_at: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(created_at)
        .map(|d| d.format("%b %Y").to_string())
        .unwrap_or_else(|_| created_at.to_string())
}

fn years_on_github(created_at: &str) -> String {
    let Ok(created) = chrono::DateTime::parse_from_rfc3339(created_at) else {
        return "-".into();
    };
    let days = (Utc::now() - created.with_timezone(&Utc)).num_days();
    let years = (days as f64 / 365.25).floor() as i64;
    if years < 1 {
        let months = (days / 30).max(1);
        format!("{months} mo")
    } else if years == 1 {
        "1 yr".into()
    } else {
        format!("{years} yrs")
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
