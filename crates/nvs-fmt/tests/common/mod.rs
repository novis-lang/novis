//! The corpus `tools/data/nvs-fmt-corpus.txt` declares, shared by `identity.rs`
//! and `corpus.rs`.
//!
//! Each `tests/*.rs` file is its own binary, so this lives here and is reached
//! through `mod common;`. `tools/nv/lib/nvs-fmt-corpus.ts` reads the same file
//! for `bun nv nvs-fmt` and `bun nv verify`, and the file says what each entry
//! means.

use std::path::{Path, PathBuf};

/// Every `.nvs` file under a `tree` of the corpus file, sorted, so a failure
/// names the same file on every machine. With `skipping`, a file a `skip`
/// names, and every file under one, is left out.
///
/// # Panics
///
/// If the corpus file cannot be read, or holds a line that is neither a
/// `tree` nor a `skip`.
pub(crate) fn files(skipping: bool) -> Vec<PathBuf> {
    let list = nvs_repo::path("tools/data/nvs-fmt-corpus.txt");
    let text =
        std::fs::read_to_string(&list).unwrap_or_else(|err| panic!("{}: {err}", list.display()));
    let mut trees = Vec::new();
    let mut skips = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line.split_whitespace().collect::<Vec<_>>().as_slice() {
            ["tree", dir] => trees.push(nvs_repo::path(dir)),
            ["skip", dir, "--", ..] => skips.push(nvs_repo::path(dir)),
            _ => panic!("{}: `{line}` is neither a tree nor a skip", list.display()),
        }
    }
    let mut out = Vec::new();
    for tree in &trees {
        collect(tree, &mut out);
    }
    if skipping {
        out.retain(|path| !skips.iter().any(|skip| path.starts_with(skip)));
    }
    out.sort();
    out
}

/// Every `.nvs` file under `dir`, appended to `out`. A directory whose name
/// starts with `.` and `node_modules` hold no file of ours.
fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            if name != "node_modules" && !name.to_string_lossy().starts_with('.') {
                collect(&path, out);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("nvs") {
            out.push(path);
        }
    }
}
