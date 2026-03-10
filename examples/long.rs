//! Example: generate a multi-page PDF from long markdown.
//!
//! Run with: `cargo run --example long`

use anyhow::Result;
use mdbaker::{markdown_to_pdf, PdfOptions, DEFAULT_STYLE};

fn main() -> Result<()> {
    let paragraphs: Vec<String> = (1..=25)
        .map(|i| {
            format!(
                "This is paragraph number {}. It contains enough text to simulate \
                 a real document with multiple blocks. Each paragraph will be \
                 counted by the page-splitting heuristic, and when the estimated \
                 line count exceeds the page capacity, the content will flow to \
                 the next page. Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
                i
            )
        })
        .collect();

    let markdown = format!(
        r#"# Multi-Page PDF Example

This document is deliberately long to trigger page breaks.

{}

## Conclusion

The PDF should now span multiple pages, all merged into a single file.
"#,
        paragraphs
            .iter()
            .map(|p| format!("\n{}\n", p))
            .collect::<String>()
    );

    markdown_to_pdf(
        &markdown,
        DEFAULT_STYLE,
        "long.pdf",
        PdfOptions::default(),
    )?;

    println!("Generated long.pdf");

    Ok(())
}
