use std::collections::HashSet;
use std::fs;
use std::path::Path;

use walkdir::WalkDir;

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

/// Outcome of resolving a markdown image's intrinsic pixel dimensions.
/// Kept as three distinct cases (rather than collapsing to `Option`) so the
/// caller can react differently: a typo'd/deleted local file is a build
/// error, an external URL or unsupported format is not.
#[derive(Debug, PartialEq)]
enum ImageDims {
    /// Dimensions were resolved — inject `width`/`height`.
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
fn image_dimensions(static_dir: &Path, dest_url: &str) -> ImageDims {
    let Some(rel) = dest_url.strip_prefix('/') else {
        return ImageDims::Skipped;
    };
    let path = static_dir.join(rel);
    if !path.is_file() {
        return ImageDims::Missing;
    }
    let dims = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("svg") => svg_dimensions(&path),
        _ => imagesize::size(&path)
            .ok()
            .map(|d| (d.width as u32, d.height as u32)),
    };
    match dims {
        Some((w, h)) => ImageDims::Resolved(w, h),
        None => ImageDims::Unrecognized,
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
pub(crate) fn rewrite_images(
    body: &str,
    static_dir: &Path,
    referenced: &mut HashSet<String>,
) -> Result<String, String> {
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
            // Alt text containing `<...>` (e.g. `vector<int>`, `shared_ptr<T>`
            // — common on a C++ blog) parses as inline HTML, not Text. It's
            // not real HTML here (alt attributes don't render markup), so
            // treat it as literal text too — otherwise it silently vanishes
            // from the alt attribute instead of erroring or warning.
            pulldown_cmark::Event::InlineHtml(t) => {
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

        if span.alt.is_empty() {
            eprintln!("  WARN: image has no alt text: {}", span.dest_url);
        }
        if let Some(rel) = span.dest_url.strip_prefix('/') {
            referenced.insert(rel.to_string());
        }

        match image_dimensions(static_dir, &span.dest_url) {
            ImageDims::Resolved(w, h) => {
                result.push_str(&format!(
                    "<img src=\"{}\" alt=\"{}\" width=\"{}\" height=\"{}\" loading=\"lazy\" decoding=\"async\">",
                    src, alt, w, h
                ));
            }
            ImageDims::Skipped => {
                result.push_str(&format!(
                    "<img src=\"{}\" alt=\"{}\" loading=\"lazy\" decoding=\"async\">",
                    src, alt
                ));
            }
            ImageDims::Unrecognized => {
                eprintln!(
                    "  WARN: could not determine dimensions for {} (unsupported or corrupt image)",
                    span.dest_url
                );
                result.push_str(&format!(
                    "<img src=\"{}\" alt=\"{}\" loading=\"lazy\" decoding=\"async\">",
                    src, alt
                ));
            }
            ImageDims::Missing => {
                return Err(format!(
                    "image not found: {} (no file at static{})",
                    span.dest_url, span.dest_url
                ));
            }
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
pub(crate) fn find_unused_images(static_dir: &Path, referenced: &HashSet<String>) -> Vec<String> {
    let img_dir = static_dir.join("img");
    if !img_dir.is_dir() {
        return Vec::new();
    }

    let mut unused: Vec<String> = WalkDir::new(&img_dir)
        .into_iter()
        .filter_map(|e| e.ok())
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
pub(crate) fn collect_template_image_refs(templates_dir: &Path, referenced: &mut HashSet<String>) {
    for entry in WalkDir::new(templates_dir).into_iter().filter_map(|e| e.ok()) {
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
        fs::write(static_dir.join("img/x.svg"), r#"<svg width="1" height="1"></svg>"#).unwrap();

        let body = "![a](/img/x.svg) ![b](https://example.com/y.png)\n";
        let mut referenced = HashSet::new();
        rewrite_images(body, &static_dir, &mut referenced).unwrap();

        assert!(referenced.contains("img/x.svg"), "{:?}", referenced);
        assert_eq!(referenced.len(), 1, "external URLs must not be tracked: {:?}", referenced);

        let _ = fs::remove_dir_all(&static_dir);
    }

    #[test]
    fn escape_html_attr_escapes_all_special_chars() {
        assert_eq!(
            escape_html_attr("a \"quote\" & <tag>"),
            "a &quot;quote&quot; &amp; &lt;tag&gt;"
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

        assert!(find_unused_images(&static_dir, &referenced).is_empty());

        let _ = fs::remove_dir_all(&static_dir);
    }

    #[test]
    fn find_unused_images_empty_when_no_img_dir() {
        let static_dir = temp_static_dir("no-img-dir");
        assert!(find_unused_images(&static_dir, &HashSet::new()).is_empty());
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
        let content = r#"{{ some_other_fn(width=100) }}"#;
        assert!(extract_path_args(content, "some_other_fn").is_empty());
    }

    #[test]
    fn extract_path_args_ignores_unrelated_function_names() {
        let content = r#"{{ get_url(path="img/a.png") }}"#;
        assert!(extract_path_args(content, "resize_image").is_empty());
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
        assert!(
            referenced.contains("img/blog/hero.png"),
            "{:?}",
            referenced
        );

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
        fs::create_dir_all(templates_dir.join("shortcodes")).unwrap();
        fs::write(
            templates_dir.join("shortcodes/figure.html"),
            r#"{{ get_image_metadata(path="img/deep/nested.png") }}"#,
        )
        .unwrap();

        let mut referenced = HashSet::new();
        collect_template_image_refs(&templates_dir, &mut referenced);
        assert!(referenced.contains("img/deep/nested.png"), "{:?}", referenced);

        let _ = fs::remove_dir_all(&templates_dir);
    }
}
