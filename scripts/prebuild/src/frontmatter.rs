use std::collections::HashMap;

use gray_matter::engine::TOML;
use gray_matter::{Matter, Pod};

/// Extract the TOML front matter (between +++ lines) and the body from a Zola markdown file.
/// Returns (`front_matter_str`, `body_str`). If no front matter, returns ("", `full_content`).
///
/// Delegates the actual delimiter/edge-case handling (EOF without a trailing
/// newline, no front matter at all, etc.) to `gray_matter` rather than
/// hand-rolling it — Zola's `+++` convention is just `gray_matter`'s default
/// `---` with a custom delimiter. `Pod` (the crate's own loosely-typed
/// value) is used as the target type since we only need the raw
/// `matter`/`content` strings back, not a parsed value — `parse_macros`
/// below does its own separate, specifically-typed parse of the raw string.
pub fn split_front_matter(content: &str) -> (String, String) {
    let trimmed = content.trim_start();
    let mut matter: Matter<TOML> = Matter::new();
    "+++".clone_into(&mut matter.delimiter);
    match matter.parse::<Pod>(trimmed) {
        Ok(parsed) if !parsed.matter.is_empty() => (parsed.matter, parsed.content),
        _ => (String::new(), content.to_string()),
    }
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

/// Parse `katex_macros` from the [extra] section of TOML front matter.
/// Supports both inline-table form (`katex_macros = { ... }`) and
/// table form (`[extra.katex_macros]`), plus any other valid TOML the
/// front matter may contain. Returns an empty map when there are no macros
/// or when the front matter is empty/malformed (with a warning).
pub fn parse_macros(front_matter: &str) -> HashMap<String, String> {
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

    #[test]
    fn split_front_matter_eof_without_trailing_newline() {
        let (fm, body) = split_front_matter("+++\ntitle = \"Test\"\n+++");
        assert_eq!(fm, "title = \"Test\"");
        assert_eq!(body, "");
    }

    #[test]
    fn split_front_matter_none() {
        let (fm, body) = split_front_matter("# Just a heading\n");
        assert_eq!(fm, "");
        assert_eq!(body, "# Just a heading\n");
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
        let fm =
            "title = \"Test\"\r\n[extra]\r\nkatex_macros = { \"\\\\R\" = \"\\\\mathbb{R}\" }\r\n";
        let macros = parse_macros(&fm.replace("\r\n", "\n"));
        assert_eq!(macros.get("\\R").unwrap(), "\\mathbb{R}");
    }
}
