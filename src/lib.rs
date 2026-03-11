//! mdbaker — Markdown to PDF with CSS styling.
//!
//! Uses [hyper-render](https://docs.rs/hyper-render) for pure-Rust HTML rendering (no Chrome).

mod merge;
mod split;

use std::path::Path;

use anyhow::{Context, Result};
use hyper_render::{render_to_pdf, Config, OutputFormat};

pub use split::split_markdown_into_pages;

/// Built-in default stylesheet (Rust documentation inspired).
pub const DEFAULT_STYLE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/default_style.css"
));

/// Paper size (dimensions at 96 DPI).
#[derive(Clone, Copy, Debug)]
pub enum PaperSize {
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    Letter,
    Legal,
    Tabloid,
}

impl Default for PaperSize {
    fn default() -> Self {
        Self::A4
    }
}

impl PaperSize {
    fn dimensions_96dpi(self, landscape: bool) -> (u32, u32) {
        let (w, h) = match self {
            Self::A0 => (3179, 4494),
            Self::A1 => (2245, 3179),
            Self::A2 => (1587, 2245),
            Self::A3 => (1123, 1587),
            Self::A4 => (794, 1123),
            Self::A5 => (559, 794),
            Self::A6 => (397, 559),
            Self::Letter => (816, 1056),
            Self::Legal => (816, 1344),
            Self::Tabloid => (1056, 1632),
        };
        if landscape {
            (h, w)
        } else {
            (w, h)
        }
    }
}

/// Line heuristic for page splitting (estimated lines vs actual rendered height).
#[derive(Clone, Debug)]
pub struct LineHeuristic {
    /// Approximate lines per page (e.g. ~50 for A4 portrait with default style).
    pub lines_per_page: f32,
    /// Chars per line for prose (used to estimate line count from text length).
    pub chars_per_line: usize,
}

impl Default for LineHeuristic {
    fn default() -> Self {
        Self {
            lines_per_page: 50.0,
            chars_per_line: 70,
        }
    }
}

/// Options for PDF generation.
#[derive(Clone, Debug)]
pub struct PdfOptions {
    pub paper: PaperSize,
    pub landscape: bool,
    pub scale: Option<f32>,
    pub line_heuristic: Option<LineHeuristic>,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            paper: PaperSize::A4,
            landscape: false,
            scale: None,
            line_heuristic: None,
        }
    }
}

/// Convert markdown and CSS to a single HTML file (no splitting).
pub fn markdown_to_html(markdown: &str, css: &str, output: impl AsRef<Path>) -> Result<()> {
    let mut html_body = String::new();
    pulldown_cmark::html::push_html(
        &mut html_body,
        pulldown_cmark::Parser::new_ext(markdown, pulldown_cmark::Options::all()),
    );
    let full_html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
{css}
</style>
</head>
<body>
{html_body}
</body>
</html>"#,
        css = css,
        html_body = html_body
    );
    std::fs::write(output.as_ref(), full_html).context("failed to write HTML")?;
    Ok(())
}

/// Convert markdown and CSS to a PDF file.
/// Splits content at heading boundaries, then renders and merges pages.
pub fn markdown_to_pdf(
    markdown: &str,
    css: &str,
    output: impl AsRef<Path>,
    opts: PdfOptions,
) -> Result<()> {
    let heuristic = opts
        .line_heuristic
        .as_ref()
        .map(|h| (h.lines_per_page, h.chars_per_line));
    let fragments = split::split_markdown_into_pages(markdown, heuristic);
    html_fragments_to_pdf(&fragments, css, output, opts)
}

/// Render pre-cut HTML body fragments into a multi-page PDF.
///
/// Each fragment should be valid HTML body content (no `<html>`/`<head>` wrapper).
/// The CSS and document skeleton are added automatically around each fragment.
///
/// This is the lower-level entry point: call it directly when you want full
/// control over how content is split across pages.
pub fn html_fragments_to_pdf(
    fragments: &[String],
    css: &str,
    output: impl AsRef<Path>,
    opts: PdfOptions,
) -> Result<()> {
    let (width, height) = opts.paper.dimensions_96dpi(opts.landscape);
    let scale = opts.scale.unwrap_or(1.0);

    let config = Config::new()
        .width(width)
        .height(height)
        .scale(scale)
        .format(OutputFormat::Pdf)
        .auto_height(false);

    let pdf_chunks: Vec<Vec<u8>> = fragments
        .iter()
        .map(|body| {
            let full_html = wrap_html(body, css);
            render_to_pdf(&full_html, config.clone()).context("PDF render failed")
        })
        .collect::<Result<Vec<_>>>()?;

    let pdf_bytes = merge::merge_pdfs(&pdf_chunks).context("merge PDFs")?;
    std::fs::write(output.as_ref(), pdf_bytes).context("failed to write PDF")?;

    Ok(())
}

fn wrap_html(body: &str, css: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
{css}
</style>
</head>
<body>
<br>
<br>
{body}
</body>
</html>"#
    )
}
