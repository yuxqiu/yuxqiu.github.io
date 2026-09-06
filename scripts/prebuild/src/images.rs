use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use walkdir::WalkDir;

use crate::caption::{caption_after, escape_attr};

/// Read intrinsic width/height from an SVG file. Delegates the actual size
/// resolution (unit conversion, percentage-of-viewBox math, the
/// width/height-vs-viewBox precedence rules) to `usvg` — a real
/// spec-compliant SVG parser — rather than hand-rolling unit stripping and
/// float parsing, which doesn't handle every unit (`cm`, `pt`, `%`, ...)
/// `usvg` does.
///
/// One thing `usvg` won't do for us: per the SVG spec it falls back to a
/// default 100x100 viewport when there's no width/height/viewBox at all
/// (needed to have *something* to render into), but that's a rendering
/// guess, not a real intrinsic size — for our purposes (setting `width`/
/// `height` on `<img>` to prevent layout shift) a guessed 100x100 would be
/// actively misleading, so that case is still explicitly checked for and
/// treated as unresolvable, same as before.
///
/// Returns usvg's own native `f32` precision, unrounded — rounding to the
/// integer pixels an `<img>` attribute needs is `image_dimensions`'s job,
/// not this function's, so both this and the raster path (`imagesize`,
/// which already hands back plain integers) return their format's native
/// numeric type and are normalized to `u32` in one place.
fn svg_dimensions(path: &Path) -> Option<(f32, f32)> {
    let content = fs::read_to_string(path).ok()?;

    let doc = roxmltree::Document::parse(&content).ok()?;
    let root = doc.root_element();
    if root.attribute("width").is_none()
        && root.attribute("height").is_none()
        && root.attribute("viewBox").is_none()
    {
        return None;
    }

    let tree = usvg::Tree::from_str(&content, &usvg::Options::default()).ok()?;
    let size = tree.size();
    Some((size.width(), size.height()))
}

/// Outcome of resolving a markdown image's intrinsic pixel dimensions.
/// Kept as three distinct cases (rather than collapsing to `Option`) so the
/// caller can react differently: a typo'd/deleted local file is a build
/// error, an external URL or unsupported format is not.
#[derive(Debug, PartialEq)]
enum ImageDims {
    /// Dimensions were resolved — inject `width`/`height` (rounded to the
    /// integers HTML requires there — SVG's own size can be fractional,
    /// a raster format's pixel dimensions from `imagesize` already aren't).
    Resolved(u32, u32),
    /// Not a site-root-relative path (external URL, etc.) — nothing to
    /// check; every current content image is root-relative
    /// (`/img/blog/...`), but a future external image is expected to hit this.
    Skipped,
    /// A site-root-relative path was given, but no file exists there. This
    /// is (almost always) a typo or a deleted/moved asset — a broken image
    /// link, not something to silently ship.
    Missing,
    /// The file exists but its dimensions couldn't be read (unsupported or
    /// corrupt format).
    Unrecognized,
}

/// Resolve a markdown image's intrinsic pixel dimensions, for injecting
/// `width`/`height` into the rendered `<img>` tag (prevents layout shift).
// Image dimensions in pixels are always small non-negative numbers, nowhere
// near u32::MAX or negative — the truncation/sign-loss these casts warn
// about can't happen in practice for a real image file.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn image_dimensions(static_dir: &Path, dest_url: &str) -> ImageDims {
    let Some(rel) = dest_url.strip_prefix('/') else {
        return ImageDims::Skipped;
    };
    let path = static_dir.join(rel);
    if !path.is_file() {
        return ImageDims::Missing;
    }
    let dims = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("svg") => {
            svg_dimensions(&path).map(|(w, h)| (w.round() as u32, h.round() as u32))
        }
        _ => imagesize::size(&path)
            .ok()
            .map(|d| (d.width as u32, d.height as u32)),
    };
    match dims {
        Some((w, h)) => ImageDims::Resolved(w, h),
        None => ImageDims::Unrecognized,
    }
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
///
/// Every site-root-relative path encountered (resolved or not) is recorded
/// into `referenced`, without the leading `/` — later used by
/// `find_unused_images` to flag `static/img/` files no post links to.
///
/// Returns `Err` if a referenced local image file doesn't exist — almost
/// always a typo'd or deleted/moved asset, so it fails the build the same
/// way a bad KaTeX-opts config would (see `render_math`), rather than
/// silently shipping a broken `<img>`. Missing dimensions for a file that
/// *does* exist (unsupported/corrupt format) is not fatal — only a warning.
///
/// # Caption convention
///
/// `![alt](url)` followed, with no blank line in between, by a single line
/// of `*italic text*` is treated as an image with a caption: the pair
/// renders as `<figure><img>...<figcaption>` instead of a bare `<img>`, and
/// the caption line is consumed (not left behind to be re-parsed). This
/// matters because with no blank line the two markdown lines are one
/// `CommonMark` paragraph, and after the image is rewritten to a raw `<img>`
/// tag, that tag starts an HTML block (`CommonMark` type 7) which swallows
/// any immediately-following line as literal, unparsed text — the emphasis
/// markers would render as literal asterisks, not become a caption. See
/// `crate::caption::caption_after` for the exact event pattern required.
struct ImageSpan {
    start: usize,
    end: usize,
    dest_url: String,
    alt: String,
    caption: Option<String>,
}

pub fn rewrite_images(
    body: &str,
    static_dir: &Path,
    referenced: &mut HashSet<String>,
) -> Result<String, String> {
    let events: Vec<_> = pulldown_cmark::Parser::new(body)
        .into_offset_iter()
        .collect();

    let mut spans: Vec<ImageSpan> = Vec::new();
    let mut i = 0;
    while i < events.len() {
        let (pulldown_cmark::Event::Start(pulldown_cmark::Tag::Image { dest_url, .. }), range) =
            &events[i]
        else {
            i += 1;
            continue;
        };
        let mut span = ImageSpan {
            start: range.start,
            end: range.end,
            dest_url: dest_url.clone().into_string(),
            alt: String::new(),
            caption: None,
        };

        // Collect alt text (Text, or InlineHtml for e.g. `vector<int>` —
        // common on a C++ blog, and not real HTML since alt attributes
        // don't render markup) up to the matching End(Image).
        i += 1;
        loop {
            match &events[i].0 {
                pulldown_cmark::Event::Text(t) | pulldown_cmark::Event::InlineHtml(t) => {
                    span.alt.push_str(t);
                    i += 1;
                }
                pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Image) => {
                    span.end = events[i].1.end;
                    i += 1;
                    break;
                }
                _ => i += 1,
            }
        }

        if let Some((caption, emphasis_end)) = caption_after(&events, i) {
            span.caption = Some(caption);
            span.end = events[emphasis_end].1.end;
        }

        spans.push(span);
    }

    let mut result = String::with_capacity(body.len());
    let mut cursor = 0;
    for span in &spans {
        result.push_str(&body[cursor..span.start]);
        let src = escape_attr(&span.dest_url);
        let alt = escape_attr(&span.alt);

        if span.alt.is_empty() {
            eprintln!("  WARN: image has no alt text: {}", span.dest_url);
        }
        if let Some(rel) = span.dest_url.strip_prefix('/') {
            referenced.insert(rel.to_string());
        }

        let (img_tag, width) = match image_dimensions(static_dir, &span.dest_url) {
            ImageDims::Resolved(w, h) => (
                format!(
                    "<img src=\"{src}\" alt=\"{alt}\" width=\"{w}\" height=\"{h}\" loading=\"lazy\" decoding=\"async\">"
                ),
                Some(w),
            ),
            ImageDims::Skipped => (
                format!("<img src=\"{src}\" alt=\"{alt}\" loading=\"lazy\" decoding=\"async\">"),
                None,
            ),
            ImageDims::Unrecognized => {
                eprintln!(
                    "  WARN: could not determine dimensions for {} (unsupported or corrupt image)",
                    span.dest_url
                );
                (
                    format!("<img src=\"{src}\" alt=\"{alt}\" loading=\"lazy\" decoding=\"async\">"),
                    None,
                )
            }
            ImageDims::Missing => {
                return Err(format!(
                    "image not found: {} (no file at static{})",
                    span.dest_url, span.dest_url
                ));
            }
        };

        match &span.caption {
            // Already escaped (and any inline code spans already rendered
            // to <code>) by caption_after. The figure's max-width is
            // capped to the image's intrinsic width (when known) so a long
            // caption wraps to the image's rendered width instead of the
            // full column — without it, <figcaption> (an ordinary block
            // element) stretches to fill the figure, which by default
            // spans the whole reading column, not just the (often
            // narrower) image.
            Some(caption) => {
                let style =
                    width.map_or_else(String::new, |w| format!(" style=\"max-width: {w}px\""));
                let _ = write!(
                    result,
                    "<figure{style}>{img_tag}<figcaption>{caption}</figcaption></figure>"
                );
            }
            None => result.push_str(&img_tag),
        }
        cursor = span.end;
    }
    result.push_str(&body[cursor..]);
    Ok(result)
}

/// Files under `static/img/` that nothing references — almost always a
/// leftover asset from a deleted or renamed image reference. `referenced` is
/// the accumulated set from every `rewrite_images` call plus
/// `collect_template_image_refs` (paths without the leading `/`, e.g.
/// `img/blog/2022/10/x.png`).
pub fn find_unused_images(static_dir: &Path, referenced: &HashSet<String>) -> Vec<String> {
    let img_dir = static_dir.join("img");
    if !img_dir.is_dir() {
        return Vec::new();
    }

    let mut unused: Vec<String> = WalkDir::new(&img_dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let rel = e
                .path()
                .strip_prefix(static_dir)
                .ok()?
                .to_string_lossy()
                .replace('\\', "/");
            (!referenced.contains(&rel)).then_some(rel)
        })
        .collect();
    unused.sort();
    unused
}

/// Extract every `path="..."` argument from calls to `fn_name(...)` in
/// `content`. A plain string scan, not a Tera parser — see
/// `collect_template_image_refs` for why that's the right tradeoff here.
/// Only handles a literal string argument on one line; a call spanning
/// multiple lines, or nested parens inside the call, is not handled (not
/// needed by any current template).
fn extract_path_args(content: &str, fn_name: &str) -> Vec<String> {
    let needle = format!("{}(", fn_name);
    let mut paths = Vec::new();
    let mut search_from = 0;
    while let Some(rel_start) = content[search_from..].find(&needle) {
        let call_start = search_from + rel_start + needle.len();
        let call_end = content[call_start..]
            .find(')')
            .map_or(content.len(), |i| call_start + i);
        let call_args = &content[call_start..call_end];
        if let Some(path_start) = call_args.find("path=\"") {
            let path_start = path_start + "path=\"".len();
            if let Some(path_end) = call_args[path_start..].find('"') {
                paths.push(call_args[path_start..path_start + path_end].to_string());
            }
        }
        search_from = call_end.max(call_start + 1);
    }
    paths
}

/// Record images referenced from Tera templates — not just markdown — into
/// `referenced`, so `find_unused_images` doesn't false-positive on an image
/// used only via a template (e.g. the homepage photo, via `resize_image()`
/// in `index.html`).
///
/// This is a plain text scan of every file under `templates/`, not a Tera
/// parser. The only call shapes handled are `resize_image(path="img/...")`,
/// `get_image_metadata(path="img/...")`, `get_url(path="img/...")` with a
/// literal string path, and a hand-written `<img src="/img/...">`. There is
/// exactly one image-referencing template call in this project today
/// (`resize_image` in `index.html`) — a real Tera AST walk would be
/// disproportionate for that. The tradeoff: a path built from a Tera
/// variable or string concatenation is invisible to this scan. If that ever
/// happens, this needs revisiting (or the false positive just needs a
/// human to notice it's not actually unused).
pub fn collect_template_image_refs(templates_dir: &Path, referenced: &mut HashSet<String>) {
    for entry in WalkDir::new(templates_dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };

        for fn_name in ["resize_image", "get_image_metadata", "get_url"] {
            for p in extract_path_args(&content, fn_name) {
                if p.starts_with("img/") {
                    referenced.insert(p);
                }
            }
        }

        // Defensive: a hand-written `<img src="/img/...">` instead of a Zola
        // image function — none exist today, but cheap to also catch.
        let mut search_from = 0;
        while let Some(rel) = content[search_from..].find("src=\"/img/") {
            let start = search_from + rel + "src=\"".len();
            let Some(end) = content[start..].find('"') else {
                break;
            };
            referenced.insert(content[start + 1..start + end].to_string());
            search_from = start + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process;

    // ---- svg_dimensions ----

    fn write_temp(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("prebuild-test-{}-{}", process::id(), name));
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn svg_dimensions_from_width_height() {
        // Returned unrounded — 593.4 stays 593.4 here; rounding to 593 is
        // image_dimensions's job (see image_dimensions_rounds_svg_size).
        let path = write_temp(
            "wh.svg",
            r#"<svg width="958" height="593.4" viewBox="0 0 100 100"></svg>"#,
        );
        assert_eq!(svg_dimensions(&path), Some((958.0, 593.4)));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn svg_dimensions_falls_back_to_viewbox() {
        // width/height omitted (common when sizing is left to CSS).
        let path = write_temp("vb.svg", r#"<svg viewBox="0 0 800 600"></svg>"#);
        assert_eq!(svg_dimensions(&path), Some((800.0, 600.0)));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn svg_dimensions_falls_back_to_viewbox_on_percent_size() {
        // width="100%" has no fixed pixel size, so it must not be parsed as 100.
        let path = write_temp(
            "pct.svg",
            r#"<svg width="100%" height="100%" viewBox="0 0 320 240"></svg>"#,
        );
        assert_eq!(svg_dimensions(&path), Some((320.0, 240.0)));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn svg_dimensions_none_when_unresolvable() {
        let path = write_temp("none.svg", r"<svg></svg>");
        assert_eq!(svg_dimensions(&path), None);
        let _ = fs::remove_file(&path);
    }

    // ---- image_dimensions ----

    fn temp_static_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("prebuild-test-static-{}-{}", process::id(), name))
    }

    #[test]
    fn image_dimensions_external_url_skipped() {
        let static_dir = std::env::temp_dir();
        assert_eq!(
            image_dimensions(&static_dir, "https://example.com/a.png"),
            ImageDims::Skipped
        );
    }

    #[test]
    fn image_dimensions_missing_file() {
        let static_dir = std::env::temp_dir();
        assert_eq!(
            image_dimensions(&static_dir, "/does/not/exist.png"),
            ImageDims::Missing
        );
    }

    #[test]
    fn image_dimensions_unrecognized_format() {
        let static_dir = temp_static_dir("unrecognized");
        fs::create_dir_all(static_dir.join("img")).unwrap();
        fs::write(static_dir.join("img/x.png"), b"not actually a png").unwrap();

        assert_eq!(
            image_dimensions(&static_dir, "/img/x.png"),
            ImageDims::Unrecognized
        );
        let _ = fs::remove_dir_all(&static_dir);
    }

    #[test]
    fn image_dimensions_resolved_svg() {
        let static_dir = temp_static_dir("resolved");
        fs::create_dir_all(static_dir.join("img")).unwrap();
        fs::write(
            static_dir.join("img/x.svg"),
            r#"<svg width="10" height="20"></svg>"#,
        )
        .unwrap();

        assert_eq!(
            image_dimensions(&static_dir, "/img/x.svg"),
            ImageDims::Resolved(10, 20)
        );
        let _ = fs::remove_dir_all(&static_dir);
    }

    #[test]
    fn image_dimensions_rounds_svg_size() {
        // svg_dimensions returns 593.4 unrounded; image_dimensions is where
        // that becomes the integer an <img height> attribute needs.
        let static_dir = temp_static_dir("rounds");
        fs::create_dir_all(static_dir.join("img")).unwrap();
        fs::write(
            static_dir.join("img/x.svg"),
            r#"<svg width="958" height="593.4" viewBox="0 0 100 100"></svg>"#,
        )
        .unwrap();

        assert_eq!(
            image_dimensions(&static_dir, "/img/x.svg"),
            ImageDims::Resolved(958, 593)
        );
        let _ = fs::remove_dir_all(&static_dir);
    }

    // ---- rewrite_images ----

    #[test]
    fn rewrite_images_errors_on_missing_file() {
        let static_dir = temp_static_dir("missing-file");
        let body = "prose\n\n![alt text](/img/missing.png)\n\nmore\n";
        let err = rewrite_images(body, &static_dir, &mut HashSet::new()).unwrap_err();
        assert!(err.contains("/img/missing.png"), "{}", err);
    }

    #[test]
    fn rewrite_images_no_dimensions_without_error_for_external_url() {
        let static_dir = std::env::temp_dir();
        let body = "![alt text](https://example.com/a.png)\n";
        let out = rewrite_images(body, &static_dir, &mut HashSet::new()).unwrap();
        assert!(
            out.contains(r#"<img src="https://example.com/a.png" alt="alt text" loading="lazy" decoding="async">"#),
            "{}",
            out
        );
        assert!(!out.contains("width="), "{}", out);
    }

    #[test]
    fn rewrite_images_no_dimensions_without_error_for_unrecognized_format() {
        let static_dir = temp_static_dir("unrecognized-rewrite");
        fs::create_dir_all(static_dir.join("img")).unwrap();
        fs::write(static_dir.join("img/x.png"), b"not actually a png").unwrap();

        let body = "![x](/img/x.png)\n";
        let out = rewrite_images(body, &static_dir, &mut HashSet::new()).unwrap();
        assert!(
            out.contains(r#"<img src="/img/x.png" alt="x" loading="lazy" decoding="async">"#),
            "{}",
            out
        );
        assert!(!out.contains("width="), "{}", out);

        let _ = fs::remove_dir_all(&static_dir);
    }

    #[test]
    fn rewrite_images_adds_dimensions_for_resolvable_svg() {
        let static_dir = temp_static_dir("resolvable-svg");
        fs::create_dir_all(static_dir.join("img")).unwrap();
        fs::write(
            static_dir.join("img/x.svg"),
            r#"<svg width="10" height="20"></svg>"#,
        )
        .unwrap();

        let body = "![x](/img/x.svg)\n";
        let out = rewrite_images(body, &static_dir, &mut HashSet::new()).unwrap();
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
        let out = rewrite_images(body, &static_dir, &mut HashSet::new()).unwrap();
        assert!(out.contains("![x](/img/x.png)"), "{}", out);
        assert!(!out.contains("<img"), "{}", out);
    }

    #[test]
    fn rewrite_images_preserves_angle_bracket_alt_text() {
        // `vector<int>`-style alt text (common on a C++ blog) parses as
        // InlineHtml, not Text — must still survive into the alt attribute.
        let static_dir = std::env::temp_dir();
        let body = "![std::shared_ptr<T> layout](https://example.com/x.png)\n";
        let out = rewrite_images(body, &static_dir, &mut HashSet::new()).unwrap();
        assert!(
            out.contains("alt=\"std::shared_ptr&lt;T&gt; layout\""),
            "{}",
            out
        );
    }

    #[test]
    fn rewrite_images_escapes_alt_text() {
        let static_dir = std::env::temp_dir();
        let body = "![a \"quote\" & more](https://example.com/x.png)\n";
        let out = rewrite_images(body, &static_dir, &mut HashSet::new()).unwrap();
        assert!(
            out.contains("alt=\"a &quot;quote&quot; &amp; more\""),
            "{}",
            out
        );
    }

    #[test]
    fn rewrite_images_tracks_referenced_local_paths_only() {
        let static_dir = temp_static_dir("tracked");
        fs::create_dir_all(static_dir.join("img")).unwrap();
        fs::write(
            static_dir.join("img/x.svg"),
            r#"<svg width="1" height="1"></svg>"#,
        )
        .unwrap();

        let body = "![a](/img/x.svg) ![b](https://example.com/y.png)\n";
        let mut referenced = HashSet::new();
        rewrite_images(body, &static_dir, &mut referenced).unwrap();

        assert!(referenced.contains("img/x.svg"), "{:?}", referenced);
        assert_eq!(
            referenced.len(),
            1,
            "external URLs must not be tracked: {:?}",
            referenced
        );

        let _ = fs::remove_dir_all(&static_dir);
    }

    #[test]
    fn rewrite_images_wraps_caption_in_figure() {
        // No blank line between the image and the italic line — the
        // caption convention.
        let static_dir = std::env::temp_dir();
        let body = "![x](https://example.com/x.png)\n*a caption*\n";
        let out = rewrite_images(body, &static_dir, &mut HashSet::new()).unwrap();
        assert!(
            out.contains("<figure><img")
                && out.contains("<figcaption>a caption</figcaption></figure>"),
            "{}",
            out
        );
        // The caption markdown must be fully consumed, not left as
        // dangling literal text after the <img>.
        assert!(!out.contains('*'), "{}", out);
    }

    #[test]
    fn rewrite_images_caps_figure_width_to_image_width() {
        // <figcaption> is an ordinary block element with no width of its
        // own, so without capping the figure it stretches to fill the
        // reading column — wider than a narrower image. The figure's
        // max-width should match the image's resolved intrinsic width.
        let static_dir = temp_static_dir("caption-width");
        fs::create_dir_all(static_dir.join("img")).unwrap();
        fs::write(
            static_dir.join("img/x.svg"),
            r#"<svg width="123" height="45"></svg>"#,
        )
        .unwrap();

        let body = "![x](/img/x.svg)\n*a caption*\n";
        let out = rewrite_images(body, &static_dir, &mut HashSet::new()).unwrap();
        assert!(
            out.contains("<figure style=\"max-width: 123px\">"),
            "{}",
            out
        );

        let _ = fs::remove_dir_all(&static_dir);
    }

    #[test]
    fn rewrite_images_figure_has_no_width_style_when_dimensions_unresolved() {
        // External URL — image_dimensions returns Skipped — no width to
        // constrain the figure to, so no inline style should be emitted.
        let static_dir = std::env::temp_dir();
        let body = "![x](https://example.com/x.png)\n*a caption*\n";
        let out = rewrite_images(body, &static_dir, &mut HashSet::new()).unwrap();
        assert!(out.contains("<figure><img"), "{}", out);
        assert!(!out.contains("style="), "{}", out);
    }

    #[test]
    fn rewrite_images_no_figure_without_caption() {
        let static_dir = std::env::temp_dir();
        let body = "![x](https://example.com/x.png)\n\nmore prose\n";
        let out = rewrite_images(body, &static_dir, &mut HashSet::new()).unwrap();
        assert!(!out.contains("<figure>"), "{}", out);
        assert!(out.contains("<img"), "{}", out);
    }

    #[test]
    fn rewrite_images_ignores_emphasis_in_a_separate_paragraph() {
        // A blank line means this is a new paragraph, not a caption.
        // rewrite_images only ever touches image spans, so the emphasis
        // markdown must survive verbatim for Zola's own markdown pass to
        // turn into <em> later.
        let static_dir = std::env::temp_dir();
        let body = "![x](https://example.com/x.png)\n\n*not a caption*\n";
        let out = rewrite_images(body, &static_dir, &mut HashSet::new()).unwrap();
        assert!(!out.contains("<figure>"), "{}", out);
        assert!(out.contains("*not a caption*"), "{}", out);
    }

    #[test]
    fn rewrite_images_ignores_caption_with_trailing_content() {
        // More text after the italic run in the same paragraph — not the
        // "image, then one line of italics, then nothing else" shape.
        let static_dir = std::env::temp_dir();
        let body = "![x](https://example.com/x.png)\n*text* and more\n";
        let out = rewrite_images(body, &static_dir, &mut HashSet::new()).unwrap();
        assert!(!out.contains("<figure>"), "{}", out);
    }

    #[test]
    fn rewrite_images_escapes_caption_text() {
        // Quotes aren't escaped here — this is text content, not an
        // attribute value, so `"` is safe as-is.
        let static_dir = std::env::temp_dir();
        let body = "![x](https://example.com/x.png)\n*a \"quote\" & <tag>*\n";
        let out = rewrite_images(body, &static_dir, &mut HashSet::new()).unwrap();
        assert!(
            out.contains("<figcaption>a \"quote\" &amp; &lt;tag&gt;</figcaption>"),
            "{}",
            out
        );
    }

    #[test]
    fn rewrite_images_renders_code_spans_in_caption() {
        // A caption containing inline code (e.g. `` `sk1` ``) must still be
        // recognized, with the code span rendered as <code>.
        let static_dir = std::env::temp_dir();
        let body = "![x](https://example.com/x.png)\n*sign with `sk1` please*\n";
        let out = rewrite_images(body, &static_dir, &mut HashSet::new()).unwrap();
        assert!(
            out.contains("<figcaption>sign with <code>sk1</code> please</figcaption>"),
            "{}",
            out
        );
    }

    // ---- find_unused_images ----

    #[test]
    fn find_unused_images_reports_unreferenced_file() {
        let static_dir = temp_static_dir("unused-check");
        fs::create_dir_all(static_dir.join("img/blog")).unwrap();
        fs::write(static_dir.join("img/blog/used.png"), b"x").unwrap();
        fs::write(static_dir.join("img/blog/orphan.png"), b"x").unwrap();

        let mut referenced = HashSet::new();
        referenced.insert("img/blog/used.png".to_string());

        let unused = find_unused_images(&static_dir, &referenced);
        assert_eq!(unused, vec!["img/blog/orphan.png".to_string()]);

        let _ = fs::remove_dir_all(&static_dir);
    }

    #[test]
    fn find_unused_images_empty_when_all_referenced() {
        let static_dir = temp_static_dir("all-referenced");
        fs::create_dir_all(static_dir.join("img")).unwrap();
        fs::write(static_dir.join("img/used.png"), b"x").unwrap();

        let mut referenced = HashSet::new();
        referenced.insert("img/used.png".to_string());

        assert_eq!(
            find_unused_images(&static_dir, &referenced),
            [] as [std::string::String; 0]
        );

        let _ = fs::remove_dir_all(&static_dir);
    }

    #[test]
    fn find_unused_images_empty_when_no_img_dir() {
        let static_dir = temp_static_dir("no-img-dir");
        assert_eq!(
            find_unused_images(&static_dir, &HashSet::new()),
            [] as [std::string::String; 0]
        );
    }

    // ---- extract_path_args ----

    #[test]
    fn extract_path_args_finds_single_call() {
        let content = r#"{% set pic = resize_image(path="img/prof_pic.jpg", width=448) %}"#;
        assert_eq!(
            extract_path_args(content, "resize_image"),
            vec!["img/prof_pic.jpg".to_string()]
        );
    }

    #[test]
    fn extract_path_args_finds_multiple_calls() {
        let content = r#"
            {{ get_url(path="favicon.svg") }}
            {{ get_url(path="img/a.png") }}
        "#;
        assert_eq!(
            extract_path_args(content, "get_url"),
            vec!["favicon.svg".to_string(), "img/a.png".to_string()]
        );
    }

    #[test]
    fn extract_path_args_ignores_calls_without_path_arg() {
        let content = r"{{ some_other_fn(width=100) }}";
        assert_eq!(
            extract_path_args(content, "some_other_fn"),
            [] as [std::string::String; 0]
        );
    }

    #[test]
    fn extract_path_args_ignores_unrelated_function_names() {
        let content = r#"{{ get_url(path="img/a.png") }}"#;
        assert_eq!(
            extract_path_args(content, "resize_image"),
            [] as [std::string::String; 0]
        );
    }

    // ---- collect_template_image_refs ----

    #[test]
    fn collect_template_image_refs_finds_resize_image_call() {
        let templates_dir = temp_static_dir("templates-resize-image");
        fs::create_dir_all(&templates_dir).unwrap();
        fs::write(
            templates_dir.join("index.html"),
            r#"{% set pic = resize_image(path="img/prof_pic.jpg", width=448, height=448) %}"#,
        )
        .unwrap();

        let mut referenced = HashSet::new();
        collect_template_image_refs(&templates_dir, &mut referenced);
        assert!(referenced.contains("img/prof_pic.jpg"), "{:?}", referenced);

        let _ = fs::remove_dir_all(&templates_dir);
    }

    #[test]
    fn collect_template_image_refs_finds_hand_written_img_tag() {
        let templates_dir = temp_static_dir("templates-raw-img");
        fs::create_dir_all(&templates_dir).unwrap();
        fs::write(
            templates_dir.join("page.html"),
            r#"<img src="/img/blog/hero.png" alt="hero">"#,
        )
        .unwrap();

        let mut referenced = HashSet::new();
        collect_template_image_refs(&templates_dir, &mut referenced);
        assert!(referenced.contains("img/blog/hero.png"), "{:?}", referenced);

        let _ = fs::remove_dir_all(&templates_dir);
    }

    #[test]
    fn collect_template_image_refs_ignores_non_image_paths() {
        let templates_dir = temp_static_dir("templates-non-image");
        fs::create_dir_all(&templates_dir).unwrap();
        fs::write(
            templates_dir.join("base.html"),
            r#"{{ get_url(path="style.css") }} {{ get_url(path="favicon.svg") }}"#,
        )
        .unwrap();

        let mut referenced = HashSet::new();
        collect_template_image_refs(&templates_dir, &mut referenced);
        assert!(referenced.is_empty(), "{:?}", referenced);

        let _ = fs::remove_dir_all(&templates_dir);
    }

    #[test]
    fn collect_template_image_refs_scans_nested_directories() {
        let templates_dir = temp_static_dir("templates-nested");
        fs::create_dir_all(templates_dir.join("components")).unwrap();
        fs::write(
            templates_dir.join("components/figure.html"),
            r#"{{ get_image_metadata(path="img/deep/nested.png") }}"#,
        )
        .unwrap();

        let mut referenced = HashSet::new();
        collect_template_image_refs(&templates_dir, &mut referenced);
        assert!(
            referenced.contains("img/deep/nested.png"),
            "{:?}",
            referenced
        );

        let _ = fs::remove_dir_all(&templates_dir);
    }
}
