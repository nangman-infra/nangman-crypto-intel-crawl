pub(super) struct AnchorScan<'a> {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) open_tag: &'a str,
    pub(super) title_html: &'a str,
}

pub(super) fn next_anchor(document: &str, cursor: usize) -> Option<AnchorScan<'_>> {
    let mut search_cursor = cursor;
    loop {
        let anchor_offset = document[search_cursor..].find("<a")?;
        let anchor_start = search_cursor + anchor_offset;
        if !is_anchor_tag_start(document, anchor_start) {
            search_cursor = anchor_start + "<a".len();
            continue;
        }
        let anchor_open_end_offset = document[anchor_start..].find('>')?;
        let anchor_open_end = anchor_start + anchor_open_end_offset;
        let title_start = anchor_open_end + 1;
        let anchor_close_offset = document[title_start..].find("</a>")?;
        let anchor_close = title_start + anchor_close_offset;
        let anchor_end = anchor_close + "</a>".len();
        return Some(AnchorScan {
            start: anchor_start,
            end: anchor_end,
            open_tag: &document[anchor_start..=anchor_open_end],
            title_html: &document[title_start..anchor_close],
        });
    }
}

fn is_anchor_tag_start(document: &str, anchor_start: usize) -> bool {
    document[anchor_start + "<a".len()..]
        .chars()
        .next()
        .is_some_and(|ch| ch == '>' || ch.is_ascii_whitespace())
}
