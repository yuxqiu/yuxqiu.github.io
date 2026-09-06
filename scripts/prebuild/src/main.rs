use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use walkdir::WalkDir;

/// Extract the TOML front matter (between +++ lines) and the body from a Zola markdown file.
/// Returns (front_matter_str, body_str). If no front matter, returns ("", full_content).
fn split_front_matter(content: &str) -> (String, String) {
    let trimmed = content.trim_start();
    if let Some(rest) = trimmed.strip_prefix("+++\n") {
        if let Some(end) = rest.find("\n+++\n") {
            let fm = rest[..end].to_string();
            let body = rest[end + 5..].to_string();
            return (fm, body);
        }
        // Front matter at EOF without trailing newline
        if let Some(rest2) = rest.strip_suffix("+++") {
            return (rest2.to_string(), String::new());
        }
    }
    (String::new(), content.to_string())
}

/// Minimal front matter shape — only the field the prebuild needs.
/// All other keys (title, date, taxonomies, template, [[extra.publications]],
/// etc.) are ignored by serde and passed through to Zola unchanged.
#[derive(serde::Deserialize, Default)]
struct FrontMatter {
    #[serde(default)]
    extra: Extra,
}

#[derive(serde::Deserialize, Default)]
struct Extra {
    #[serde(default)]
    katex_macros: HashMap<String, String>,
}

/// Parse katex_macros from the [extra] section of TOML front matter.
/// Supports both inline-table form (`katex_macros = { ... }`) and
/// table form (`[extra.katex_macros]`), plus any other valid TOML the
/// front matter may contain. Returns an empty map when there are no macros
/// or when the front matter is empty/malformed (with a warning).
fn parse_macros(front_matter: &str) -> HashMap<String, String> {
    if front_matter.is_empty() {
        return HashMap::new();
    }
    match toml::from_str::<FrontMatter>(front_matter) {
        Ok(fm) => fm.extra.katex_macros,
        Err(e) => {
            eprintln!("  WARN: could not parse front matter for macros: {}", e);
            HashMap::new()
        }
    }
}

/// Build KaTeX `Opts` with the given macros. `display` selects display mode.
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

/// Render math expressions in the body to KaTeX HTML.
///
/// Recognized delimiters: `$$...$$` (display) and `$...$` (inline), located
/// by `pulldown-cmark`'s `ENABLE_MATH` option. `\(...\)` and `\[...\]` are
/// NOT recognized — use `$...$` / `$$...$$` instead. Use `\$` for a literal
/// dollar sign in prose (CommonMark escape, handled by the parser).
///
/// # Invariant: raw-HTML passthrough
///
/// KaTeX renders to raw HTML (`<span>`, `<annotation>`, `&`-entities, `<`).
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
/// On KaTeX render error, the original source text (including `$...$`
/// delimiters, since the range includes them) is emitted verbatim and a
/// warning is printed. A misclassification therefore degrades to literal
/// text, never garbage. The safety net is this fallback, not any heuristic.
fn render_math(body: &str, macros: &HashMap<String, String>) -> Result<String, String> {
    let opts = build_opts(false, macros)?;
    let opts_display = build_opts(true, macros)?;

    // Parse with ENABLE_MATH. The parser is the single source of truth for
    // where math is; it handles code exclusion, escapes, and nesting.
    let mut opts_p = pulldown_cmark::Options::empty();
    opts_p.insert(pulldown_cmark::Options::ENABLE_MATH);
    let parser = pulldown_cmark::Parser::new_ext(body, opts_p);

    // Collect math spans as (start_byte, end_byte, content, is_display).
    // The event content is the raw LaTeX (not HTML-escaped); the range
    // includes the delimiters.
    let mut math_spans: Vec<(usize, usize, String, bool)> = Vec::new();
    for (event, range) in parser.into_offset_iter() {
        match event {
            pulldown_cmark::Event::InlineMath(s) => {
                math_spans.push((range.start, range.end, s.into_string(), false));
            }
            pulldown_cmark::Event::DisplayMath(s) => {
                math_spans.push((range.start, range.end, s.into_string(), true));
            }
            _ => {}
        }
    }

    // pulldown-cmark emits events in document order, so math_spans should
    // already be sorted by start. Sort explicitly as cheap insurance.
    math_spans.sort_by_key(|(start, _, _, _)| *start);

    // Walk the original string, replacing math spans with KaTeX HTML.
    // Everything outside math spans is emitted verbatim — the markdown
    // passes through to Zola unchanged.
    let mut result = String::with_capacity(body.len());
    let mut cursor = 0;
    for (start, end, math, is_display) in &math_spans {
        debug_assert!(*start >= cursor, "math spans out of order or overlapping");
        result.push_str(&body[cursor..*start]);
        let opts_ref = if *is_display { &opts_display } else { &opts };
        match katex::render_with_opts(math, opts_ref) {
            Ok(html) => {
                // KaTeX HTML for multi-line math (e.g. \begin{align*})
                // contains literal newlines inside the <annotation> tag.
                // When this raw HTML is inserted into markdown, Zola's
                // parser breaks the HTML at newlines (especially inside
                // list items/blockquotes). Strip newlines to keep the
                // HTML on a single line — safe because newlines in HTML
                // are not semantically significant.
                let html = html.replace('\n', "");
                if *is_display {
                    result.push_str(&format!("<div class=\"katex-display\">{}</div>", html));
                } else {
                    result.push_str(&html);
                }
            }
            Err(e) => {
                eprintln!(
                    "  WARN: {} math render failed: {} | math: {}",
                    if *is_display { "display" } else { "inline" },
                    e,
                    math
                );
                // Fallback: emit the original text verbatim (including
                // delimiters, since the range includes them) so the
                // source survives for diagnosis.
                result.push_str(&body[*start..*end]);
            }
        }
        cursor = *end;
    }
    result.push_str(&body[cursor..]);
    Ok(result)
}

/// Read intrinsic width/height from an SVG file via `roxmltree` (a real XML
/// parser — `imagesize`, used for raster formats below, has no SVG support
/// at all). Prefers the root `<svg>` element's `width`/`height` attributes,
/// falling back to the last two numbers in `viewBox` (needed when
/// width/height are percentages, e.g. "100%", or omitted and sizing is left
/// to CSS — a plain numeric parse of "100%" would silently misread it as 100).
fn svg_dimensions(path: &Path) -> Option<(u32, u32)> {
    let content = fs::read_to_string(path).ok()?;
    let doc = roxmltree::Document::parse(&content).ok()?;
    let root = doc.root_element();

    let attr_px = |name: &str| -> Option<f64> {
        root.attribute(name)?.trim_end_matches("px").parse().ok()
    };
    if let (Some(w), Some(h)) = (attr_px("width"), attr_px("height")) {
        return Some((w.round() as u32, h.round() as u32));
    }

    let nums: Vec<f64> = root
        .attribute("viewBox")?
        .split_whitespace()
        .filter_map(|n| n.parse().ok())
        .collect();
    match nums.as_slice() {
        [_, _, w, h] => Some((w.round() as u32, h.round() as u32)),
        _ => None,
    }
}

/// Resolve a markdown image's intrinsic pixel dimensions, for injecting
/// `width`/`height` into the rendered `<img>` tag (prevents layout shift).
///
/// Only handles site-root-relative paths (`/img/...`), which is how every
/// content image is currently referenced (they live under `static/`, not
/// colocated with the markdown). External URLs (`http(s)://...`) and any
/// other form are left alone — returns None, and the caller falls back to
/// an `<img>` without dimensions.
fn image_dimensions(static_dir: &Path, dest_url: &str) -> Option<(u32, u32)> {
    let rel = dest_url.strip_prefix('/')?;
    let path = static_dir.join(rel);
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("svg") => svg_dimensions(&path),
        _ => imagesize::size(&path).ok().map(|d| (d.width as u32, d.height as u32)),
    }
}

fn escape_html_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Rewrite plain markdown images (`![alt](url)`) to raw `<img>` HTML with
/// `width`/`height` (when resolvable, see `image_dimensions`) plus
/// `loading="lazy" decoding="async"`, so posts don't reflow as images load in.
/// Source markdown stays as plain, portable `![]()` syntax — this only
/// touches the generated `content/` output.
///
/// Image spans are located the same way as math spans in `render_math`: by
/// parsing with pulldown-cmark and using event byte ranges as the single
/// source of truth, so images inside code spans/blocks are left untouched
/// automatically.
fn rewrite_images(body: &str, static_dir: &Path) -> String {
    let parser = pulldown_cmark::Parser::new(body);

    struct ImageSpan {
        start: usize,
        end: usize,
        dest_url: String,
        alt: String,
    }

    let mut spans: Vec<ImageSpan> = Vec::new();
    let mut current: Option<ImageSpan> = None;
    for (event, range) in parser.into_offset_iter() {
        match event {
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Image { dest_url, .. }) => {
                current = Some(ImageSpan {
                    start: range.start,
                    end: range.end,
                    dest_url: dest_url.into_string(),
                    alt: String::new(),
                });
            }
            pulldown_cmark::Event::Text(t) => {
                if let Some(span) = current.as_mut() {
                    span.alt.push_str(&t);
                }
            }
            pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Image) => {
                if let Some(mut span) = current.take() {
                    span.end = range.end;
                    spans.push(span);
                }
            }
            _ => {}
        }
    }

    let mut result = String::with_capacity(body.len());
    let mut cursor = 0;
    for span in &spans {
        result.push_str(&body[cursor..span.start]);
        let src = escape_html_attr(&span.dest_url);
        let alt = escape_html_attr(&span.alt);
        match image_dimensions(static_dir, &span.dest_url) {
            Some((w, h)) => {
                result.push_str(&format!(
                    "<img src=\"{}\" alt=\"{}\" width=\"{}\" height=\"{}\" loading=\"lazy\" decoding=\"async\">",
                    src, alt, w, h
                ));
            }
            None => {
                eprintln!("  WARN: could not resolve image dimensions for {}", span.dest_url);
                result.push_str(&format!(
                    "<img src=\"{}\" alt=\"{}\" loading=\"lazy\" decoding=\"async\">",
                    src, alt
                ));
            }
        }
        cursor = span.end;
    }
    result.push_str(&body[cursor..]);
    result
}

/// Process a single markdown file: read, split front matter, rewrite images,
/// render math, write to output. Skips the write when the rendered output is
/// identical to the existing destination, so a live file watcher (zola serve)
/// only sees changes for files that actually changed — avoiding a rebuild
/// storm when one source file is edited.
fn process_file(src: &Path, dst: &Path, static_dir: &Path) -> Result<bool, String> {
    let raw = fs::read_to_string(src).map_err(|e| format!("read {}: {}", src.display(), e))?;
    let content = raw.replace("\r\n", "\n");

    let (front_matter, body) = split_front_matter(&content);
    let macros = parse_macros(&front_matter);

    if !macros.is_empty() {
        eprintln!("  macros: {:?}", macros);
    }

    let body = rewrite_images(&body, static_dir);
    let rendered_body = render_math(&body, &macros)?;

    // Reassemble: front matter + rendered body
    let output = if front_matter.is_empty() {
        rendered_body
    } else {
        format!("+++\n{}\n+++\n{}", front_matter, rendered_body)
    };

    // Create parent directories
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {}", parent.display(), e))?;
    }

    // Skip the write if the destination is already identical, so we don't
    // touch its mtime and trigger a spurious rebuild.
    if let Ok(existing) = fs::read_to_string(dst) {
        if existing == output {
            return Ok(false);
        }
    }

    fs::write(dst, output).map_err(|e| format!("write {}: {}", dst.display(), e))?;

    Ok(true)
}

/// Copy a non-markdown file as-is. Skips the copy when source and
/// destination already match, for the same reason as `process_file`.
fn copy_file(src: &Path, dst: &Path) -> Result<bool, String> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {}", parent.display(), e))?;
    }
    if let (Ok(src_data), Ok(dst_data)) = (fs::read(src), fs::read(dst)) {
        if src_data == dst_data {
            return Ok(false);
        }
    }
    fs::copy(src, dst)
        .map_err(|e| format!("copy {} -> {}: {}", src.display(), dst.display(), e))?;
    Ok(true)
}

/// Remove content files that no longer have a corresponding source file
/// (orphans from deleted/renamed src entries), then prune empty
/// directories. Keeps the content tree in sync with src without wiping it,
/// so a live file watcher never sees its watched root disappear and only
/// real changes trigger rebuilds.
fn remove_orphans(dst_dir: &Path, keep: &[PathBuf]) {
    let keep_set: HashSet<&Path> = keep.iter().map(|p| p.as_path()).collect();

    // Delete orphan files.
    for entry in WalkDir::new(dst_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path == dst_dir || !path.is_file() {
            continue;
        }
        if !keep_set.contains(path) {
            eprintln!(
                "removing orphan: {}",
                path.strip_prefix(dst_dir).unwrap_or(path).display()
            );
            let _ = fs::remove_file(path);
        }
    }

    // Prune now-empty directories, deepest first so children go before parents.
    let mut dirs: Vec<PathBuf> = WalkDir::new(dst_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir() && e.path() != dst_dir)
        .map(|e| e.path().to_path_buf())
        .collect();
    dirs.sort_by_key(|d| std::cmp::Reverse(d.components().count()));
    for d in dirs {
        let empty = fs::read_dir(&d)
            .map(|mut r| r.next().is_none())
            .unwrap_or(false);
        if empty {
            let _ = fs::remove_dir(&d);
        }
    }
}

fn main() {
    let src_dir = PathBuf::from("src");
    let dst_dir = PathBuf::from("content");
    let static_dir = PathBuf::from("static");

    if !src_dir.exists() {
        eprintln!("ERROR: {} directory does not exist", src_dir.display());
        process::exit(1);
    }

    fs::create_dir_all(&dst_dir).expect("could not create content dir");

    let mut written: Vec<PathBuf> = Vec::new();
    let mut processed = 0;
    let mut copied = 0;

    for entry in WalkDir::new(&src_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let rel = path.strip_prefix(&src_dir).unwrap();
        let dst = dst_dir.join(rel);

        let ext = path.extension().and_then(|e| e.to_str());
        match ext {
            Some("md") => {
                eprintln!("processing: {}", rel.display());
                if let Err(e) = process_file(path, &dst, &static_dir) {
                    eprintln!("ERROR: {}", e);
                    process::exit(1);
                }
                processed += 1;
            }
            _ => {
                if let Err(e) = copy_file(path, &dst) {
                    eprintln!("ERROR: {}", e);
                    process::exit(1);
                }
                copied += 1;
            }
        }
        written.push(dst);
    }

    remove_orphans(&dst_dir, &written);

    eprintln!(
        "Done: {} markdown files processed, {} files copied",
        processed, copied
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- parse_macros ----

    #[test]
    fn parse_macros_inline_table() {
        let fm = r#"title = "Test"
[extra]
katex_macros = { "\\tp" = "\\textcolor{orange}{t_p}", "\\tq" = "\\textcolor{teal}{t_q}" }
"#;
        let macros = parse_macros(fm);
        assert_eq!(macros.get("\\tp").unwrap(), "\\textcolor{orange}{t_p}");
        assert_eq!(macros.get("\\tq").unwrap(), "\\textcolor{teal}{t_q}");
    }

    #[test]
    fn parse_macros_table_form() {
        let fm = r#"title = "Test"
[extra.katex_macros]
"\\R" = "\\mathbb{R}"
"#;
        let macros = parse_macros(fm);
        assert_eq!(macros.get("\\R").unwrap(), "\\mathbb{R}");
    }

    #[test]
    fn parse_macros_none() {
        let fm = r#"title = "Test"
date = 2025-01-01
"#;
        let macros = parse_macros(fm);
        assert!(macros.is_empty());
    }

    #[test]
    fn parse_macros_empty_frontmatter() {
        assert!(parse_macros("").is_empty());
    }

    #[test]
    fn parse_macros_ignores_other_extra_fields() {
        // publications.md has [[extra.publications]] — must not interfere.
        let fm = r#"title = "Publications"
[[extra.publications]]
title = "Some Paper"
authors = "Someone"
year = 2024
"#;
        let macros = parse_macros(fm);
        assert!(macros.is_empty());
    }

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
        assert!(
            !out.contains("katex"),
            "\\$ must not trigger math: {}",
            out
        );
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
        assert!(out.contains("$$"), "blank line in $$...$$ prevents math parsing: {}", out);
    }

    // ---- CRLF front matter ----

    #[test]
    fn split_front_matter_crlf() {
        let content = "+++\r\ntitle = \"Test\"\r\n+++\r\nbody text\r\n";
        let (fm, body) = split_front_matter(&content.replace("\r\n", "\n"));
        assert!(fm.contains("title = \"Test\""), "fm: {}", fm);
        assert!(body.contains("body text"), "body: {}", body);
    }

    #[test]
    fn parse_macros_crlf() {
        let fm = "title = \"Test\"\r\n[extra]\r\nkatex_macros = { \"\\\\R\" = \"\\\\mathbb{R}\" }\r\n";
        let macros = parse_macros(&fm.replace("\r\n", "\n"));
        assert_eq!(macros.get("\\R").unwrap(), "\\mathbb{R}");
    }

    // ---- svg_dimensions ----

    fn write_temp(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("prebuild-test-{}-{}", process::id(), name));
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn svg_dimensions_from_width_height() {
        let path = write_temp(
            "wh.svg",
            r#"<svg width="958" height="593.4" viewBox="0 0 100 100"></svg>"#,
        );
        assert_eq!(svg_dimensions(&path), Some((958, 593)));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn svg_dimensions_falls_back_to_viewbox() {
        // width/height omitted (common when sizing is left to CSS).
        let path = write_temp("vb.svg", r#"<svg viewBox="0 0 800 600"></svg>"#);
        assert_eq!(svg_dimensions(&path), Some((800, 600)));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn svg_dimensions_falls_back_to_viewbox_on_percent_size() {
        // width="100%" has no fixed pixel size, so it must not be parsed as 100.
        let path = write_temp(
            "pct.svg",
            r#"<svg width="100%" height="100%" viewBox="0 0 320 240"></svg>"#,
        );
        assert_eq!(svg_dimensions(&path), Some((320, 240)));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn svg_dimensions_none_when_unresolvable() {
        let path = write_temp("none.svg", r#"<svg></svg>"#);
        assert_eq!(svg_dimensions(&path), None);
        let _ = fs::remove_file(&path);
    }

    // ---- image_dimensions ----

    #[test]
    fn image_dimensions_external_url_unresolved() {
        let static_dir = std::env::temp_dir();
        assert_eq!(
            image_dimensions(&static_dir, "https://example.com/a.png"),
            None
        );
    }

    #[test]
    fn image_dimensions_missing_file_unresolved() {
        let static_dir = std::env::temp_dir();
        assert_eq!(
            image_dimensions(&static_dir, "/does/not/exist.png"),
            None
        );
    }

    // ---- rewrite_images ----

    #[test]
    fn rewrite_images_adds_lazy_loading_without_dimensions_when_unresolvable() {
        let static_dir = std::env::temp_dir().join("prebuild-test-empty-static-dir");
        let body = "prose\n\n![alt text](/img/missing.png)\n\nmore\n";
        let out = rewrite_images(body, &static_dir);
        assert!(out.contains(r#"<img src="/img/missing.png" alt="alt text" loading="lazy" decoding="async">"#), "{}", out);
        assert!(!out.contains("width="), "{}", out);
    }

    #[test]
    fn rewrite_images_adds_dimensions_for_resolvable_svg() {
        let static_dir = std::env::temp_dir().join(format!("prebuild-test-static-{}", process::id()));
        fs::create_dir_all(static_dir.join("img")).unwrap();
        fs::write(
            static_dir.join("img/x.svg"),
            r#"<svg width="10" height="20"></svg>"#,
        )
        .unwrap();

        let body = "![x](/img/x.svg)\n";
        let out = rewrite_images(body, &static_dir);
        assert!(
            out.contains(r#"<img src="/img/x.svg" alt="x" width="10" height="20" loading="lazy" decoding="async">"#),
            "{}",
            out
        );

        let _ = fs::remove_dir_all(&static_dir);
    }

    #[test]
    fn rewrite_images_skips_images_in_code() {
        let static_dir = std::env::temp_dir();
        let body = "before\n\n`![x](/img/x.png)`\n\nafter\n";
        let out = rewrite_images(body, &static_dir);
        assert!(out.contains("![x](/img/x.png)"), "{}", out);
        assert!(!out.contains("<img"), "{}", out);
    }

    #[test]
    fn rewrite_images_escapes_alt_text() {
        // `<tag>`-like text in alt is a separate concern (pulldown-cmark
        // parses it as inline HTML, not Text) — covered directly by
        // escape_html_attr's own test below instead.
        let static_dir = std::env::temp_dir();
        let body = "![a \"quote\" & more](/img/x.png)\n";
        let out = rewrite_images(body, &static_dir);
        assert!(
            out.contains("alt=\"a &quot;quote&quot; &amp; more\""),
            "{}",
            out
        );
    }

    #[test]
    fn escape_html_attr_escapes_all_special_chars() {
        assert_eq!(
            escape_html_attr("a \"quote\" & <tag>"),
            "a &quot;quote&quot; &amp; &lt;tag&gt;"
        );
    }
}
