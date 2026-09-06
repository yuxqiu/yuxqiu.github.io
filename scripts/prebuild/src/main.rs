use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use walkdir::WalkDir;

mod frontmatter;
mod images;
mod math;

use frontmatter::{parse_macros, split_front_matter};
use images::{collect_template_image_refs, find_unused_images, rewrite_images};
use math::render_math;

/// Process a single markdown file: read, split front matter, rewrite images,
/// render math, write to output. Skips the write when the rendered output is
/// identical to the existing destination, so a live file watcher (zola serve)
/// only sees changes for files that actually changed — avoiding a rebuild
/// storm when one source file is edited.
fn process_file(
    src: &Path,
    dst: &Path,
    static_dir: &Path,
    referenced_images: &mut HashSet<String>,
) -> Result<bool, String> {
    let raw = fs::read_to_string(src).map_err(|e| format!("read {}: {}", src.display(), e))?;
    let content = raw.replace("\r\n", "\n");

    let (front_matter, body) = split_front_matter(&content);
    let macros = parse_macros(&front_matter);

    if !macros.is_empty() {
        eprintln!("  macros: {:?}", macros);
    }

    let body = rewrite_images(&body, static_dir, referenced_images)?;
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
    let templates_dir = PathBuf::from("templates");

    if !src_dir.exists() {
        eprintln!("ERROR: {} directory does not exist", src_dir.display());
        process::exit(1);
    }

    fs::create_dir_all(&dst_dir).expect("could not create content dir");

    let mut written: Vec<PathBuf> = Vec::new();
    let mut processed = 0;
    let mut copied = 0;
    let mut referenced_images: HashSet<String> = HashSet::new();

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
                if let Err(e) = process_file(path, &dst, &static_dir, &mut referenced_images) {
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

    collect_template_image_refs(&templates_dir, &mut referenced_images);

    for unused in find_unused_images(&static_dir, &referenced_images) {
        eprintln!("  WARN: unused image asset: static/{}", unused);
    }

    eprintln!(
        "Done: {} markdown files processed, {} files copied",
        processed, copied
    );
}
