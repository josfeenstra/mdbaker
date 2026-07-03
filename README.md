# mdbaker
[DISCLAIMER: FULLY AI SLOBBED]

Convert Markdown to PDF with CSS styling. Uses [hyper-render](https://docs.rs/hyper-render) — pure Rust, no Chrome.

Overflowing content is split across multiple pages automatically and merged into a single PDF.

## Example

Run the bundled example (generates `brief.pdf`):

```bash
cargo run --example brief
```

Multi-page example (generates `long.pdf`):

```bash
cargo run --example long
```

Using the built-in default style on a file:

```bash
mdbaker brief.md
```

With a custom stylesheet:

```bash
mdbaker brief.md -s style.css
```

With optional PDF options:

```bash
mdbaker brief.md -s style.css -o output.pdf --paper letter --orientation landscape --scale 1.2
```

## Options

| Flag | Description |
|------|-------------|
| `-s, --style` | CSS file (optional; uses built-in default when omitted) |
| `-o, --output` | Output PDF path (default: input stem + `.pdf`) |
| `--paper` | Paper size: a0–a6, letter, legal, tabloid (default: a4) |
| `--orientation` | portrait or landscape (default: portrait) |
| `--scale` | Scale factor (0.1–2.0) |

## Requirements

None — pure Rust, no external binaries.

**Limitations (from hyper-render):** System fonts only (no `@font-face`), no external images.

**Multi-page:** Uses a heuristic (block-level markdown + estimated line count) to split content at sensible boundaries; very long blocks (e.g. huge code blocks) may still overflow a single page.

## Installation

- (Install rust)[https://rust-lang.org/]
- `git clone https://github.com/josfeenstra/mdbaker`
- `cd mdbaker`
- `cargo install --path .`
- `mdbaker --help`
