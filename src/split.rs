//! Split markdown into page-sized HTML fragments at semantic boundaries.
//!
//! H1 headings produce the cover page, H2 headings produce table-of-contents
//! entries and always start new pages, and H3 headings are treated as
//! subsection boundaries for the page-size heuristic.
//! Sections between headings are grouped into pages using estimated line costs
//! calibrated for A4 portrait with the default style.

use std::collections::HashMap;

use pulldown_cmark::{CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

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
pub fn split_markdown_into_pages(markdown: &str, heuristic: Option<(f32, usize)>) -> Vec<String> {
    split_markdown_into_pages_with_date(markdown, heuristic, "")
}

pub fn split_markdown_into_pages_with_date(
    markdown: &str,
    heuristic: Option<(f32, usize)>,
    current_date: &str,
) -> Vec<String> {
    let (lines_per_page, chars_per_line) = heuristic.unwrap_or((LINES_PER_PAGE, CHARS_PER_LINE));
    let events: Vec<Event> = Parser::new_ext(markdown, Options::empty()).collect();
    if events.is_empty() {
        return vec![];
    }

    let document = digest_document(&events, chars_per_line);
    let events = add_section_ids(&events, &document.section_slugs_by_index);
    emit_pages(&events, &document, lines_per_page, current_date)
}

#[derive(Clone, Debug)]
struct Document {
    cover: Option<Cover>,
    toc: Vec<TocEntry>,
    chunks: Vec<Chunk>,
    section_slugs_by_index: HashMap<usize, String>,
}

#[derive(Clone, Debug)]
struct Cover {
    title_lines: Vec<String>,
    body_start: usize,
    body_end: usize,
}

#[derive(Clone, Debug)]
struct TocEntry {
    title: String,
    slug: String,
    start: usize,
}

#[derive(Clone, Debug)]
struct Chunk {
    start: usize,
    end: usize,
    cost: f32,
    force_new_page: bool,
}

fn digest_document(events: &[Event], chars_per_line: usize) -> Document {
    let (cover, content_start) = digest_cover(events);
    let toc = collect_toc(events, content_start);
    let section_slugs_by_index = toc
        .iter()
        .map(|entry| (entry.start, entry.slug.clone()))
        .collect();
    let chunks = digest_chunks(events, content_start, chars_per_line);

    Document {
        cover,
        toc,
        chunks,
        section_slugs_by_index,
    }
}

fn digest_cover(events: &[Event]) -> (Option<Cover>, usize) {
    let Some(first_h1) = events
        .iter()
        .position(|event| heading_level(event) == Some(HeadingLevel::H1))
    else {
        return (None, 0);
    };

    let mut title_lines = Vec::new();
    let mut index = first_h1;

    while let Some((HeadingLevel::H1, title, end)) = heading_at(events, index) {
        title_lines.push(title);
        index = end;
    }

    let body_start = index;
    while index < events.len() {
        if matches!(events[index], Event::Rule) || heading_level(&events[index]).is_some() {
            break;
        }
        index += 1;
    }

    (
        Some(Cover {
            title_lines,
            body_start,
            body_end: index,
        }),
        index,
    )
}

fn collect_toc(events: &[Event], content_start: usize) -> Vec<TocEntry> {
    let mut entries = Vec::new();
    let mut used_slugs = HashMap::<String, usize>::new();

    for index in content_start..events.len() {
        let Some((HeadingLevel::H2, title, _)) = heading_at(events, index) else {
            continue;
        };
        let slug = unique_slug(&title, &mut used_slugs);
        entries.push(TocEntry {
            title,
            slug,
            start: index,
        });
    }

    entries
}

fn digest_chunks(events: &[Event], content_start: usize, chars_per_line: usize) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut start = content_start;
    let mut next_forced_by_rule = false;

    for index in content_start..events.len() {
        if matches!(events[index], Event::Rule) {
            if push_chunk(
                events,
                &mut chunks,
                start,
                index,
                next_forced_by_rule,
                chars_per_line,
            ) {
                next_forced_by_rule = false;
            }
            start = index + 1;
            next_forced_by_rule = true;
            continue;
        }

        let Some(level) = heading_level(&events[index]) else {
            continue;
        };
        if !matches!(
            level,
            HeadingLevel::H1 | HeadingLevel::H2 | HeadingLevel::H3
        ) {
            continue;
        }

        let pushed = push_chunk(
            events,
            &mut chunks,
            start,
            index,
            next_forced_by_rule,
            chars_per_line,
        );
        start = index;
        next_forced_by_rule = (!pushed && next_forced_by_rule)
            || matches!(level, HeadingLevel::H1 | HeadingLevel::H2);
    }

    push_chunk(
        events,
        &mut chunks,
        start,
        events.len(),
        next_forced_by_rule,
        chars_per_line,
    );

    chunks
}

fn push_chunk(
    events: &[Event],
    chunks: &mut Vec<Chunk>,
    start: usize,
    end: usize,
    force_new_page: bool,
    chars_per_line: usize,
) -> bool {
    if start >= end || events[start..end].iter().all(is_ignorable_event) {
        return false;
    }

    chunks.push(Chunk {
        start,
        end,
        cost: estimate_cost(&events[start..end], chars_per_line),
        force_new_page,
    });
    true
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

fn emit_pages(
    events: &[Event],
    document: &Document,
    lines_per_page: f32,
    current_date: &str,
) -> Vec<String> {
    let mut pages = Vec::new();

    if let Some(cover) = &document.cover {
        pages.push(cover_to_html(events, cover, current_date));
    }

    if !document.toc.is_empty() {
        pages.push(toc_to_html(&document.toc));
    }

    let mut page_events = Vec::new();
    let mut page_cost = 0.0f32;

    for chunk in &document.chunks {
        if chunk.force_new_page && !page_events.is_empty() {
            pages.push(events_to_html(&page_events));
            page_events.clear();
            page_cost = 0.0;
        }

        if page_cost + chunk.cost > lines_per_page && !page_events.is_empty() {
            pages.push(events_to_html(&page_events));
            page_events.clear();
            page_cost = 0.0;
        }

        page_events.extend(events[chunk.start..chunk.end].iter().cloned());
        page_cost += chunk.cost;
    }

    if !page_events.is_empty() {
        pages.push(events_to_html(&page_events));
    }

    pages
}

fn add_section_ids<'a>(
    events: &[Event<'a>],
    section_slugs_by_index: &HashMap<usize, String>,
) -> Vec<Event<'a>> {
    events
        .iter()
        .enumerate()
        .map(
            |(index, event)| match (section_slugs_by_index.get(&index), event) {
                (
                    Some(slug),
                    Event::Start(Tag::Heading {
                        level,
                        classes,
                        attrs,
                        ..
                    }),
                ) => Event::Start(Tag::Heading {
                    level: *level,
                    id: Some(CowStr::Boxed(slug.clone().into_boxed_str())),
                    classes: classes.clone(),
                    attrs: attrs.clone(),
                }),
                _ => event.clone(),
            },
        )
        .collect()
}

fn cover_to_html(events: &[Event], cover: &Cover, current_date: &str) -> String {
    let title = cover
        .title_lines
        .iter()
        .map(|line| format!("<span>{}</span>", escape_html(line)))
        .collect::<Vec<_>>()
        .join("<br>");
    let body = events_to_html(&events[cover.body_start..cover.body_end]);
    let date = if current_date.is_empty() {
        String::new()
    } else {
        format!(
            r#"<p class="mdbaker-cover-date">{}</p>"#,
            escape_html(current_date)
        )
    };

    format!(
        r#"<section class="mdbaker-cover"><div><h1>{title}</h1>{date}<div class="mdbaker-cover-body">{body}</div></div></section>"#
    )
}

fn toc_to_html(entries: &[TocEntry]) -> String {
    let links = entries
        .iter()
        .map(|entry| {
            format!(
                r##"<li><a href="#{}">{}</a></li>"##,
                escape_html(&entry.slug),
                escape_html(&entry.title)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(r#"<section class="mdbaker-toc"><h1>Table of Contents</h1><ol>{links}</ol></section>"#)
}

fn events_to_html(events: &[Event]) -> String {
    let mut out = String::new();
    pulldown_cmark::html::push_html(&mut out, events.iter().cloned());
    out
}

fn heading_at(events: &[Event], index: usize) -> Option<(HeadingLevel, String, usize)> {
    let Event::Start(Tag::Heading { level, .. }) = events.get(index)? else {
        return None;
    };

    let mut title = String::new();

    for (offset, event) in events[index + 1..].iter().enumerate() {
        match event {
            Event::Text(text) | Event::Code(text) => title.push_str(text),
            Event::SoftBreak | Event::HardBreak => title.push(' '),
            Event::End(TagEnd::Heading(_)) => {
                return Some((*level, title.trim().to_string(), index + offset + 2));
            }
            _ => {}
        }
    }

    None
}

fn heading_level(event: &Event) -> Option<HeadingLevel> {
    match event {
        Event::Start(Tag::Heading { level, .. }) => Some(*level),
        _ => None,
    }
}

fn is_ignorable_event(event: &Event) -> bool {
    match event {
        Event::Rule => true,
        Event::Text(text) => text.trim().is_empty(),
        Event::SoftBreak | Event::HardBreak => true,
        _ => false,
    }
}

fn unique_slug(title: &str, used_slugs: &mut HashMap<String, usize>) -> String {
    let base = slugify(title);
    let count = used_slugs.entry(base.clone()).or_insert(0);
    *count += 1;

    if *count == 1 {
        base
    } else {
        format!("{base}-{}", *count)
    }
}

fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for ch in title.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "section".to_string()
    } else {
        slug
    }
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
