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
| `--width` | `680` | 360–1600 |
| `--out` / `-o` | stdout | output path |
| `--token` | `GH_TOKEN` / `GITHUB_TOKEN` | PAT or Actions token |

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
- Footer with render timestamp
