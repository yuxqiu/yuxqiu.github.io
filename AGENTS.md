# AGENTS.md

## Build Commands

```sh
make build    # prebuild (KaTeX) + zola build → public/
make serve    # prebuild + zola serve (live reload at http://127.0.0.1:1111)
              #   also watches src/ via watchexec and re-runs prebuild on
              #   changes, so editing src/ is reflected without restarting
make clean    # rm -rf public content
make prebuild # just the Rust KaTeX prebuild step
```

All commands run through `nix develop --command` for reproducibility.
`make serve` is the only supported dev entry point — running `zola serve`
directly bypasses the prebuild and yields a stale/empty site (zola watches
`content/`, not `src/`, and cannot run the prebuild itself).

## Build Pipeline

```
src/           → content/          → public/
(markdown)       (prebuild)           (zola build)
```

1. **Prebuild** (`scripts/prebuild/src/main.rs`): Rust script that reads `src/*.md`, renders LaTeX math to KaTeX HTML at build time (no client-side JS), and writes to `content/`. Non-markdown files are copied as-is. Writes are incremental — a file is rewritten only when its rendered output changed, so `make serve` triggers one zola rebuild per edit instead of a storm. Math regions are located with `pulldown-cmark`'s `ENABLE_MATH` option (`Event::InlineMath` / `Event::DisplayMath`), which is the single source of truth for where math is. The parser never emits math events inside code, so code exclusion is automatic. Only `$...$` and `$$...$$` are recognized as math delimiters; `\(...\)` and `\[...\]` are not supported (use `\$` for literal dollars). CRLF line endings are normalized to LF on read. Orphans (deleted src files) are pruned from `content/`. Tests: `cargo test --manifest-path scripts/prebuild/Cargo.toml`.
2. **Zola build**: Reads `content/`, applies templates from `templates/`, compiles `sass/style.scss`, copies `static/`, outputs to `public/`.

Both `content/` and `public/` are gitignored. Only `src/` is edited by hand.

## Adding Content

### Blog post
1. Create `src/blog/YYYY-MM-DD-slug.md`
2. Frontmatter:
   ```toml
   +++
   title = "Post Title"
   date = YYYY-MM-DD
   taxonomies = { tags = ["tag1", "tag2"] }
   +++
   ```
3. Tags must be lowercase (e.g., `til`, `cpp`, `cryptography`).
4. Math: use `$...$` for inline, `$$...$$` for display. The prebuild renders these to KaTeX HTML. Use `\$` for a literal dollar sign in prose. `\(...\)` and `\[...\]` are not supported. **No blank lines inside `$$...$$` blocks** — pulldown-cmark treats blank lines as paragraph breaks, which terminates the math block.
5. Custom KaTeX macros: add `[extra]` section with `katex_macros = { "\\macro" = "\\definition" }`.

### Writing
Same as blog but in `src/writings/`. Uses the same `section.html` and `blog-page.html` templates.

### Publication
Edit `src/publications.md`. Add entries to the `[[extra.publications]]` array:
```toml
[[extra.publications]]
title = "Paper Title"
authors = "Author1, Author2"
venue = "Conference Name"
year = 2024

[[extra.publications.links]]
name = "PDF"
url = "/pdf/..."
```

## Template Architecture

- `base.html` — master template (nav, footer, head). Nav links come from `zola.toml` `[extra] nav` array.
- `partials/macros.html` — Tera macros. Currently exports `page_header(title, desc="", meta="", tags=[])`, the unified page title header used by all page templates via `{% import "partials/macros.html" as m %}`.
- `partials/social-links.html` — shared social link list (CV, GitHub, LinkedIn, Email, Keybase, RSS). Used by both `base.html` footer and `index.html` home links.
- `index.html` — homepage with photo, name, tagline, bio.
- `section.html` — blog/writings listing, grouped by year.
- `blog-page.html` — individual blog/writing post.
- `page.html` — generic page (More page).
- `publications.html` — publications list.
- `404.html` — error page.
- `tags/list.html`, `tags/single.html` — tag index and tag detail.

## CSS Architecture

`sass/style.scss` — single file, ~1030 lines, organized in numbered sections:

1. **Design tokens** (`:root`) — colors, fonts, type scale, spacing, layout. All values flow through semantic mappings (`--text`, `--accent`, `--border`, etc.).
2. **Mixins** — `link-reset`, `last-child-flush`, `callout-base` for repeated patterns.
3. **Reset & base** — universal reset, body defaults.
4. **Layout** — `.container` (680px reading width), `.container--wide` (820px).
5. **Navigation** — sticky header, BEM-named `.site-nav__*`.
6. **Footer** — `.site-footer__*`.
7. **Typography elements** — headings, code, blockquotes, tables, footnotes, images.
8. **Page-specific** — `.page-header`/`.page-title` (unified), `.home__*`, `.post__*`, `.pubs`/`.pub__*`, `.list__*`.
9. **GitHub alert styles** — `.markdown-alert-{note,tip,important,warning,caution}`, proof-end, sidenote.
10. **KaTeX adjustments** — math display spacing.
11. **Travel map** — SVG world map styling.
12. **Utility** — `.sr-only`.

### Kami Design Constraints

The site follows the [Kami](https://github.com/tw93/Kami) design system:

- **Parchment background** `#f5f4ed`, never pure white
- **Single accent**: ink blue `#1B365D`, ≤5% of page area
- **All grays warm-toned** (yellow-brown undertone), no cool blue-gray
- **Serif weight locked at 500**, no bold (except publication titles which use 600)
- **Line heights**: headlines 1.1–1.3, dense 1.4–1.45, reading 1.5–1.55. Never 1.6+
- **No italics** anywhere
- **Depth via ring/whisper shadow**, not hard drop shadows
- **Tag backgrounds**: solid hex, no rgba

## Configuration

`zola.toml` contains:
- Site metadata (title, description, base_url)
- Build settings (sass compilation, feeds, sitemap)
- Taxonomies (tags)
- Markdown rendering options
- `[extra]` section: author, tagline, social links, CV path, and `nav` array for navigation links
