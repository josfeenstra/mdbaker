//! Heuristic splitting of markdown into page-sized chunks.
//!
//! Splits at block boundaries (paragraphs, headings, code blocks, list items, etc.)
//! using estimated line counts calibrated for A4 with the default style.

use pulldown_cmark::{Event, Tag};

/// Approximate lines per page for A4 portrait with default style.
/// Body: 11px font, 1.6 line-height, ~950px usable height ≈ 54 lines.
/// Use conservative value to avoid overflow.
const LINES_PER_PAGE: f32 = 55.0;

/// Chars per line for typical prose (65em max-width, 11px).
const CHARS_PER_LINE: usize = 70;

/// Split markdown into chunks that should fit on single PDF pages.
/// Each chunk is a complete HTML string (body content only) for that page.
pub fn split_markdown_into_chunks<'a>(markdown: &'a str) -> Vec<String> {
    let events: Vec<Event<'a>> =
        pulldown_cmark::Parser::new_ext(markdown, pulldown_cmark::Options::empty())
            .into_iter()
            .collect();

    if events.is_empty() {
        return vec![];
    }

    let blocks = collect_blocks_with_costs(&events);
    let mut chunks = Vec::new();
    let mut current_events: Vec<Event<'a>> = Vec::new();
    let mut current_cost: f32 = 0.0;

    for (start, end, cost) in blocks {
        if current_cost + cost > LINES_PER_PAGE && !current_events.is_empty() {
            let html = events_to_html(&current_events);
            chunks.push(html);
            current_events.clear();
            current_cost = 0.0;
        }
        current_events.extend(events[start..=end].iter().cloned());
        current_cost += cost;
    }

    if !current_events.is_empty() {
        chunks.push(events_to_html(&current_events));
    }

    if chunks.is_empty() {
        chunks.push(events_to_html(&events));
    }

    chunks
}

/// Block as (start_event_idx, end_event_idx, estimated_line_cost).
fn collect_blocks_with_costs(events: &[Event<'_>]) -> Vec<(usize, usize, f32)> {
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < events.len() {
        let start = i;
        let (end, cost) = scan_block(events, i);
        blocks.push((start, end, cost));
        i = end + 1;
    }

    blocks
}

/// Scan from event `start` to end of block, return (end_idx, cost).
fn scan_block(events: &[Event<'_>], start: usize) -> (usize, f32) {
    match &events[start] {
        Event::Start(tag) => {
            let mut depth = 1;
            let mut cost = block_cost(tag);
            let mut text_chars = 0;
            let mut code_lines = 0;

            let mut i = start + 1;
            while i < events.len() && depth > 0 {
                match &events[i] {
                    Event::Start(_) => depth += 1,
                    Event::End(_) => depth -= 1,
                    Event::Text(s) => text_chars += s.len(),
                    Event::Code(s) => text_chars += s.len(),
                    Event::HardBreak => cost += 1.0,
                    Event::SoftBreak => {}
                    _ => {}
                }
                if matches!(tag, Tag::CodeBlock(_)) {
                    if let Event::Text(s) = &events[i] {
                        code_lines += s.lines().count().max(1);
                    }
                }
                i += 1;
            }

            let end = i.saturating_sub(1);

            if matches!(tag, Tag::CodeBlock(_)) {
                cost = 2.0 + code_lines as f32;
            } else if text_chars > 0 {
                cost += (text_chars as f32 / CHARS_PER_LINE as f32).ceil();
            }

            (end, cost.max(1.0))
        }
        Event::Rule => (start, 2.0),
        _ => {
            let mut i = start;
            while i < events.len() {
                if matches!(&events[i], Event::Start(_) | Event::End(_)) {
                    break;
                }
                i += 1;
            }
            (i.min(events.len() - 1), 1.0)
        }
    }
}

fn block_cost(tag: &Tag<'_>) -> f32 {
    match tag {
        Tag::Paragraph => 1.0,
        Tag::Heading { .. } => 2.0,
        Tag::BlockQuote => 1.0,
        Tag::CodeBlock(_) => 2.0,
        Tag::List(_) => 0.5,
        Tag::Item => 1.0,
        Tag::Table(_) => 1.0,
        Tag::TableHead => 1.0,
        Tag::TableCell => 0.5,
        Tag::TableRow => 0.5,
        Tag::HtmlBlock => 2.0,
        Tag::FootnoteDefinition(_) => 1.0,
        Tag::MetadataBlock(_) => 0.0,
        _ => 1.0,
    }
}

fn events_to_html(events: &[Event<'_>]) -> String {
    let mut out = String::new();
    pulldown_cmark::html::push_html(&mut out, events.iter().cloned());
    out
}
