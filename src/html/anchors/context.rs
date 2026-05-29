use super::super::text::{clean_html_text, context_body_from_clean_text};

const MAX_CONTEXT_SCAN_BYTES: usize = 2_000;

pub(super) fn extract_context_body(
    document: &str,
    anchor_start: usize,
    anchor_end: usize,
    title: &str,
) -> String {
    let start = find_context_start(document, anchor_start);
    let end = find_context_end(document, anchor_end);
    if start >= end || end > document.len() {
        return String::new();
    }
    let context = clean_html_text(&document[start..end]);
    context_body_from_clean_text(&context, title)
}

fn find_context_start(document: &str, anchor_start: usize) -> usize {
    let lookback_start = floor_char_boundary(
        document,
        anchor_start.saturating_sub(MAX_CONTEXT_SCAN_BYTES),
    );
    let lookback = &document[lookback_start..anchor_start];
    ["<article", "<li", "<section", "<div", "<p"]
        .iter()
        .filter_map(|tag| lookback.rfind(tag).map(|offset| lookback_start + offset))
        .max()
        .unwrap_or(lookback_start)
}

fn find_context_end(document: &str, anchor_end: usize) -> usize {
    let bounded_end = ceil_char_boundary(
        document,
        anchor_end
            .saturating_add(MAX_CONTEXT_SCAN_BYTES)
            .min(document.len()),
    );
    let lookahead = &document[anchor_end..bounded_end];
    ["</article>", "</li>", "</section>", "</div>", "</p>"]
        .iter()
        .filter_map(|tag| {
            lookahead
                .find(tag)
                .map(|offset| anchor_end + offset + tag.len())
        })
        .min()
        .unwrap_or(bounded_end)
}

fn floor_char_boundary(value: &str, mut index: usize) -> usize {
    while index > 0 && !value.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_char_boundary(value: &str, mut index: usize) -> usize {
    while index < value.len() && !value.is_char_boundary(index) {
        index += 1;
    }
    index
}
