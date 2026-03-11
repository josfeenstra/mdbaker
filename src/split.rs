//! Split markdown into page-sized HTML fragments at heading boundaries.
//!
//! Only h1/h2/h3 headings are treated as valid page-break points.
//! Sections between headings are grouped into pages using estimated line costs
//! calibrated for A4 portrait with the default style.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag};

/// Approximate lines per page for A4 portrait with default style.
/// Body: 11px font, 1.6 line-height, ~950px usable height ≈ 54 lines.
/// Use conservative value to avoid overflow.
const LINES_PER_PAGE: f32 = 50.0;

/// Chars per line for typical prose (65em max-width, 11px).
const CHARS_PER_LINE: usize = 70;

/// Split markdown into HTML body fragments, one per page.
///
/// Content is split only at h1/h2/h3 heading boundaries. Adjacent sections
/// are grouped together until their combined estimated cost would exceed a
/// page, at which point a new page begins.
///
/// If a single section between headings exceeds a full page, it will still
/// occupy one page (overflow). A future two-pass renderer can address this
/// by measuring rendered height.
///
/// `heuristic` is `Some((lines_per_page, chars_per_line))` to override the
/// built-in defaults. Pass `None` to use the defaults (50 lines/page, 70 chars/line).
pub fn split_markdown_into_pages(
    markdown: &str,
    heuristic: Option<(f32, usize)>,
) -> Vec<String> {
    let (lines_per_page, chars_per_line) = heuristic.unwrap_or((LINES_PER_PAGE, CHARS_PER_LINE));
    let events: Vec<Event> = Parser::new_ext(markdown, Options::empty()).collect();
    if events.is_empty() {
        return vec![];
    }

    let sections = split_events_at_headings(&events, chars_per_line);
    group_into_pages(&events, &sections, lines_per_page)
}

/// (start_event_index, end_event_index exclusive, estimated line cost)
type Section = (usize, usize, f32);

/// Walk the event stream and produce one section per heading boundary.
/// The first section may not start with a heading if there is content before
/// the first h1/h2/h3.
fn split_events_at_headings(events: &[Event], chars_per_line: usize) -> Vec<Section> {
    let mut heading_indices: Vec<usize> = Vec::new();

    for (i, event) in events.iter().enumerate() {
        if let Event::Start(Tag::Heading { level, .. }) = event {
            if matches!(level, HeadingLevel::H1 | HeadingLevel::H2 | HeadingLevel::H3) {
                heading_indices.push(i);
            }
        }
    }

    let mut boundaries: Vec<usize> = Vec::new();

    if heading_indices.first().map_or(true, |&h| h > 0) {
        boundaries.push(0);
    }
    boundaries.extend(&heading_indices);

    boundaries
        .iter()
        .enumerate()
        .map(|(i, &start)| {
            let end = boundaries.get(i + 1).copied().unwrap_or(events.len());
            let cost = estimate_cost(&events[start..end], chars_per_line);
            (start, end, cost)
        })
        .collect()
}

/// Estimate how many "lines" a slice of events will occupy on the page.
fn estimate_cost(events: &[Event], chars_per_line: usize) -> f32 {
    let mut lines = 0.0f32;
    let mut prose_chars = 0usize;
    let mut in_code_block = false;

    for event in events {
        match event {
            Event::Start(Tag::Heading { .. }) => lines += 2.5,
            Event::Start(Tag::Paragraph) => lines += 0.5,
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                lines += 1.5;
            }
            Event::Start(Tag::BlockQuote) => lines += 1.0,
            Event::Start(Tag::List(_)) => lines += 0.5,
            Event::Start(Tag::Item) => lines += 1.0,
            Event::Start(Tag::Table(_)) => lines += 1.0,
            Event::Start(Tag::TableHead) => lines += 1.0,
            Event::Start(Tag::TableRow) => lines += 0.5,
            Event::Start(Tag::HtmlBlock) => lines += 2.0,
            Event::Rule => lines += 2.0,
            Event::HardBreak => lines += 1.0,
            Event::Text(s) if in_code_block => {
                lines += s.lines().count().max(1) as f32;
            }
            Event::Text(s) | Event::Code(s) => prose_chars += s.len(),
            Event::End(_) if in_code_block => in_code_block = false,
            _ => {}
        }
    }

    if prose_chars > 0 {
        lines += (prose_chars as f32 / chars_per_line as f32).ceil();
    }
    lines.max(1.0)
}

/// Group sections into pages. Each page accumulates sections until adding the
/// next one would exceed `lines_per_page`, then a new page starts.
fn group_into_pages(events: &[Event], sections: &[Section], lines_per_page: f32) -> Vec<String> {
    if sections.is_empty() {
        return vec![events_to_html(events)];
    }

    let mut pages: Vec<String> = Vec::new();
    let mut page_start = 0usize;
    let mut page_cost = 0.0f32;

    for (i, &(_, _, cost)) in sections.iter().enumerate() {
        if page_cost + cost > lines_per_page && i > page_start {
            let ev_start = sections[page_start].0;
            let ev_end = sections[i].0;
            pages.push(events_to_html(&events[ev_start..ev_end]));
            page_start = i;
            page_cost = 0.0;
        }
        page_cost += cost;
    }

    let ev_start = sections[page_start].0;
    let ev_end = sections.last().unwrap().1;
    pages.push(events_to_html(&events[ev_start..ev_end]));

    pages
}

fn events_to_html(events: &[Event]) -> String {
    let mut out = String::new();
    pulldown_cmark::html::push_html(&mut out, events.iter().cloned());
    out
}
