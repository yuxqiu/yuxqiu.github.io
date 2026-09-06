use std::collections::HashMap;

/// Extract the TOML front matter (between +++ lines) and the body from a Zola markdown file.
/// Returns (`front_matter_str`, `body_str`). If no front matter, returns ("", `full_content`).
pub fn split_front_matter(content: &str) -> (String, String) {
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
