//! Example: generate a PDF from embedded markdown using the built-in default style.
//!
//! Run with: `cargo run --example brief`

use anyhow::Result;
use mdbaker::{markdown_to_pdf, PdfOptions, DEFAULT_STYLE};

const EXAMPLE_MARKDOWN: &str = r#"# mdbaker Example Brief

This example demonstrates the **built-in default style** (Rust documentation inspired).

this is a line
this is another line

this is a third line, after two enters


This is a fourth line, after a triple enter.

## Usage

```bash
cargo run --example brief
```

## Features

- CommonMark markdown parsing
- Optional CSS (built-in default when omitted)
- Paper size, orientation, margins
- Code blocks with syntax-friendly styling

## Sample Code

```rust
fn main() {
    println!("Hello, mdbaker!");
}
```

## Blockquote

> The built-in style resembles Rust's official documentation layout.
"#;

fn main() -> Result<()> {
    let output = "brief.pdf";

    markdown_to_pdf(
        EXAMPLE_MARKDOWN,
        DEFAULT_STYLE,
        output,
        PdfOptions::default(),
    )?;

    println!("Generated {output}");

    Ok(())
}
