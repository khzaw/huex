# huex

`huex` is a CLI that extracts dominant colors from an image and returns either compact human-readable terminal output or structured JSON for downstream agents.

It clusters pixels in Oklab, uses k-means++ initialization, stops when centroid movement falls below a perceptual Delta-E threshold, and merges near-duplicate palette entries after clustering.

## Features

- Agent-friendly `--json` output with hex, RGB, Oklab coordinates, population, and percentage.
- Compact default terminal output with ANSI swatches, hex values, and percentages.
- `--sort population|luminance|hue` to control color ordering.
- `--harmony` computes color-theory harmonies (complement, analogous, triadic, split-complement, tetradic, or all) for each dominant color.
- Optional `--rgb` and `--verbose` terminal modes.
- Deterministic sampling and initialization via `--seed`.
- Accepts `--image <PATH>` or a positional image path.
- Supports `-` as the input path to read image bytes from stdin.

## Install

### Homebrew

```bash
brew install khzaw/tap/huex
```

### From source

```bash
cargo build --release
```

The binary will be available at `target/release/huex`.

## Usage

```bash
# Human-readable output
huex --image ./fixtures/duo.ppm

# Human-readable output with RGB values
huex --image ./fixtures/duo.ppm --rgb

# Detailed terminal report
huex --image ./fixtures/duo.ppm --verbose

# Sort by luminance (lightest first)
huex --image ./fixtures/duo.ppm --sort luminance

# Sort by hue (color wheel order)
huex --image ./fixtures/duo.ppm --sort hue

# Compute complement harmonies for each dominant color
huex --image ./fixtures/duo.ppm --harmony complement

# All harmony types with detailed output
huex --image ./fixtures/duo.ppm --harmony all --verbose

# Harmonies in JSON
huex --image ./fixtures/duo.ppm --harmony triadic --json

# JSON output for scripts and agents
huex --image ./fixtures/duo.ppm --json

# Read from stdin
cat ./fixtures/duo.ppm | huex --image - --json

# Run without installing
cargo run -- --image ./fixtures/duo.ppm --json
```

## Flags

- `--image <PATH>`: image path, or `-` for stdin.
- `-k, --k <N>`: requested number of clusters before deduplication. Default: `5`.
- `--iter <N>`: maximum k-means iterations. Default: `50`.
- `--sample <N>`: max sampled pixels for clustering. Use `0` to cluster all visible pixels. Default: `10000`.
- `--seed <N>`: deterministic seed for sampling and k-means++ initialization. Default: `42`.
- `--rgb`: include RGB values in the compact terminal output.
- `--verbose`: show the detailed terminal report.
- `--hex`: print one hex color per line, ideal for piping.
- `--json`: emit JSON instead of ANSI text.
- `--sort <MODE>`: sort output colors by `population` (default), `luminance` (lightest first), or `hue` (color wheel order).
- `--harmony <MODE>`: compute color harmonies for each dominant color. Modes: `complement`, `analogous`, `triadic`, `split-complement`, `tetradic`, `all`.

## Notes

- Transparent pixels are composited over white before analysis.
- Final palette percentages are computed against all visible pixels, not just the sampled set.
- Post-processing merges any centroids within Delta-E `< 5.0`, so the final palette may contain fewer than `k` colors.
