use std::fmt::Write as _;

/// Escape text for use as HTML element content (escapes `&`, `<`, `>`).
/// Thin wrapper around the `html-escape` crate — escaping is
/// context-sensitive (text vs. attribute), so this and `escape_attr` are
/// deliberately not the same function.
pub fn escape_text(s: &str) -> std::borrow::Cow<'_, str> {
    html_escape::encode_text(s)
}

/// Escape text for use inside a double-quoted HTML attribute value
/// (escapes `&`, `<`, `>`, `"`).
pub fn escape_attr(s: &str) -> std::borrow::Cow<'_, str> {
    html_escape::encode_double_quoted_attribute(s)
}

/// Shared by `images::rewrite_images` and `math::render_math`: both let an
/// image or a display-math block be annotated with a caption, using the
/// same convention — a single line of `*italic text*` immediately
/// following, with no blank line in between.
///
/// If `events[i]` starts the matching event pattern — `SoftBreak`,
/// `Start(Emphasis)`, one or more `Text`/`InlineHtml`/`Code` runs,
/// `End(Emphasis)`, then `End(Paragraph)` — returns the caption already
/// rendered to (escaped) HTML, plus the index of its closing
/// `End(Emphasis)` event. Requiring the paragraph to end right after the
/// emphasis run keeps this from misfiring on an italicized phrase that's
/// merely followed by more prose in the same paragraph.
pub fn caption_after(
    events: &[(pulldown_cmark::Event<'_>, std::ops::Range<usize>)],
    i: usize,
) -> Option<(String, usize)> {
    use pulldown_cmark::{Event, Tag, TagEnd};

    if !matches!(events.get(i)?.0, Event::SoftBreak) {
        return None;
    }
    if !matches!(events.get(i + 1)?.0, Event::Start(Tag::Emphasis)) {
        return None;
    }

    let mut html = String::new();
    let mut j = i + 2;
    loop {
        match &events.get(j)?.0 {
            Event::Text(t) | Event::InlineHtml(t) => {
                html.push_str(&escape_text(t));
                j += 1;
            }
            // An inline code span (e.g. `` `sk1` `` in a caption) — render
            // the same way Zola's own markdown pass would: as <code>.
            Event::Code(t) => {
                let _ = write!(html, "<code>{}</code>", escape_text(t));
                j += 1;
            }
            Event::End(TagEnd::Emphasis) => break,
            _ => return None,
        }
    }
    let emphasis_end = j;
    if matches!(events.get(j + 1)?.0, Event::End(TagEnd::Paragraph)) {
        Some((html, emphasis_end))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_text_escapes_text_special_chars() {
        // Quotes are left alone in text content — only relevant for
        // escape_attr.
        assert_eq!(
            escape_text("a \"quote\" & <tag>"),
            "a \"quote\" &amp; &lt;tag&gt;"
        );
    }

    #[test]
    fn escape_attr_escapes_all_special_chars() {
        assert_eq!(
            escape_attr("a \"quote\" & <tag>"),
            "a &quot;quote&quot; &amp; &lt;tag&gt;"
        );
    }

    #[test]
    fn caption_after_matches_caption_following_a_paragraph_of_plain_text() {
        // Mirrors what a caption looks like after any content ending a
        // paragraph — e.g. `some text\n*a caption*\n`.
        let body = "x\n*a caption*\n";
        let events: Vec<_> = pulldown_cmark::Parser::new(body)
            .into_offset_iter()
            .collect();
        // events: Start(Paragraph), Text("x"), SoftBreak, Start(Emphasis),
        // Text("a caption"), End(Emphasis), End(Paragraph)
        let softbreak_idx = 2;
        let (caption, _) = caption_after(&events, softbreak_idx).unwrap();
        assert_eq!(caption, "a caption");
    }

    #[test]
    fn caption_after_rejects_trailing_content_in_the_same_paragraph() {
        let body = "x\n*not a caption* and more\n";
        let events: Vec<_> = pulldown_cmark::Parser::new(body)
            .into_offset_iter()
            .collect();
        let softbreak_idx = 2;
        assert!(caption_after(&events, softbreak_idx).is_none());
    }
}
