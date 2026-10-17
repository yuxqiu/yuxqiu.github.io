# AGENTS.md

## Build Commands

```sh
make build    # prebuild (KaTeX) + zola build → public/
make serve    # prebuild + zola serve (live reload at http://127.0.0.1:1111)
              #   also watches src/ via watchexec and re-runs prebuild on
              #   changes, so editing src/ is reflected without restarting
make clean    # rm -rf public content
make prebuild # just the Rust KaTeX prebuild step
make scene    # just the footer-scene fallback PNGs (see Footer scene below)
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

1. **Prebuild** (`scripts/prebuild/src/main.rs`): Rust script that reads `src/*.md`, renders LaTeX math to KaTeX HTML at build time (no client-side JS), and writes to `content/`. Non-markdown files are copied as-is. Writes are incremental — a file is rewritten only when its rendered output changed, so `make serve` triggers one zola rebuild per edit instead of a storm. Math regions are located with `pulldown-cmark`'s `ENABLE_MATH` option (`Event::InlineMath` / `Event::DisplayMath`), which is the single source of truth for where math is. The parser never emits math events inside code, so code exclusion is automatic. Only `$...$` and `$$...$$` are recognized as math delimiters; `\(...\)` and `\[...\]` are not supported (use `\$` for literal dollars). CRLF line endings are normalized to LF on read. Markdown-significant characters (`*`, `_`, `\`, `-`, …) in the text between KaTeX's tags are emitted as numeric character references, because Zola still parses that text as markdown (otherwise two `^*` in one paragraph pair into `<em>` and corrupt the math). Orphans (deleted src files) are pruned from `content/`. Tests: `cargo test --manifest-path scripts/prebuild/Cargo.toml`. Before committing changes to `scripts/prebuild/`, run `cargo fmt --manifest-path scripts/prebuild/Cargo.toml` and `cargo clippy --manifest-path scripts/prebuild/Cargo.toml --all-targets` (lint config, including the `pedantic`/`nursery` groups, lives in `scripts/prebuild/Cargo.toml`'s `[lints.clippy]`) — both must be clean.
2. **Zola build**: Reads `content/`, applies templates from `templates/`, compiles `sass/style.scss`, copies `static/`, outputs to `public/`.

Both `content/` and `public/` are gitignored. Only `src/` is edited by hand.

### Footer scene

Every page ends in a small pixel-art scene in the Kami palette plus a few muted naturals (sage, ochre, rust, wood), running into a footer whose background is the scene's sand ground (`--warm-sand`). A few pages have a pinned scene, listed in `PINNED` in `static/js/scene/scene.js`: the homepage (a ginger cat asleep on a stack of books in a meadow — tail flick, a drifting z — with flowers, a tree, and a small walker in a sun hat and ink-blue jacket) and Publications (a meadow with a tree, flowers, and the walker). Every other page composes its own scene from a kit — base lake or meadow; reeds, dock with lantern, cat on books, tree, flowers, rowboat, kite flyer, butterfly; walker or not — seeded by the page's path (`data-path` on `.scene`), so a page keeps its scene and readers find different small things on different pages. To keep a page's scene for good, copy what `sceneFor(path)` returns into `PINNED`; pinned scenes don't move when the kit or seeding changes (an item's own drawing code still applies).

The code is three ES modules in `static/js/scene/` plus a Node build step:

- `kit.js` — the art: palette, sprites, bases (`BASES`, backdrops that repeat every `PERIOD` pixels from a fixed seed, transparent skies) and items (`ITEMS`; the walker is an item too). Edit here to change how something looks or moves.
- `scene.js` — which scene each page gets (`PINNED`, the seeded composition, `FALLBACK`), as **layers**: an ordered list of `{ id, w, at, tile, paint }`. Tile layers repeat from x = 0; others sit with their left edge at `at` of the leftover width, `round(at × (W − w))`, which is exactly how CSS places a background at `at × 100%`. Interface: `layersFor(path)`, `fallbacks()`, `paint(fill, layers, W, t)`, plus `H`/`SCALE`/`NARROW_SCALE`/`NARROW_MAX`. No DOM.
- `canvas.js` — the browser entry `base.html` loads: paints `layersFor(data-path)` on the canvas, animated unless `prefers-reduced-motion`. It also reads the footer's computed background each frame: dark-mode extensions (Dark Reader and the like) recolor the page's CSS but not canvas pixels, so when the footer is no longer `GROUND`, `recolor()` (in `scene.js`) flips the scene's lightness around the footer's new color — the ground meets the footer exactly, near-neutral grays take its tint, real colors keep their hue. The no-JS PNGs aren't recolored by such extensions.
- `scripts/scene/render.mjs` (run by `make scene`, a dependency of `make build`/`make serve`) writes the no-JS fallback into `static/scene/` (gitignored): one PNG per layer of each scene in `fallbacks()`, cut out at frame 0, and `scene.css`, which sizes `.scene` and stacks those PNGs per page (`.scene[data-path=…]`; the plain lake with the walker for unpinned pages). `png.mjs` encodes PNGs; `raster.mjs` paints into a pixel buffer.

The invariant that makes this work: at frame 0 a non-tile layer paints only inside its own `[x, x + w)`, so cutting it out and placing it with CSS reproduces the live frame. `node --test scripts/scene/scene.test.mjs` checks that each fallback composited equals the live frame 0 at several widths, that page scenes are deterministic and distinct, and the dark-mode recolor; run it after changing the kit or scene code. An inline script in `base.html`'s head adds `.js` to `<html>` before first paint, and `scene.css` hides the fallback under `.js`, so visitors with JS never see it flash. Per-page fallbacks for unpinned pages were considered and skipped: cheap in bytes, but the build would need every page's path before Zola runs, for the benefit of no-JS visitors only. The only value duplicated outside the scene code is `K.sand` ↔ `--warm-sand` (the footer background), marked with a comment. The footer overlaps the scene by 1px (`margin-top: -1px; position: relative`) so fractional offsets on long pages don't leave a light hairline at some zoom levels. The site must read fine without JS. Rejected along the way: a blue lake with gulls and a sky-blue homepage (too eye-catching), fading pages into sky, a lake with drifting light dashes (read as floating debris), and one fixed scene on every page. Local check: `zola build` writes absolute `https://yuxqiu.github.io/...` URLs for CSS/JS, so to test changes on a static server build with `--base-url http://127.0.0.1:<port>` and serve on that same port (`make serve` handles this itself).

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
url = "/pdf/papers/<yyyy>-<slug>/<slug>-<venue><yy>.pdf"
```

Paper assets are vendored into `static/pdf/papers/<yyyy>-<slug>/` (one directory per publication) and linked by absolute path rather than to external hosts. Filenames are self-describing, because browsers save a PDF under its URL's last segment: `<slug>-<venue><yy>.pdf`, with a suffix for extras (e.g. `2023-truelearn/truelearn-orsum23.pdf`, `truelearn-orsum23-slides.pdf`). PDFs linked from blog posts follow the same scheme under `static/pdf/writings/`, using the four-digit year in place of the venue (e.g. `2025-dlbf/dlbf-2025.pdf`).

## Template Architecture

- `base.html` — master template (nav, footer scene, footer, head, skip-link). Nav links come from `zola.toml` `[extra] nav` array. Head emits a canonical link and OG/Twitter meta tags with site-wide defaults; child templates override them via the `title`, `description`, `og_type`, `og_title`, `og_description` blocks (Tera blocks don't share scope across each other or with top-level `{% set %}`, so each block must recompute its own value rather than reusing a variable).
- `partials/macros.html` — Tera macros. Currently exports `page_header(title, desc="", meta="", tags=[])`, the unified page title header used by all page templates via `{% import "partials/macros.html" as m %}`.
- `partials/social-links.html` / `partials/home-links.html` — two deliberately different link lists, not a shared partial: `social-links.html` (GitHub, LinkedIn, Keybase, RSS) is included by `base.html`'s footer; `home-links.html` (CV, GitHub, LinkedIn, Email) is included by `index.html`'s homepage link row. Some links (Keybase, RSS) are intentionally footer-only and others (CV, Email) intentionally homepage-only — this is a curated split per page, not drift to fix by merging them into one list.
- `index.html` — homepage with photo, name, tagline, bio.
- `section.html` — blog listing, grouped by year.
- `blog-page.html` — individual blog post.
- `page.html` — generic page (Misc page).
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

### Font-loading strategy (intentional, not a bug)

`--font-serif`/`--font-sans`/`--font-mono` name "Source Serif 4", "Inter", and "JetBrains Mono" first, but no `@font-face`/webfont is shipped anywhere — this is deliberate, not an oversight. Visitors who already have those fonts installed locally get them; everyone else gets the fallback stack (Charter/Georgia/Times New Roman, system-ui, SF Mono/Cascadia Code), which the design already accounts for. Do not add a font CDN link or self-hosted `@font-face` to "fix" this.

## Configuration

`zola.toml` contains:
- Site metadata (title, description, base_url)
- Build settings (sass compilation, feeds, sitemap)
- Taxonomies (tags)
- Markdown rendering options
- `[extra]` section: author, tagline (array of topics — joined with " · ", stacked one per line below 430px), social links, CV path, and `nav` array for navigation links
