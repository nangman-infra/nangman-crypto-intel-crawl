use super::super::text::attr_value;

const MIN_ANCHOR_TITLE_CHARS: usize = 8;

pub(super) fn extract_href(open_tag: &str) -> Option<String> {
    let value = attr_value(open_tag, "href")?;
    let trimmed = value.trim();
    if is_ignored_href(trimmed) {
        return None;
    }
    Some(trimmed.to_owned())
}

pub(super) fn absolutize_url(base_url: &str, href: &str) -> String {
    if href.starts_with("https://") || href.starts_with("http://") {
        return href.to_owned();
    }
    if href.starts_with('/')
        && let Some((scheme, rest)) = base_url.split_once("://")
        && let Some(host) = rest.split('/').next()
    {
        return format!("{scheme}://{host}{href}");
    }
    let base = base_url.trim_end_matches('/');
    format!("{base}/{href}")
}

pub(super) fn should_skip_anchor(title: &str, href: &str) -> bool {
    title.chars().count() < MIN_ANCHOR_TITLE_CHARS
        || is_navigation_title(title)
        || is_static_asset_link(href)
}

fn is_ignored_href(trimmed: &str) -> bool {
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return true;
    }
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("javascript:")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
        || lower.starts_with("data:")
        || lower.starts_with("vbscript:")
}

fn is_navigation_title(title: &str) -> bool {
    let normalized = title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_uppercase();
    matches!(
        normalized.as_str(),
        "ABOUT"
            | "ABOUT US"
            | "APP"
            | "BLOG"
            | "CAREERS"
            | "CONTACT"
            | "CONTACT US"
            | "COOKIE POLICY"
            | "DISCORD"
            | "DOCS"
            | "DOCUMENTATION"
            | "GITHUB"
            | "GET STARTED"
            | "HOME"
            | "LAUNCH APP"
            | "LEARN MORE"
            | "LOGIN"
            | "MEDIUM"
            | "MENU"
            | "NEWS"
            | "PRIVACY"
            | "PRIVACY POLICY"
            | "READ MORE"
            | "SIGN IN"
            | "TELEGRAM"
            | "TERMS"
            | "TERMS OF SERVICE"
            | "TWITTER"
            | "X"
    )
}

fn is_static_asset_link(href: &str) -> bool {
    let lower = href.split('?').next().unwrap_or(href).to_ascii_lowercase();
    matches!(
        lower.rsplit('.').next(),
        Some("avif" | "css" | "gif" | "ico" | "jpeg" | "jpg" | "js" | "png" | "svg" | "webp")
    )
}
