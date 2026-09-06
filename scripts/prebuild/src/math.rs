use std::collections::HashMap;
use std::fmt::Write as _;

/// Build `KaTeX` `Opts` with the given macros. `display` selects display mode.
///
/// katex-rs uses `derive_builder`: `throw_on_error` / `display_mode` are
/// `&mut self` setters, while `add_macro` consumes `self` and returns
/// `Self`. The `.clone()` after the setters converts the builder to an
/// owned value so the `add_macro` chain can consume and reassign it.
fn build_opts(display: bool, macros: &HashMap<String, String>) -> Result<katex::Opts, String> {
    let mut b = katex::Opts::builder();
    b.throw_on_error(false);
    if display {
        b.display_mode(true);
    }
    let mut b = b.clone();
    for (k, v) in macros {
        b = b.add_macro(k.clone(), v.clone());
    }
    b.build().map_err(|e| format!("katex opts: {}", e))
}

/// Render math expressions in the body to `KaTeX` HTML.
///
/// Recognized delimiters: `$$...$$` (display) and `$...$` (inline), located
/// by `pulldown-cmark`'s `ENABLE_MATH` option. `\(...\)` and `\[...\]` are
/// NOT recognized — use `$...$` / `$$...$$` instead. Use `\$` for a literal
/// dollar sign in prose (`CommonMark` escape, handled by the parser).
///
/// # Invariant: raw-HTML passthrough
///
/// `KaTeX` renders to raw HTML (`<span>`, `<annotation>`, `&`-entities, `<`).
/// This function inserts that HTML directly into the markdown body, which
/// Zola later converts to HTML. It relies on Zola passing raw inline/block
/// HTML through markdown unchanged — it does not escape `<` inside HTML
/// spans. If Zola's markdown backend ever re-escaped raw HTML, math would
/// render as visible `<span ...>` text. There is no automated build guard
/// for this; a regression would be visible immediately on any math page.
///
/// # Math location
///
/// Math spans are located by `pulldown-cmark`'s `ENABLE_MATH` option, which
/// emits `Event::InlineMath` / `Event::DisplayMath` for `$...$` / `$$...$$`.
/// The parser is the single source of truth: it never emits math inside code
/// spans or blocks, handles escaped `\$`, balanced brace nesting, multi-line
/// math, and math inside blockquotes/lists. The event content is the raw
/// LaTeX source (not HTML-escaped), and the event range includes the
/// delimiters — both properties verified empirically against pulldown-cmark
/// 0.13.
///
/// # Fallback contract
///
/// On `KaTeX` render error, the original source text (including `$...$`
/// delimiters, since the range includes them) is emitted verbatim and a
/// warning is printed. A misclassification therefore degrades to literal
/// text, never garbage. The safety net is this fallback, not any heuristic.
///
/// # Caption convention
///
/// A `$$...$$` display-math block followed, with no blank line in between,
/// by a single line of `*italic text*` is treated as an equation with a
/// caption — same convention as `images::rewrite_images`, and the same
/// underlying reason: with no blank line the two are one `CommonMark`
/// paragraph, and after the equation is rewritten to a raw `KaTeX` `<div>`,
/// that starts an HTML block which would otherwise swallow the caption
/// line as literal, unparsed text. The pair renders as
/// `<figure><div class="katex-display">...<figcaption>` instead. Unlike an
/// image, an equation's rendered width isn't known at build time, so
/// (unlike `rewrite_images`) the figure isn't width-capped to it — a long
/// caption can end up wider than a narrow equation. Inline math (`$...$`)
/// is not eligible; a caption only makes sense for a block-level equation.
/// See `crate::caption::caption_after` for the exact event pattern
/// required.
pub fn render_math(body: &str, macros: &HashMap<String, String>) -> Result<String, String> {
    let opts = build_opts(false, macros)?;
    let opts_display = build_opts(true, macros)?;

    // Parse with ENABLE_MATH. The parser is the single source of truth for
    // where math is; it handles code exclusion, escapes, and nesting.
    let mut opts_p = pulldown_cmark::Options::empty();
    opts_p.insert(pulldown_cmark::Options::ENABLE_MATH);
    let events: Vec<_> = pulldown_cmark::Parser::new_ext(body, opts_p)
        .into_offset_iter()
        .collect();

    // Collect math spans. The event content is the raw LaTeX (not
    // HTML-escaped); the range includes the delimiters.
    let mut math_spans: Vec<MathSpan> = Vec::new();
    let mut i = 0;
    while i < events.len() {
        match &events[i].0 {
            pulldown_cmark::Event::InlineMath(s) => {
                math_spans.push(MathSpan {
                    start: events[i].1.start,
                    end: events[i].1.end,
                    math: s.clone().into_string(),
                    is_display: false,
                    caption: None,
                });
            }
            pulldown_cmark::Event::DisplayMath(s) => {
                let mut span = MathSpan {
                    start: events[i].1.start,
                    end: events[i].1.end,
                    math: s.clone().into_string(),
                    is_display: true,
                    caption: None,
                };
                if let Some((caption, emphasis_end)) = crate::caption::caption_after(&events, i + 1)
                {
                    span.caption = Some(caption);
                    span.end = events[emphasis_end].1.end;
                }
                math_spans.push(span);
            }
            _ => {}
        }
        i += 1;
    }

    // pulldown-cmark emits events in document order, so math_spans should
    // already be sorted by start. Sort explicitly as cheap insurance.
    math_spans.sort_by_key(|s| s.start);

    // Walk the original string, replacing math spans with KaTeX HTML.
    // Everything outside math spans is emitted verbatim — the markdown
    // passes through to Zola unchanged.
    let mut result = String::with_capacity(body.len());
    let mut cursor = 0;
    for span in &math_spans {
        debug_assert!(
            span.start >= cursor,
            "math spans out of order or overlapping"
        );
        result.push_str(&body[cursor..span.start]);
        let opts_ref = if span.is_display {
            &opts_display
        } else {
            &opts
        };
        match katex::render_with_opts(&span.math, opts_ref) {
            Ok(html) => {
                // KaTeX HTML for multi-line math (e.g. \begin{align*})
                // contains literal newlines inside the <annotation> tag.
                // When this raw HTML is inserted into markdown, Zola's
                // parser breaks the HTML at newlines (especially inside
                // list items/blockquotes). Strip newlines to keep the
                // HTML on a single line — safe because newlines in HTML
                // are not semantically significant.
                let html = html.replace('\n', "");
                // Zola 0.23+ renders every content file as a Tera template,
                // so a literal `{{`, `{%` or `{#` breaks the build. The
                // `<annotation>` element KaTeX emits embeds the original
                // TeX source verbatim, and `\newcommand{...}[1]{...#1...}`
                // macro bodies routinely produce a `{#1` sequence — Tera's
                // comment-open sigil. Wrap the rendered HTML in `{% raw %}`
                // so its contents are never parsed as Tera.
                if span.is_display {
                    let inner = format!(
                        "{{% raw %}}<div class=\"katex-display\">{html}</div>{{% endraw %}}"
                    );
                    match &span.caption {
                        // Already escaped by caption_after.
                        Some(caption) => {
                            let _ = write!(
                                result,
                                "<figure>{inner}<figcaption>{caption}</figcaption></figure>"
                            );
                        }
                        None => result.push_str(&inner),
                    }
                } else {
                    let _ = write!(result, "{{% raw %}}{}{{% endraw %}}", html);
                }
            }
            Err(e) => {
                eprintln!(
                    "  WARN: {} math render failed: {} | math: {}",
                    if span.is_display { "display" } else { "inline" },
                    e,
                    span.math
                );
                // Fallback: emit the original text verbatim (including
                // delimiters and, if present, the caption line — the span
                // was extended to cover it) so the source survives for
                // diagnosis. Also wrapped in `{% raw %}` since the raw TeX
                // source can itself contain `{{`/`{%`/`{#`.
                let _ = write!(
                    result,
                    "{{% raw %}}{}{{% endraw %}}",
                    &body[span.start..span.end]
                );
            }
        }
        cursor = span.end;
    }
    result.push_str(&body[cursor..]);
    Ok(result)
}

struct MathSpan {
    start: usize,
    end: usize,
    math: String,
    is_display: bool,
    caption: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- render_math: code exclusion (end-to-end) ----
    //
    // katex HTML isn't byte-stable across environments, so we assert the
    // structural contract: math inside code stays verbatim (the `$`
    // survives), math in prose is replaced (the `$...$` delimiters vanish).

    fn no_macros() -> HashMap<String, String> {
        HashMap::new()
    }

    #[test]
    fn render_skips_inline_code() {
        let body = "prose $a$ and `$b$` code";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(out.contains("$b$"), "code math must be verbatim: {}", out);
        assert!(!out.contains("$a$"), "prose math must render: {}", out);
    }

    #[test]
    fn render_skips_backtick_fence() {
        let body = "before $a$\n```\n$x = 1$\n```\nafter $b$\n";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(
            out.contains("$x = 1$"),
            "fenced code must be verbatim: {}",
            out
        );
        assert!(
            !out.contains("$a$") && !out.contains("$b$"),
            "prose must render: {}",
            out
        );
    }

    #[test]
    fn render_skips_tilde_fence() {
        let body = "before $a$\n~~~\n$y$\n~~~\nafter $b$\n";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(out.contains("$y$"), "tilde code must be verbatim: {}", out);
        assert!(
            !out.contains("$a$") && !out.contains("$b$"),
            "prose must render: {}",
            out
        );
    }

    #[test]
    fn render_skips_indented_block() {
        let body = "para $a$\n\n    indented $c$\n\nafter $b$\n";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(
            out.contains("$c$"),
            "indented code must be verbatim: {}",
            out
        );
        assert!(
            !out.contains("$a$") && !out.contains("$b$"),
            "prose must render: {}",
            out
        );
    }

    #[test]
    fn render_renders_in_blockquote_and_list() {
        let body = "> note $a$ here\n\n- item $b$\n";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(!out.contains("$a$"), "blockquote math must render: {}", out);
        assert!(!out.contains("$b$"), "list math must render: {}", out);
    }

    #[test]
    fn render_single_letter_inline() {
        let body = "let $x$ be a value\n";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(
            !out.contains("$x$"),
            "single-letter math must render: {}",
            out
        );
    }

    #[test]
    fn render_display_block() {
        let body = "text\n$$\na + b\n$$\nmore\n";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(!out.contains("$$"), "display math must render: {}", out);
    }

    // ---- render_math: new behavior tests ----

    #[test]
    fn render_bracket_display_not_math() {
        // C++ standard references like \[defns.well.formed\] must NOT be
        // rendered as math. pulldown-cmark does not treat \[...\] as math.
        let body = "see \\[defns.well.formed\\] in the standard\n";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(
            !out.contains("katex"),
            "C++ ref must not be rendered as math: {}",
            out
        );
        assert!(
            out.contains("defns.well.formed"),
            "C++ ref text must survive: {}",
            out
        );
    }

    #[test]
    fn render_paren_inline_not_math() {
        // \(...\) is no longer a recognized delimiter.
        let body = "text \\(not math\\) here\n";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(
            !out.contains("katex"),
            "\\(...\\) must not be rendered as math: {}",
            out
        );
    }

    #[test]
    fn render_multiline_inline_math() {
        // pulldown-cmark allows multi-line $...$ (old scanner broke on newline).
        let body = "text $a +\n b$ more\n";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(
            !out.contains("$a +") || !out.contains("b$"),
            "multi-line inline math must render: {}",
            out
        );
    }

    #[test]
    fn render_dollar_in_prose() {
        // \$ in prose is a CommonMark escape → literal $ in output, not math.
        let body = "price \\$5 here\n";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(!out.contains("katex"), "\\$ must not trigger math: {}", out);
        assert!(
            out.contains('$'),
            "literal $ must survive in output: {}",
            out
        );
    }

    #[test]
    fn render_escaped_dollar_in_math() {
        // \$ inside $...$ math is handled by KaTeX (e.g. \xleftarrow{\$}).
        let body = "math $\\xleftarrow{\\$}$ here\n";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(
            !out.contains("$\\xleftarrow{\\$}$"),
            "math with escaped dollar must render: {}",
            out
        );
    }

    #[test]
    fn render_whitespace_edge_math() {
        // pulldown-cmark rejects inline math that starts or ends with whitespace.
        let body = "text $ x$ and $y $ here\n";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(
            out.contains("$ x$"),
            "leading-space math must stay literal: {}",
            out
        );
        assert!(
            out.contains("$y $"),
            "trailing-space math must stay literal: {}",
            out
        );
    }

    #[test]
    fn render_empty_display_math() {
        // Empty display math $$$$ should render without error.
        let body = "text\n$$$$\nmore\n";
        let out = render_math(body, &no_macros()).unwrap();
        // Should not panic or error — just verify it produces output.
        assert!(!out.is_empty(), "empty display math must not crash");
    }

    #[test]
    fn render_empty_inline_not_math() {
        // $$ (empty inline) is not treated as math by pulldown-cmark.
        let body = "text $$ not math\n";
        let out = render_math(body, &no_macros()).unwrap();
        // $$ might be parsed as display math with empty content, or as
        // literal text. Either way, no panic.
        assert!(!out.is_empty(), "empty inline must not crash");
    }

    #[test]
    fn render_multiline_display_no_newlines_in_html() {
        // Multi-line $$...$$ math (e.g. \begin{align*}) must produce
        // single-line HTML — newlines in the KaTeX output break Zola's
        // markdown parser, especially inside list items/blockquotes.
        let body = "text\n$$\n\\begin{align*}\na &= b \\\\\nc &= d\n\\end{align*}\n$$\nmore\n";
        let out = render_math(body, &no_macros()).unwrap();
        // The katex-display div must not contain newlines.
        if let Some(start) = out.find("<div class=\"katex-display\">") {
            let chunk = &out[start..];
            let end = chunk.find("</div>").unwrap_or(chunk.len());
            let div_content = &chunk[..end];
            assert!(
                !div_content.contains('\n'),
                "KaTeX HTML must be single-line (no newlines): found newline in div"
            );
        }
    }

    #[test]
    fn render_display_math_no_blank_line_inside() {
        // pulldown-cmark does not recognize $$...$$ as math when it contains
        // a blank line. This is a CommonMark math spec limitation. The test
        // documents the behavior so a future change is noticed.
        let body = "text\n$$\na = b\n\nc = d\n$$\nmore\n";
        let out = render_math(body, &no_macros()).unwrap();
        // With a blank line, pulldown-cmark does NOT parse this as math.
        // The $$ delimiters survive as literal text.
        assert!(
            out.contains("$$"),
            "blank line in $$...$$ prevents math parsing: {}",
            out
        );
    }

    // ---- render_math: caption convention ----

    #[test]
    fn render_wraps_display_math_caption_in_figure() {
        // No blank line between $$...$$ and the italic line — the caption
        // convention.
        let body = "$$\na + b\n$$\n*a caption*\n";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(
            out.contains("<figure>{% raw %}<div class=\"katex-display\">")
                && out.contains("<figcaption>a caption</figcaption></figure>"),
            "{}",
            out
        );
        // The caption markdown must be fully consumed, not left dangling.
        assert!(!out.contains('*'), "{}", out);
    }

    #[test]
    fn render_no_figure_around_display_math_without_caption() {
        let body = "$$\na + b\n$$\n\nmore prose\n";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(!out.contains("<figure>"), "{}", out);
    }

    #[test]
    fn render_ignores_emphasis_in_a_separate_paragraph_after_display_math() {
        // A blank line means this is a new paragraph, not a caption.
        let body = "$$\na + b\n$$\n\n*not a caption*\n";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(!out.contains("<figure>"), "{}", out);
        assert!(out.contains("*not a caption*"), "{}", out);
    }

    #[test]
    fn render_ignores_inline_math_caption_attempt() {
        // The caption convention only applies to display ($$) math — an
        // inline $...$ span followed by an italic line is not a caption
        // (it isn't even the same paragraph shape: inline math doesn't end
        // a paragraph the way a display block does).
        let body = "prose $a$\n*not a caption*\n";
        let out = render_math(body, &no_macros()).unwrap();
        assert!(!out.contains("<figure>"), "{}", out);
    }
}
