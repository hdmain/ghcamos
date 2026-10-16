# ghcamos

Self-contained GitHub stats SVG cards for profile READMEs. Rendered in Rust, published via GitHub Pages — no runtime server. Compatible with GitHub Camo (system fonts only, no remote assets).

## Embed

After the workflow publishes Pages:

```markdown
![Stats](https://hdmain.github.io/ghcamos/stats.svg)
```

Append `?v=1` (bump as needed) if Camo serves a stale cache.

## Local usage

```bash
# PowerShell
$env:GH_TOKEN = (gh auth token)
cargo run --release -- render --user hdmain --out stats.svg
```

```bash
# bash
export GH_TOKEN="$(gh auth token)"
cargo run --release -- render --user hdmain --out stats.svg
```

### Flags

| Flag | Default | Notes |
|------|---------|-------|
| `--user` | `hdmain` | GitHub login |
| `--theme` | `github_dark` | `github_dark` · `light` · `nord` |
| `--period` | `auto` | `auto` · `morning` · `day` · `night` (auto = Europe/Warsaw: 05–11 / 11–18 / else) |
| `--fires` | off | corner pixel-fire sprites (kept in repo, opt-in) |
| `--lastfm-user` | `hdmain` | Last.fm username ([profile](https://www.last.fm/user/hdmain)) |
| `--lastfm-api-key` | `LASTFM_API_KEY` | optional; without it a public widget fallback is used |
| `--no-lastfm` | off | hide the Last.fm section |
| `--width` | `680` | 360–1600 |
| `--out` / `-o` | stdout | output path |
| `--token` | `GH_TOKEN` / `GITHUB_TOKEN` | PAT or Actions token |

Optional: create a free [Last.fm API key](https://www.last.fm/api/account/create) and add repo secret `LASTFM_API_KEY`.

Pixel sky backgrounds live in [`bgs/`](bgs/) (`mornig`, `day`, `night`) as stacked layers (`1.png` sky + drifting clouds). Cron every 15m picks the period from Warsaw local time; cloud layers slowly scroll via SMIL.

## GitHub Actions

Workflow [`.github/workflows/render.yml`](.github/workflows/render.yml) builds, renders `dist/stats.svg`, and deploys to GitHub Pages every 15 minutes (plus on push / manual dispatch).

One-time setup:

1. Push this repo to GitHub.
2. Settings → Pages → Source = **GitHub Actions**.
3. (Optional) Add a classic PAT with `read:user` (and `repo` for private contribution counts) as secret `STATS_GH_TOKEN`.

## Card contents

- Header: display name, `@login`, member since, years on GitHub
- Lifetime stats: commits, PRs, issues, reviews, owned stars, followers
- Top languages (by bytes across owned non-fork repos)
- Top 3 owned repos by stars
- Last.fm now-playing / last-played ([hdmain](https://www.last.fm/user/hdmain))
- Optional pixel fire corners (`--fires`) from [Devkidd — Pixel Fire Asset Pack v1.2](https://devkidd.itch.io/pixel-fire-asset-pack)
- Footer with render timestamp
