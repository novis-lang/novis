//! The two claims the whole corpus makes about `nvs fmt`: its output is a fixed
//! point, and it is never a semantic change.
//!
//! Both are properties of every file rather than of a construct, so neither has
//! a fixture: a rule that converges on the cases somebody thought to write down
//! and diverges on the ninety-ninth corpus file is exactly the failure these
//! exist to catch. `rule:tooling/fmt-is-idempotent` is the first, and the goal's
//! § *Standing decisions* — "never a semantic change" — is the second.
//!
//! The walk is `crates/nvs-fmt/tests/identity.rs`'s, with the one directory that
//! test steps over put back: `tests/fmt/input` holds files that are wrong about
//! a rule on purpose, which makes them the most interesting input here. A file
//! that is already formatted proves idempotence over a no-op; a file the
//! formatter has to move proves it over the move.
//!
//! A file the formatter refuses is skipped by both, because a refusal is the
//! claim `identity.rs` makes and a file that produces no text has no second
//! pass to compare.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use nvs_diagnostics::SourceMap;
use nvs_fmt::format;
use nvs_syntax::walk::{self, Field, Node};

/// The repository root, from this crate's manifest.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.nvs` file under `dir`, appended to `out`.
fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("nvs") {
            out.push(path);
        }
    }
}

/// Every corpus file, sorted, so a failure names the same file on every machine.
fn corpus() -> Vec<PathBuf> {
    let root = repo_root();
    let mut paths = Vec::new();
    collect(&root.join("examples"), &mut paths);
    collect(&root.join("tests"), &mut paths);
    paths.sort();
    paths
}

/// `text` formatted under `name`, or [`None`] for text the formatter refuses.
fn formatted(name: &str, text: &str) -> Option<String> {
    let mut map = SourceMap::new();
    let id = map.add(name.to_owned(), text);
    format(map.file(id)).ok()
}

/// Where two renderings of one file first diverge, as the line number and the
/// two lines themselves.
///
/// A byte count would say a file moved and leave the reader to find out where;
/// the split is on `'\n'` rather than by [`str::lines`] so that a difference in
/// the final newline alone is a difference in the last line rather than in
/// nothing.
fn first_difference(left: &str, right: &str) -> String {
    let mut lefts = left.split('\n');
    let mut rights = right.split('\n');
    let mut line = 1_usize;
    loop {
        let (l, r) = (lefts.next(), rights.next());
        if l == r {
            if l.is_none() {
                return "no line differs".to_owned();
            }
            line += 1;
            continue;
        }
        let end = "<end of text>";
        return format!(
            "line {line}\n    first pass:  {}\n    second pass: {}",
            l.unwrap_or(end),
            r.unwrap_or(end)
        );
    }
}

/// `node` and everything under it, as the shape the formatter may not change.
///
/// Every production's own spelling, its scalars and the name it wrote — and no
/// span, because every offset in a file moves the moment its layout does. The
/// name is rendered as the source text it points at rather than as its range,
/// which is what makes this a claim about the program rather than about the
/// tree's arithmetic.
fn shape(node: &Node, source: &str) -> String {
    let mut out = String::new();
    render(node, source, 0, &mut out);
    out
}

/// One node into `out`, then its children one level deeper.
fn render(node: &Node, source: &str, depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push_str("  ");
    }
    out.push_str(node.kind);
    for (label, field) in &node.fields {
        match field {
            Field::Word(word) => write!(out, " {label}={word}"),
            Field::Flag(flag) => write!(out, " {label}={flag}"),
        }
        .expect("a string never fails to write");
    }
    if let Some(span) = node.name {
        let text = &source[span.start as usize..span.end as usize];
        write!(out, " `{text}`").expect("a string never fails to write");
    }
    out.push('\n');
    for child in &node.children {
        render(child, source, depth + 1, out);
    }
}

/// `rule:tooling/fmt-is-idempotent` over the corpus: the formatter's own output
/// is a fixed point, whether or not its input was one.
#[test]
fn formatting_the_whole_corpus_twice_changes_nothing_the_second_time() {
    let mut moved = Vec::new();
    let mut converged = 0_usize;
    for path in corpus() {
        let name = path.display().to_string();
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Some(once) = formatted(&name, &text) else {
            continue;
        };
        match formatted(&name, &once) {
            // The output of a formatter that refuses what does not parse has to
            // parse, so this is a rule that wrote a file it cannot read.
            None => moved.push(format!("{name}: its own output is refused")),
            Some(twice) if twice != once => {
                moved.push(format!("{name}: {}", first_difference(&once, &twice)));
            }
            Some(_) => converged += 1,
        }
    }

    assert!(
        converged > 50,
        "only {converged} corpus file(s) reached a second pass — this test is measuring nothing"
    );
    assert!(
        moved.is_empty(),
        "{} corpus file(s) moved again on a second pass, so `nvs fmt` is not a fixed point \
         over them and `--check` cannot mean anything:\n{}",
        moved.len(),
        moved.join("\n")
    );
}

/// The goal's § *Standing decisions*, "never a semantic change": the file that
/// comes back is the same program, whatever its layout now is.
#[test]
fn every_formatted_corpus_file_parses_to_the_same_tree_as_its_input() {
    let mut changed = Vec::new();
    let mut compared = 0_usize;
    for path in corpus() {
        let name = path.display().to_string();
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        // A file the strict parse rejects is one of the deliberately broken
        // ones, and it has no tree on either side to compare.
        let Ok(before) = walk::of_source(&name, &text) else {
            continue;
        };
        let Some(output) = formatted(&name, &text) else {
            continue;
        };
        match walk::of_source(&name, &output) {
            Err(err) => changed.push(format!("{name}: the formatted file does not parse: {err}")),
            Ok(after) => {
                compared += 1;
                let (before, after) = (shape(&before, &text), shape(&after, &output));
                if before != after {
                    changed.push(format!("{name}: {}", first_difference(&before, &after)));
                }
            }
        }
    }

    assert!(
        compared > 50,
        "only {compared} corpus file(s) were parsed on both sides — this test is measuring nothing"
    );
    assert!(
        changed.is_empty(),
        "{} corpus file(s) parse to a different tree after formatting, which is the one thing \
         a formatter may never do:\n{}",
        changed.len(),
        changed.join("\n")
    );
}
