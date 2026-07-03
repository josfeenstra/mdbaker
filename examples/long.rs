//! Example: generate a multi-page PDF from long markdown.
//!
//! Run with: `cargo run --example long`

use anyhow::Result;
use mdbaker::{markdown_to_pdf, PdfOptions, DEFAULT_STYLE};

fn main() -> Result<()> {
    let section = |heading: &str, count: usize, start: usize| -> String {
        let paras: String = (start..start + count)
            .map(|i| {
                format!(
                    "\nThis is paragraph {}. It contains enough text to simulate \
                     a real document with multiple blocks. Each paragraph will be \
                     counted by the page-splitting heuristic, and when the estimated \
                     line count exceeds the page capacity, the content will flow to \
                     the next page. Lorem ipsum dolor sit amet, consectetur adipiscing elit.\n",
                    i
                )
            })
            .collect();
        format!("{heading}\n{paras}")
    };

    let markdown = format!(
        r#"# Multi-Page PDF Example

This document is deliberately long to trigger page breaks.

{}
{}
{}

## Conclusion

The PDF should now span multiple pages, all merged into a single file.
"#,
        section("## Part One: Introduction", 8, 1),
        section("## Part Two: Development", 10, 9),
        section("### Subsection: Details", 8, 19),
    );

    markdown_to_pdf(&markdown, DEFAULT_STYLE, "long.pdf", PdfOptions::default())?;

    println!("Generated long.pdf");

    Ok(())
}
