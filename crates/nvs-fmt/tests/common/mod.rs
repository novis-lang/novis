//! The corpus `tools/data/nvs-fmt-corpus.txt` declares, shared by `identity.rs`
//! and `corpus.rs`.
//!
//! Each `tests/*.rs` file is its own binary, so this lives here and is reached
//! through `mod common;`. `tools/nv/lib/nvs-fmt-corpus.ts` reads the same file
//! for `bun nv nvs-fmt` and `bun nv verify`, and the file says what each entry
//! means. A page's ```nvs blocks are read line by line, as
//! `tools/nv/lib/nvs-fences.ts` reads them.

#![allow(
    dead_code,
    reason = "each test binary compiles all of this module, and `corpus.rs` reads only the files"
)]

use std::path::{Path, PathBuf};

/// The entries of the corpus file, each as a path in the tree.
struct Corpus {
    trees: Vec<PathBuf>,
    skips: Vec<PathBuf>,
    pages: Vec<PathBuf>,
}

/// The corpus file, read.
///
/// # Panics
///
/// If the file cannot be read, or holds a line that is not a `tree`, a
/// `pages` or a `skip`.
fn corpus() -> Corpus {
    let list = nvs_repo::path("tools/data/nvs-fmt-corpus.txt");
    let text =
        std::fs::read_to_string(&list).unwrap_or_else(|err| panic!("{}: {err}", list.display()));
    let mut corpus = Corpus {
        trees: Vec::new(),
        skips: Vec::new(),
        pages: Vec::new(),
    };
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line.split_whitespace().collect::<Vec<_>>().as_slice() {
            ["tree", dir] => corpus.trees.push(nvs_repo::path(dir)),
            ["pages", path] => corpus.pages.push(nvs_repo::path(path)),
            ["skip", path, "--", ..] => corpus.skips.push(nvs_repo::path(path)),
            _ => panic!(
                "{}: `{line}` is not a tree, a pages entry or a skip",
                list.display()
            ),
        }
    }
    corpus
}

/// Every `.nvs` file under a `tree` of the corpus file, sorted, so a failure
/// names the same file on every machine. With `skipping`, a file a `skip`
/// names, and every file under one, is left out.
pub(crate) fn files(skipping: bool) -> Vec<PathBuf> {
    let corpus = corpus();
    let mut out = Vec::new();
    for tree in &corpus.trees {
        collect(tree, &["nvs"], &mut out);
    }
    if skipping {
        out.retain(|path| !corpus.skips.iter().any(|skip| path.starts_with(skip)));
    }
    out.sort();
    out
}

/// Every `.md` and `.mdx` file that is a `pages` entry or under one, and
/// neither a skip nor under one, sorted.
pub(crate) fn pages() -> Vec<PathBuf> {
    let corpus = corpus();
    let mut out = Vec::new();
    for page in &corpus.pages {
        if page.is_file() {
            out.push(page.clone());
        } else {
            collect(page, &["md", "mdx"], &mut out);
        }
    }
    out.retain(|path| !corpus.skips.iter().any(|skip| path.starts_with(skip)));
    out.sort();
    out.dedup();
    out
}

/// Every file under `dir` with one of `extensions`, appended to `out`. A
/// directory whose name starts with `.` and `node_modules` hold no file of ours.
fn collect(dir: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            if name != "node_modules" && !name.to_string_lossy().starts_with('.') {
                collect(&path, extensions, out);
            }
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| extensions.contains(&e))
        {
            out.push(path);
        }
    }
}

/// One ```nvs block of a page: the line of its opening fence, and its body
/// without the indent the fence has, each line ending in `\n`.
pub(crate) struct Block {
    pub(crate) line: usize,
    pub(crate) body: String,
}

/// The ```nvs and ```novis blocks of `text`, in order. A block closes on the
/// first later line that is its fence's indent and ``` alone.
pub(crate) fn blocks(text: &str) -> Vec<Block> {
    let lines: Vec<&str> = text
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let indent = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
        let rest = &line[indent.len()..];
        let opens = ["```nvs", "```novis"].iter().any(|fence| {
            rest.strip_prefix(fence)
                .is_some_and(|after| !after.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
        });
        if !opens {
            i += 1;
            continue;
        }
        let close = format!("{indent}```");
        let Some(end) = (i + 1..lines.len()).find(|&k| lines[k] == close) else {
            break;
        };
        let body: String = lines[i + 1..end]
            .iter()
            .map(|l| {
                let l = l.strip_prefix(indent).unwrap_or_else(|| l.trim_start());
                format!("{l}\n")
            })
            .collect();
        out.push(Block { line: i + 1, body });
        i = end + 1;
    }
    out
}

/// What the formatter reads for a block: the body itself when it opens code
/// with a tag, and the body behind a `<?nvs` line when it is code from its
/// first line.
pub(crate) fn wrapped(body: &str) -> String {
    if tagged(body) {
        body.to_owned()
    } else {
        format!("<?nvs\n{body}")
    }
}

/// The formatter's output for [`wrapped`]`(body)`, back as a block body, or
/// [`None`] when it lost the `<?nvs` line the block was given.
pub(crate) fn unwrapped(body: &str, formatted: &str) -> Option<String> {
    if tagged(body) {
        Some(formatted.to_owned())
    } else {
        formatted.strip_prefix("<?nvs\n").map(str::to_owned)
    }
}

fn tagged(body: &str) -> bool {
    body.contains("<?nvs") || body.contains("<?=")
}
