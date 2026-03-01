//! mdbaker — Markdown to PDF with CSS styling.
//!
//! Uses [hyper-render](https://docs.rs/hyper-render) for pure-Rust HTML rendering (no Chrome).

use std::path::Path;

use anyhow::{Context, Result};
use hyper_render::{render_to_pdf, Config, OutputFormat};
use pulldown_cmark::{html, Options as MarkdownOptions, Parser as MarkdownParser};

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

/// Options for PDF generation.
#[derive(Clone, Debug)]
pub struct PdfOptions {
    pub paper: PaperSize,
    pub landscape: bool,
    pub scale: Option<f32>,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            paper: PaperSize::A4,
            landscape: false,
            scale: None,
        }
    }
}

/// Convert markdown and CSS to a PDF file.
pub fn markdown_to_pdf(
    markdown: &str,
    css: &str,
    output: impl AsRef<Path>,
    opts: PdfOptions,
) -> Result<()> {
    let parser = MarkdownParser::new_ext(markdown, MarkdownOptions::empty());
    let mut html_body = String::new();
    html::push_html(&mut html_body, parser);

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

    let (width, height) = opts.paper.dimensions_96dpi(opts.landscape);
    let scale = opts.scale.unwrap_or(1.0);

    let config = Config::new()
        .width(width)
        .height(height)
        .scale(scale)
        .format(OutputFormat::Pdf)
        .auto_height(false);

    let pdf_bytes = render_to_pdf(&full_html, config).context("PDF render failed")?;

    std::fs::write(output.as_ref(), pdf_bytes).context("failed to write PDF")?;

    Ok(())
}
