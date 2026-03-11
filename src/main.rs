use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use mdbaker::{
    markdown_to_html, markdown_to_pdf, LineHeuristic, PaperSize, PdfOptions, DEFAULT_STYLE,
};

#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
enum Paper {
    A0,
    A1,
    A2,
    A3,
    #[default]
    A4,
    A5,
    A6,
    Letter,
    Legal,
    Tabloid,
}

impl From<Paper> for PaperSize {
    fn from(p: Paper) -> Self {
        match p {
            Paper::A0 => PaperSize::A0,
            Paper::A1 => PaperSize::A1,
            Paper::A2 => PaperSize::A2,
            Paper::A3 => PaperSize::A3,
            Paper::A4 => PaperSize::A4,
            Paper::A5 => PaperSize::A5,
            Paper::A6 => PaperSize::A6,
            Paper::Letter => PaperSize::Letter,
            Paper::Legal => PaperSize::Legal,
            Paper::Tabloid => PaperSize::Tabloid,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

#[derive(Parser, Debug)]
#[command(name = "mdbaker")]
#[command(about = "Convert Markdown to PDF with CSS styling (pure Rust, no Chrome)")]
struct Args {
    /// Markdown input file
    input: PathBuf,

    /// CSS stylesheet file (default: built-in Rust-docs style)
    #[arg(short, long)]
    style: Option<PathBuf>,

    /// Output PDF file (default: input stem with .pdf extension)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Paper size
    #[arg(long, value_enum, default_value_t = Paper::A4)]
    paper: Paper,

    /// Page orientation
    #[arg(long, value_enum, default_value_t = Orientation::Portrait)]
    orientation: Orientation,

    /// Scale factor (0.1–2.0)
    #[arg(long)]
    scale: Option<f32>,

    /// Lines per page for split heuristic (default: 50)
    #[arg(long)]
    lines_per_page: Option<f32>,

    /// Chars per line for prose in split heuristic (default: 70)
    #[arg(long)]
    chars_per_line: Option<usize>,

    /// Output HTML file instead of PDF (no page splitting)
    #[arg(long)]
    html: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let markdown = std::fs::read_to_string(&args.input)
        .with_context(|| format!("failed to read markdown file: {}", args.input.display()))?;

    let css = match &args.style {
        Some(path) => std::fs::read_to_string(path)
            .with_context(|| format!("failed to read CSS file: {}", path.display()))?,
        None => DEFAULT_STYLE.to_string(),
    };

    let output = args
        .output
        .unwrap_or_else(|| default_output_path(&args.input, args.html));

    if args.html {
        markdown_to_html(&markdown, &css, &output)?;
    } else {
        let line_heuristic = match (args.lines_per_page, args.chars_per_line) {
            (None, None) => None,
            (lpp, cpl) => Some(LineHeuristic {
                lines_per_page: lpp.unwrap_or(55.0),
                chars_per_line: cpl.unwrap_or(70),
            }),
        };
        let opts = PdfOptions {
            paper: args.paper.into(),
            landscape: matches!(args.orientation, Orientation::Landscape),
            scale: args.scale,
            line_heuristic,
        };
        markdown_to_pdf(&markdown, &css, &output, opts)?;
    }

    Ok(())
}

fn default_output_path(input: &PathBuf, html: bool) -> PathBuf {
    let mut out = input.clone();
    if let Some(stem) = input.file_stem() {
        out.set_file_name(stem);
    }
    out.set_extension(if html { "html" } else { "pdf" });
    out
}
