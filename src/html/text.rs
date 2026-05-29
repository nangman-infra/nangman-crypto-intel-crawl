const MIN_CONTEXT_BODY_CHARS: usize = 40;
const MAX_CONTEXT_BODY_CHARS: usize = 1_200;
const PAGE_ACTION_SECTION_MARKERS: &[&str] =
    &["how to buy", "where to buy", "buy now", "start trading"];

pub(super) fn extract_meta_content(
    body: &str,
    selector_attr: &str,
    selector_value: &str,
) -> Option<String> {
    let mut cursor = 0;
    while let Some(offset) = body[cursor..].find("<meta") {
        let tag_start = cursor + offset;
        let Some(tag_end_offset) = body[tag_start..].find('>') else {
            break;
        };
        let tag_end = tag_start + tag_end_offset;
        let tag = &body[tag_start..=tag_end];
        let selector_matches = attr_value(tag, selector_attr)
            .is_some_and(|value| value.eq_ignore_ascii_case(selector_value));
        if selector_matches && let Some(content) = attr_value(tag, "content") {
            let cleaned = clean_html_text(&content);
            if !cleaned.trim().is_empty() {
                return Some(cleaned);
            }
        }
        cursor = tag_end + 1;
    }
    None
}

pub(super) fn extract_tag_text(body: &str, tag_name: &str) -> Option<String> {
    let tag_html = extract_tag_html(body, tag_name)?;
    let cleaned = clean_html_text(tag_html);
    (!cleaned.trim().is_empty()).then_some(cleaned)
}

pub(super) fn extract_tag_html<'a>(body: &'a str, tag_name: &str) -> Option<&'a str> {
    let open = format!("<{tag_name}");
    let close = format!("</{tag_name}>");
    let start = body.find(&open)?;
    let open_end = body[start..].find('>').map(|offset| start + offset + 1)?;
    let close_start = body[open_end..]
        .find(&close)
        .map(|offset| open_end + offset)?;
    Some(&body[open_end..close_start])
}

pub(super) fn attr_value(tag: &str, attr_name: &str) -> Option<String> {
    let attr_pos = tag.find(&format!("{attr_name}="))?;
    let after = &tag[attr_pos + attr_name.len() + 1..];
    let mut chars = after.chars();
    let quote = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    Some(chars.take_while(|ch| *ch != quote).collect::<String>())
}

pub(super) fn context_body_from_clean_text(context: &str, title: &str) -> String {
    let trimmed_context = context.trim();
    if trimmed_context.is_empty() {
        return String::new();
    }
    let trimmed_title = title.trim();
    let without_title = trimmed_context
        .strip_prefix(trimmed_title)
        .unwrap_or(trimmed_context)
        .trim_matches(|ch: char| ch.is_whitespace() || matches!(ch, '-' | '|' | ':' | '·'));
    let candidate = if without_title.chars().count() >= MIN_CONTEXT_BODY_CHARS {
        without_title
    } else if trimmed_context != trimmed_title
        && trimmed_context.chars().count() >= MIN_CONTEXT_BODY_CHARS
    {
        trimmed_context
    } else {
        ""
    };
    let candidate = trim_page_action_sections(candidate).trim();
    truncate_clean_text(candidate, MAX_CONTEXT_BODY_CHARS)
}

fn trim_page_action_sections(value: &str) -> &str {
    let lowered = value.to_ascii_lowercase();
    PAGE_ACTION_SECTION_MARKERS
        .iter()
        .filter_map(|marker| lowered.find(marker))
        .filter(|index| value[..*index].chars().count() >= MIN_CONTEXT_BODY_CHARS)
        .min()
        .map(|index| value[..index].trim())
        .unwrap_or(value)
}

fn truncate_clean_text(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push_str("...");
    }
    output
}

pub(super) fn clean_visible_text(value: &str) -> String {
    clean_html_text(&strip_ignored_html_blocks(value))
}

fn strip_ignored_html_blocks(value: &str) -> String {
    let mut output = value.to_owned();
    for tag in ["script", "style", "svg", "noscript"] {
        output = strip_html_block(&output, tag);
    }
    output
}

fn strip_html_block(value: &str, tag_name: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0;
    let open_pattern = format!("<{tag_name}");
    let close_pattern = format!("</{tag_name}>");
    while let Some(start_offset) = value[cursor..].find(&open_pattern) {
        let start = cursor + start_offset;
        output.push_str(&value[cursor..start]);
        let Some(close_offset) = value[start..].find(&close_pattern) else {
            cursor = value.len();
            break;
        };
        cursor = start + close_offset + close_pattern.len();
    }
    output.push_str(&value[cursor..]);
    output
}

pub(super) fn clean_html_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&rsquo;", "'")
        .replace("&ldquo;", "\"")
        .replace("&rdquo;", "\"")
        .replace("&#x27;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
